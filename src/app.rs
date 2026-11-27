//! Application state and key handling. No terminal I/O and no drawing here.

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    widgets::TableState,
};

use crate::{
    bluetooth::{AdapterInfo, Address, Command, DeviceInfo, Handle, Level, Snapshot, Update},
    config::Config,
    input::Input,
    theme::{Theme, THEMES},
};

const LOG_CAPACITY: usize = 200;
const TOAST_TICKS: u32 = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Help,
    ConfirmRemove { address: Address, name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Paired,
    Connected,
}

impl Filter {
    pub fn label(self) -> &'static str {
        match self {
            Filter::All => "all",
            Filter::Paired => "paired",
            Filter::Connected => "connected",
        }
    }

    fn next(self) -> Self {
        match self {
            Filter::All => Filter::Paired,
            Filter::Paired => Filter::Connected,
            Filter::Connected => Filter::All,
        }
    }

    fn accepts(self, device: &DeviceInfo) -> bool {
        match self {
            Filter::All => true,
            Filter::Paired => device.paired,
            Filter::Connected => device.connected,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub elapsed: Duration,
    pub level: Level,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub level: Level,
    pub text: String,
    ttl: u32,
}

pub struct App {
    pub mode: Mode,
    pub should_quit: bool,
    pub adapter: Option<AdapterInfo>,
    pub devices: Vec<DeviceInfo>,
    pub table: TableState,
    pub filter: Filter,
    pub log: VecDeque<LogEntry>,
    pub toast: Option<Toast>,
    pub frame: u64,
    pub pending: usize,
    selected: Option<Address>,
    theme: usize,
    transparent_background: bool,
    started: Instant,
    bt: Handle,
}

impl App {
    pub fn new(bt: Handle, config: Config) -> Self {
        let theme = Self::theme_index(&config.theme);
        let mut app = Self {
            mode: Mode::Normal,
            should_quit: false,
            adapter: None,
            devices: Vec::new(),
            table: TableState::default(),
            filter: Filter::All,
            log: VecDeque::new(),
            toast: None,
            frame: 0,
            pending: 0,
            selected: None,
            theme,
            transparent_background: config.transparent_background,
            started: Instant::now(),
            bt,
        };
        app.push_log(Level::Info, "bluey started, press ? for help");
        app
    }

    fn theme_index(name: &str) -> usize {
        THEMES.iter().position(|theme| theme.name.eq_ignore_ascii_case(name)).unwrap_or(0)
    }

    pub fn theme(&self) -> Theme {
        let theme = THEMES[self.theme];
        if self.transparent_background {
            theme.with_transparent_background()
        } else {
            theme
        }
    }

    /// Devices that pass the current filter, in display order.
    pub fn visible(&self) -> Vec<&DeviceInfo> {
        self.devices.iter().filter(|d| self.filter.accepts(d)).collect()
    }

    pub fn selected_device(&self) -> Option<&DeviceInfo> {
        let address = self.selected?;
        self.visible().into_iter().find(|d| d.address == address)
    }

    // -- events -------------------------------------------------------------

    pub fn on_tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        if let Some(toast) = &mut self.toast {
            toast.ttl = toast.ttl.saturating_sub(1);
            if toast.ttl == 0 {
                self.toast = None;
            }
        }
    }

    pub fn on_update(&mut self, update: Update) {
        match update {
            Update::Snapshot(snapshot) => self.apply_snapshot(snapshot),
            Update::Notice(level, text) => self.notify(level, text),
        }
    }

    pub fn on_input(&mut self, input: Input) {
        if let Input::Key(key) = input {
            if key.kind == KeyEventKind::Press {
                self.on_key(key);
            }
        }
    }

    fn apply_snapshot(&mut self, snapshot: Snapshot) {
        self.adapter = Some(snapshot.adapter);
        self.pending = snapshot.pending;
        self.devices = snapshot.devices;
        // Stable order: connected, paired, named, then alphabetical. Signal
        // strength is deliberately *not* a sort key so rows do not jitter.
        self.devices.sort_by_cached_key(|d| {
            (!d.connected, !d.paired, !d.has_name(), d.alias.to_lowercase(), d.address.to_string())
        });
        self.sync_selection();
    }

    fn notify(&mut self, level: Level, text: String) {
        self.push_log(level, text.clone());
        self.toast = Some(Toast { level, text, ttl: TOAST_TICKS });
    }

    fn push_log(&mut self, level: Level, text: impl Into<String>) {
        if self.log.len() == LOG_CAPACITY {
            self.log.pop_front();
        }
        self.log.push_back(LogEntry { elapsed: self.started.elapsed(), level, text: text.into() });
    }

    // -- selection ----------------------------------------------------------

    /// Re-anchor the table cursor to the selected address after the list changed.
    fn sync_selection(&mut self) {
        let old_index = self.table.selected().unwrap_or(0);
        let visible = self.visible();
        let index = match self.selected.and_then(|a| visible.iter().position(|d| d.address == a)) {
            Some(index) => Some(index),
            None if visible.is_empty() => None,
            None => Some(old_index.min(visible.len() - 1)),
        };
        let selected = index.map(|i| visible[i].address);
        self.selected = selected;
        self.table.select(index);
    }

    fn move_selection(&mut self, delta: isize) {
        let visible = self.visible();
        if visible.is_empty() {
            return;
        }
        let current = self.table.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, visible.len() as isize - 1) as usize;
        let selected = visible[next].address;
        self.selected = Some(selected);
        self.table.select(Some(next));
    }

    // -- keys ---------------------------------------------------------------

    fn on_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        match self.mode.clone() {
            Mode::Help => {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char('?')
                ) {
                    self.mode = Mode::Normal;
                }
            }
            Mode::ConfirmRemove { address, name } => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.bt.send(Command::Remove { address, name });
                    self.mode = Mode::Normal;
                }
                KeyCode::Char('n') | KeyCode::Esc | KeyCode::Char('q') => {
                    self.mode = Mode::Normal;
                }
                _ => {}
            },
            Mode::Normal => self.on_normal_key(key.code),
        }
    }

    fn on_normal_key(&mut self, code: KeyCode) {
        let target = self.selected_device().map(|d| (d.address, d.alias.clone()));
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('?') => self.mode = Mode::Help,

            KeyCode::Char('j') | KeyCode::Down => self.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection(-1),
            KeyCode::PageDown => self.move_selection(5),
            KeyCode::PageUp => self.move_selection(-5),
            KeyCode::Char('g') | KeyCode::Home => self.move_selection(isize::MIN / 2),
            KeyCode::Char('G') | KeyCode::End => self.move_selection(isize::MAX / 2),

            KeyCode::Enter | KeyCode::Char('c') => {
                if let Some((address, name)) = target {
                    self.bt.send(Command::ToggleConnection { address, name });
                }
            }
            KeyCode::Char('p') => {
                if let Some((address, name)) = target {
                    self.bt.send(Command::Pair { address, name });
                }
            }
            KeyCode::Char('t') => {
                if let Some((address, name)) = target {
                    self.bt.send(Command::ToggleTrust { address, name });
                }
            }
            KeyCode::Char('b') => {
                if let Some((address, name)) = target {
                    self.bt.send(Command::ToggleBlocked { address, name });
                }
            }
            KeyCode::Char('x') => {
                if let Some((address, name)) = target {
                    self.mode = Mode::ConfirmRemove { address, name };
                }
            }

            KeyCode::Char('s') => self.bt.send(Command::ToggleScan),
            KeyCode::Char('o') => self.bt.send(Command::TogglePower),
            KeyCode::Char('d') => self.bt.send(Command::ToggleDiscoverable),

            KeyCode::Char('f') => {
                self.filter = self.filter.next();
                self.sync_selection();
            }
            KeyCode::Char('T') => {
                self.theme = (self.theme + 1) % THEMES.len();
                let name = self.theme().name;
                self.notify(Level::Info, format!("Theme: {name}"));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bluetooth::AdapterInfo;

    fn adapter() -> AdapterInfo {
        AdapterInfo {
            name: "hci0".into(),
            alias: "host".into(),
            address: Address::new([0, 1, 2, 3, 4, 5]),
            powered: true,
            discovering: true,
            discoverable: false,
            pairable: true,
        }
    }

    fn device(last: u8, alias: &str, paired: bool, connected: bool) -> DeviceInfo {
        DeviceInfo {
            address: Address::new([0, 0, 0, 0, 0, last]),
            alias: alias.into(),
            icon: None,
            class: None,
            paired,
            connected,
            trusted: false,
            blocked: false,
            rssi: Some(-60),
            service_count: 0,
        }
    }

    fn snapshot(devices: Vec<DeviceInfo>) -> Update {
        Update::Snapshot(Snapshot { adapter: adapter(), devices, pending: 0 })
    }

    #[test]
    fn devices_are_sorted_connected_then_paired_then_name() {
        let mut app = App::new(Handle::detached(), crate::config::Config::default());
        app.on_update(snapshot(vec![
            device(1, "Zed", false, false),
            device(2, "Beta", true, false),
            device(3, "Alpha", true, true),
        ]));
        let names: Vec<_> = app.devices.iter().map(|d| d.alias.as_str()).collect();
        assert_eq!(names, ["Alpha", "Beta", "Zed"]);
    }

    #[test]
    fn selection_follows_the_device_when_the_list_changes() {
        let mut app = App::new(Handle::detached(), crate::config::Config::default());
        app.on_update(snapshot(vec![
            device(1, "Alpha", false, false),
            device(2, "Beta", false, false),
        ]));
        app.move_selection(1);
        assert_eq!(app.selected_device().unwrap().alias, "Beta");

        app.on_update(snapshot(vec![
            device(0, "Aardvark", false, false),
            device(1, "Alpha", false, false),
            device(2, "Beta", false, false),
        ]));
        assert_eq!(app.selected_device().unwrap().alias, "Beta");
        assert_eq!(app.table.selected(), Some(2));
    }

    #[test]
    fn filter_limits_visible_devices() {
        let mut app = App::new(Handle::detached(), crate::config::Config::default());
        app.on_update(snapshot(vec![
            device(1, "Alpha", true, false),
            device(2, "Beta", false, false),
        ]));
        app.filter = Filter::Paired;
        assert_eq!(app.visible().len(), 1);
    }
}
