use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::widgets::truncate;
use crate::{app::App, theme::Theme};

const HINTS: [(&str, &str); 11] = [
    ("j/k", "move"),
    ("Enter", "connect"),
    ("p", "pair"),
    ("t", "trust"),
    ("b", "block"),
    ("x", "remove"),
    ("s", "scan"),
    ("o", "power"),
    ("f", "filter"),
    ("?", "help"),
    ("q", "quit"),
];

pub fn render(frame: &mut Frame, area: Rect, app: &App, t: &Theme) {
    let line = match &app.toast {
        Some(toast) => Line::from(vec![
            Span::styled(
                format!(" {} ", toast.level.glyph()),
                Style::new().fg(t.on_accent).bg(t.level(toast.level)).bold(),
            ),
            Span::styled(
                format!(" {}", truncate(&toast.text, area.width.saturating_sub(5) as usize)),
                Style::new().fg(t.fg),
            ),
        ]),
        None => hints(area.width as usize, t),
    };
    frame.render_widget(Paragraph::new(line).style(Style::new().bg(t.bg)), area);
}

fn hints(max_width: usize, t: &Theme) -> Line<'static> {
    let mut spans = Vec::new();
    let mut used = 0;
    for (key, label) in HINTS {
        let width = key.chars().count() + label.chars().count() + 4;
        if used + width > max_width {
            break;
        }
        used += width;
        spans.push(Span::styled(format!(" {key} "), Style::new().fg(t.accent).bold()));
        spans.push(Span::styled(format!(" {label} "), Style::new().fg(t.dim)));
    }
    Line::from(spans)
}
