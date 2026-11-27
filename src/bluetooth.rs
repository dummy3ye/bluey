//! BlueZ backend.
//!
//! The UI talks to Bluetooth exclusively through a [`Handle`]: it sends
//! [`Command`]s in and receives [`Update`]s out. A single background task owns
//! the adapter, polls it for fresh [`Snapshot`]s and runs slow operations
//! (connect, pair, ...) as detached jobs so the interface never blocks.

use std::{
    future::Future,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{Context, Result};
pub use bluer::Address;
use bluer::{
    agent::{
        Agent, AgentHandle, AuthorizeService, ReqResult, RequestAuthorization, RequestConfirmation,
    },
    Adapter, Session,
};
use futures::StreamExt;
use tokio::{sync::mpsc, task::JoinHandle};

const POLL_INTERVAL: Duration = Duration::from_millis(900);

// ---------------------------------------------------------------------------
// Public model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Success,
    Warn,
    Error,
}

impl Level {
    pub fn glyph(self) -> &'static str {
        match self {
            Level::Info => "\u{203a}",
            Level::Success => "\u{2713}",
            Level::Warn => "!",
            Level::Error => "\u{2717}",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdapterInfo {
    pub name: String,
    pub alias: String,
    pub address: Address,
    pub powered: bool,
    pub discovering: bool,
    pub discoverable: bool,
    pub pairable: bool,
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub address: Address,
    pub alias: String,
    pub icon: Option<String>,
    pub class: Option<u32>,
    pub paired: bool,
    pub connected: bool,
    pub trusted: bool,
    pub blocked: bool,
    pub rssi: Option<i16>,
    pub service_count: usize,
}

impl DeviceInfo {
    /// `true` when the device advertises a real name (BlueZ falls back to the
    /// address with dashes when it does not).
    pub fn has_name(&self) -> bool {
        self.alias.replace('-', ":") != self.address.to_string()
    }

    /// Human-readable device type derived from the BlueZ icon name.
    pub fn kind(&self) -> &'static str {
        let icon = self.icon.as_deref().unwrap_or("");
        let has = |needle: &str| icon.contains(needle);
        if has("headphones") {
            "Headphones"
        } else if has("headset") {
            "Headset"
        } else if has("speaker") || has("audio-card") {
            "Speaker"
        } else if has("keyboard") {
            "Keyboard"
        } else if has("mouse") {
            "Mouse"
        } else if has("gaming") {
            "Gamepad"
        } else if has("tablet") {
            "Tablet"
        } else if has("phone") {
            "Phone"
        } else if has("computer") {
            "Computer"
        } else if has("display") {
            "Display"
        } else if has("camera") {
            "Camera"
        } else if has("printer") {
            "Printer"
        } else if has("network") || has("modem") {
            "Network"
        } else if icon.is_empty() {
            "Unknown"
        } else {
            "Device"
        }
    }

    /// Major device class decoded from the Class of Device field.
    pub fn major_class(&self) -> Option<&'static str> {
        let class = self.class?;
        Some(match (class >> 8) & 0x1f {
            1 => "Computer",
            2 => "Phone",
            3 => "Network",
            4 => "Audio/Video",
            5 => "Peripheral",
            6 => "Imaging",
            7 => "Wearable",
            8 => "Toy",
            9 => "Health",
            _ => "Miscellaneous",
        })
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub adapter: AdapterInfo,
    pub devices: Vec<DeviceInfo>,
    /// Number of background jobs (connect, pair, ...) currently running.
    pub pending: usize,
}

#[derive(Debug, Clone)]
pub enum Update {
    Snapshot(Snapshot),
    Notice(Level, String),
}

#[derive(Debug, Clone)]
pub enum Command {
    ToggleScan,
    TogglePower,
    ToggleDiscoverable,
    ToggleConnection { address: Address, name: String },
    Pair { address: Address, name: String },
    ToggleTrust { address: Address, name: String },
    ToggleBlocked { address: Address, name: String },
    Remove { address: Address, name: String },
}

/// Cheap, cloneable sender used by the UI to issue commands.
#[derive(Debug, Clone)]
pub struct Handle {
    tx: mpsc::UnboundedSender<Command>,
}

impl Handle {
    pub fn send(&self, command: Command) {
        let _ = self.tx.send(command);
    }

