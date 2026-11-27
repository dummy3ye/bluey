//! Right-hand column: device details, adapter status and the activity log.

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::widgets::{mmss, panel, pill, signal_bars, signal_label, signal_level};
use crate::{app::App, theme::Theme};

const LABEL_WIDTH: usize = 12;

pub fn render(frame: &mut Frame, area: Rect, app: &App, t: &Theme) {
    if area.height >= 24 {
        let [details, adapter, log] =
            Layout::vertical([Constraint::Length(11), Constraint::Length(7), Constraint::Min(3)])
                .areas(area);
        self::details(frame, details, app, t);
        self::adapter(frame, adapter, app, t);
        self::log(frame, log, app, t);
    } else {
        let [details, adapter] =
            Layout::vertical([Constraint::Min(9), Constraint::Length(7)]).areas(area);
        self::details(frame, details, app, t);
        self::adapter(frame, adapter, app, t);
    }
}

fn label(text: &str, t: &Theme) -> Span<'static> {
    Span::styled(format!("{text:<LABEL_WIDTH$}"), Style::new().fg(t.dim))
}

fn flag(on: bool, t: &Theme) -> Span<'static> {
    if on {
        Span::styled("\u{25cf} on", Style::new().fg(t.good))
    } else {
        Span::styled("\u{25cb} off", Style::new().fg(t.dim))
    }
}

fn details(frame: &mut Frame, area: Rect, app: &App, t: &Theme) {
    let block = panel(t, "Details", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(d) = app.selected_device() else {
        let rect = Rect { y: inner.y + inner.height / 2, height: 1.min(inner.height), ..inner };
        frame.render_widget(
            Paragraph::new(Span::styled("no device selected", Style::new().fg(t.dim)))
                .alignment(Alignment::Center),
            rect,
        );
        return;
    };

    let level = signal_level(d.rssi);
    let mut signal = vec![label("Signal", t)];
    signal.extend(signal_bars(t, d.rssi));
    signal.push(Span::styled(
        match d.rssi {
            Some(rssi) => format!("  {rssi} dBm  {}", signal_label(level)),
            None => "  out of range".to_owned(),
        },
        Style::new().fg(t.fg),
    ));

    let mut status = vec![label("Status", t)];
    status.push(if d.connected {
        pill("connected", t.on_accent, t.good)
    } else {
        pill("offline", t.dim, t.sel)
    });
    if d.paired {
        status.push(Span::raw(" "));
        status.push(pill("paired", t.on_accent, t.accent));
    }
    if d.trusted {
        status.push(Span::raw(" "));
        status.push(pill("trusted", t.on_accent, t.accent2));
    }
    if d.blocked {
        status.push(Span::raw(" "));
        status.push(pill("blocked", t.on_accent, t.bad));
    }

    let class = match (d.class, d.major_class()) {
        (Some(raw), Some(major)) => format!("0x{raw:06X}  {major}"),
        _ => "n/a".to_owned(),
    };

    let lines = vec![
        Line::from(Span::styled(d.alias.clone(), Style::new().fg(t.accent).bold())),
        Line::from(Span::styled(d.kind(), Style::new().fg(t.dim))),
        Line::raw(""),
        Line::from(vec![
            label("Address", t),
            Span::styled(d.address.to_string(), Style::new().fg(t.fg)),
        ]),
        Line::from(signal),
        Line::from(status),
        Line::from(vec![
            label("Services", t),
            Span::styled(d.service_count.to_string(), Style::new().fg(t.fg)),
        ]),
        Line::from(vec![label("Class", t), Span::styled(class, Style::new().fg(t.fg))]),
    ];
    let padded = Rect { x: inner.x + 1, width: inner.width.saturating_sub(2), ..inner };
    frame.render_widget(Paragraph::new(lines), padded);
}

fn adapter(frame: &mut Frame, area: Rect, app: &App, t: &Theme) {
    let block = panel(t, "Adapter", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = match &app.adapter {
        Some(a) => vec![
            Line::from(vec![
                label("Name", t),
                Span::styled(format!("{}  ({})", a.alias, a.name), Style::new().fg(t.fg)),
            ]),
            Line::from(vec![
                label("Address", t),
                Span::styled(a.address.to_string(), Style::new().fg(t.fg)),
            ]),
            Line::from(vec![
                label("Power", t),
                flag(a.powered, t),
                Span::raw("    "),
                label("Scanning", t),
                flag(a.discovering, t),
            ]),
            Line::from(vec![label("Discoverable", t), flag(a.discoverable, t)]),
            Line::from(vec![label("Pairable", t), flag(a.pairable, t)]),
        ],
        None => vec![Line::from(Span::styled("connecting\u{2026}", Style::new().fg(t.dim)))],
    };
    let padded = Rect { x: inner.x + 1, width: inner.width.saturating_sub(2), ..inner };
    frame.render_widget(Paragraph::new(lines), padded);
}

fn log(frame: &mut Frame, area: Rect, app: &App, t: &Theme) {
    let block = panel(t, "Activity", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let capacity = inner.height as usize;
    let skip = app.log.len().saturating_sub(capacity);
    let width = inner.width.saturating_sub(10) as usize;
    let lines: Vec<Line> = app
        .log
        .iter()
        .skip(skip)
        .map(|entry| {
            Line::from(vec![
                Span::styled(format!(" {} ", mmss(entry.elapsed)), Style::new().fg(t.dim)),
                Span::styled(
                    format!("{} ", entry.level.glyph()),
                    Style::new().fg(t.level(entry.level)).bold(),
                ),
                Span::styled(super::widgets::truncate(&entry.text, width), Style::new().fg(t.fg)),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}