    /// A handle that goes nowhere; handy for tests.
    #[cfg(test)]
    pub fn detached() -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();
        Self { tx }
    }
}

/// Connect to BlueZ, start the backend task and return its channels.
pub async fn spawn() -> Result<(Handle, mpsc::UnboundedReceiver<Update>)> {
    let session = Session::new()
        .await
        .context("could not reach the BlueZ daemon (is bluetooth.service running?)")?;
    let adapter = session
        .default_adapter()
        .await
        .context("no Bluetooth adapter found (check `rfkill list` and your hardware)")?;

    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (update_tx, update_rx) = mpsc::unbounded_channel();

    let backend = Backend {
        _session: session,
        adapter,
        events: update_tx,
        scan: None,
        pending: Arc::new(AtomicUsize::new(0)),
        _agent: None,
        last_error: None,
    };
    tokio::spawn(backend.run(command_rx));

    Ok((Handle { tx: command_tx }, update_rx))
}

// ---------------------------------------------------------------------------
// Backend task
// ---------------------------------------------------------------------------

struct Backend {
    /// Kept alive for the lifetime of the D-Bus connection.
    _session: Session,
    adapter: Adapter,
    events: mpsc::UnboundedSender<Update>,
    scan: Option<JoinHandle<()>>,
    pending: Arc<AtomicUsize>,
    _agent: Option<AgentHandle>,
    last_error: Option<String>,
}

impl Backend {
    async fn run(mut self, mut commands: mpsc::UnboundedReceiver<Command>) {
        self.register_agent().await;
        let _ = self.adapter.set_pairable(true).await;
        self.set_scan(true).await;
        self.publish().await;

        let mut poll = tokio::time::interval(POLL_INTERVAL);
        loop {
            tokio::select! {
                command = commands.recv() => match command {
                    Some(command) => {
                        self.handle(command).await;
                        self.publish().await;
                    }
                    None => break,
                },
                _ = poll.tick() => self.publish().await,
            }
        }
        if let Some(scan) = self.scan.take() {
            scan.abort();
        }
    }

    fn notice(&self, level: Level, message: impl Into<String>) {
        let _ = self.events.send(Update::Notice(level, message.into()));
    }

    async fn register_agent(&mut self) {
        let agent = Agent {
            request_default: true,
            request_confirmation: Some(Box::new(|req| Box::pin(accept_confirmation(req)))),
            request_authorization: Some(Box::new(|req| Box::pin(accept_authorization(req)))),
            authorize_service: Some(Box::new(|req| Box::pin(accept_service(req)))),
            ..Default::default()
        };
        match self._session.register_agent(agent).await {
            Ok(handle) => self._agent = Some(handle),
            Err(err) => self.notice(Level::Warn, format!("Pairing agent unavailable: {err}")),
        }
    }

    fn scanning(&self) -> bool {
        self.scan.as_ref().is_some_and(|task| !task.is_finished())
    }

    async fn set_scan(&mut self, on: bool) {
        if on {
            if self.scanning() {
                return;
            }
            if !matches!(self.adapter.is_powered().await, Ok(true)) {
                self.notice(Level::Warn, "Adapter is powered off, press o to turn it on");
                return;
            }
            let adapter = self.adapter.clone();
            let events = self.events.clone();
            self.scan = Some(tokio::spawn(async move {
                match adapter.discover_devices().await {
                    Ok(stream) => {
                        futures::pin_mut!(stream);
                        while stream.next().await.is_some() {}
                    }
                    Err(err) => {
                        let _ = events
                            .send(Update::Notice(Level::Error, format!("Scan failed: {err}")));
                    }
                }
            }));
            self.notice(Level::Info, "Scanning for devices");
        } else if let Some(task) = self.scan.take() {
            task.abort();
            self.notice(Level::Info, "Scan stopped");
        }
    }

    async fn handle(&mut self, command: Command) {
        match command {
            Command::ToggleScan => {
                let active = self.scanning();
                self.set_scan(!active).await;
            }
            Command::TogglePower => self.toggle_power().await,
            Command::ToggleDiscoverable => {
                let current = self.adapter.is_discoverable().await.unwrap_or(false);
                match self.adapter.set_discoverable(!current).await {
                    Ok(()) if current => self.notice(Level::Info, "Adapter hidden"),
                    Ok(()) => self.notice(Level::Info, "Adapter is now discoverable"),
                    Err(err) => {
                        self.notice(Level::Error, format!("Could not change visibility: {err}"))
                    }
                }
            }
            Command::ToggleConnection { address, name } => {
                self.notice(Level::Info, format!("Working on {name}\u{2026}"));
                let adapter = self.adapter.clone();
                self.spawn_job(name.clone(), toggle_connection(adapter, address, name));
            }
            Command::Pair { address, name } => {
                self.notice(Level::Info, format!("Pairing with {name}\u{2026}"));
                let adapter = self.adapter.clone();
                self.spawn_job(name.clone(), pair(adapter, address, name));
            }
            Command::ToggleTrust { address, name } => {
                let adapter = self.adapter.clone();
                self.spawn_job(name.clone(), toggle_trust(adapter, address, name));
            }
            Command::ToggleBlocked { address, name } => {
                let adapter = self.adapter.clone();
                self.spawn_job(name.clone(), toggle_blocked(adapter, address, name));
            }
            Command::Remove { address, name } => {
                let adapter = self.adapter.clone();
                self.spawn_job(name.clone(), remove(adapter, address, name));
            }
        }
    }

    async fn toggle_power(&mut self) {
        let on = self.adapter.is_powered().await.unwrap_or(false);
        if on {
            self.set_scan(false).await;
        }
        match self.adapter.set_powered(!on).await {
            Ok(()) if on => self.notice(Level::Success, "Adapter powered off"),
            Ok(()) => {
                self.notice(Level::Success, "Adapter powered on");
                self.set_scan(true).await;
            }
            Err(err) => self.notice(Level::Error, format!("Could not change power state: {err}")),
        }
    }

    /// Run a slow operation in the background and report its outcome.
    fn spawn_job<Fut>(&self, name: String, job: Fut)
    where
        Fut: Future<Output = bluer::Result<String>> + Send + 'static,
    {
        let events = self.events.clone();
        let pending = Arc::clone(&self.pending);
        pending.fetch_add(1, Ordering::SeqCst);
        tokio::spawn(async move {
            let outcome = match job.await {
                Ok(message) => Update::Notice(Level::Success, message),
                Err(err) => Update::Notice(Level::Error, format!("{name}: {err}")),
            };
            pending.fetch_sub(1, Ordering::SeqCst);
            let _ = events.send(outcome);
        });
    }

    async fn publish(&mut self) {
        match self.snapshot().await {
            Ok(snapshot) => {
                self.last_error = None;
                let _ = self.events.send(Update::Snapshot(snapshot));
            }
            Err(err) => {
                let message = format!("{err:#}");
                if self.last_error.as_deref() != Some(message.as_str()) {
                    self.notice(Level::Error, message.clone());
                    self.last_error = Some(message);
                }
            }
        }
    }

    async fn snapshot(&self) -> Result<Snapshot> {
        let a = &self.adapter;
        let adapter = AdapterInfo {
            name: a.name().to_string(),
            alias: a.alias().await?,
            address: a.address().await?,
            powered: a.is_powered().await?,
            discovering: a.is_discovering().await?,
            discoverable: a.is_discoverable().await?,
            pairable: a.is_pairable().await?,
        };

        let mut devices = Vec::new();
        for address in a.device_addresses().await? {
            // A device may vanish between listing and reading; just skip it.
            if let Ok(info) = read_device(a, address).await {
                devices.push(info);
            }
        }

        Ok(Snapshot { adapter, devices, pending: self.pending.load(Ordering::SeqCst) })
    }
}

async fn read_device(adapter: &Adapter, address: Address) -> bluer::Result<DeviceInfo> {
    let device = adapter.device(address)?;
    Ok(DeviceInfo {
        address,
        alias: device.alias().await?,
        icon: device.icon().await?,
        class: device.class().await?,
        paired: device.is_paired().await?,
        connected: device.is_connected().await?,
        trusted: device.is_trusted().await?,
        blocked: device.is_blocked().await?,
        rssi: device.rssi().await?,
        service_count: device.uuids().await?.map_or(0, |uuids| uuids.len()),
    })
}

// ---------------------------------------------------------------------------
// Device jobs
// ---------------------------------------------------------------------------

async fn toggle_connection(
    adapter: Adapter,
    address: Address,
    name: String,
) -> bluer::Result<String> {
    let device = adapter.device(address)?;
    if device.is_connected().await? {
        device.disconnect().await?;
        Ok(format!("Disconnected from {name}"))
    } else {
        device.connect().await?;
        Ok(format!("Connected to {name}"))
    }
}

async fn pair(adapter: Adapter, address: Address, name: String) -> bluer::Result<String> {
    let device = adapter.device(address)?;
    if device.is_paired().await? {
        return Ok(format!("{name} is already paired"));
    }
    device.pair().await?;
    device.set_trusted(true).await?;
    Ok(format!("Paired with {name}"))
}

async fn toggle_trust(adapter: Adapter, address: Address, name: String) -> bluer::Result<String> {
    let device = adapter.device(address)?;
    let trusted = device.is_trusted().await?;
    device.set_trusted(!trusted).await?;
    Ok(if trusted { format!("{name} is no longer trusted") } else { format!("Trusting {name}") })
}

async fn toggle_blocked(adapter: Adapter, address: Address, name: String) -> bluer::Result<String> {
    let device = adapter.device(address)?;
    let blocked = device.is_blocked().await?;
    device.set_blocked(!blocked).await?;
    Ok(if blocked { format!("{name} unblocked") } else { format!("{name} blocked") })
}

async fn remove(adapter: Adapter, address: Address, name: String) -> bluer::Result<String> {
    adapter.remove_device(address).await?;
    Ok(format!("Removed {name}"))
}

// ---------------------------------------------------------------------------
// Pairing agent: accept "just works" and numeric-comparison requests.
// ---------------------------------------------------------------------------

async fn accept_confirmation(_: RequestConfirmation) -> ReqResult<()> {
    Ok(())
}

async fn accept_authorization(_: RequestAuthorization) -> ReqResult<()> {
    Ok(())
}

async fn accept_service(_: AuthorizeService) -> ReqResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(icon: Option<&str>, class: Option<u32>) -> DeviceInfo {
        DeviceInfo {
            address: Address::new([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]),
            alias: "AA-BB-CC-DD-EE-FF".into(),
            icon: icon.map(str::to_owned),
            class,
            paired: false,
            connected: false,
            trusted: false,
            blocked: false,
            rssi: None,
            service_count: 0,
        }
    }

    #[test]
    fn kind_is_derived_from_icon() {
        assert_eq!(device(Some("audio-headphones"), None).kind(), "Headphones");
        assert_eq!(device(Some("input-keyboard"), None).kind(), "Keyboard");
        assert_eq!(device(Some("something-odd"), None).kind(), "Device");
        assert_eq!(device(None, None).kind(), "Unknown");
    }

    #[test]
    fn major_class_is_decoded() {
        assert_eq!(device(None, Some(0x240404)).major_class(), Some("Audio/Video"));
        assert_eq!(device(None, None).major_class(), None);
    }

    #[test]
    fn unnamed_devices_are_detected() {
        let mut d = device(None, None);
        assert!(!d.has_name());
        d.alias = "Keyboard".into();
        assert!(d.has_name());
    }
}
