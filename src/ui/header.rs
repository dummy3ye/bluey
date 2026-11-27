use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph},
    Frame,
};

use super::widgets::{pill, spinner};
use crate::{app::App, theme::Theme};

pub fn render(frame: &mut Frame, area: Rect, app: &App, t: &Theme) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(t.border))
        .style(Style::new().bg(t.bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let left = Line::from(vec![Span::styled(
        format!(" bluey - v{}", env!("CARGO_PKG_VERSION")),
        Style::new().fg(t.accent).bold(),
    )]);

    // Right: adapter, power, scan state.
    let spin = spinner(app.frame);
    let mut right: Vec<Span> = Vec::new();
    if let Some(adapter) = &app.adapter {
        right.push(pill(&adapter.name, t.fg, t.sel));
        right.push(Span::raw(" "));
        right.push(if adapter.powered {
            pill("power on", t.on_accent, t.good)
        } else {
            pill("power off", t.on_accent, t.bad)
        });
        right.push(Span::raw(" "));
        right.push(if adapter.discovering {
            pill(&format!("scanning {spin}"), t.on_accent, t.accent)
        } else {
            pill("idle", t.dim, t.sel)
        });
    } else {
        right.push(pill(&format!("connecting {spin}"), t.on_accent, t.warn));
    }
    if app.pending > 0 {
        right.push(Span::raw(" "));
        right.push(pill(&format!("working {spin}"), t.on_accent, t.warn));
    }
    right.push(Span::raw(" "));
    let right = Line::from(right);

    let right_width = (right.width() as u16).min(inner.width.saturating_sub(20));
    let [left_area, right_area] =
        Layout::horizontal([Constraint::Min(10), Constraint::Length(right_width)]).areas(inner);
    frame.render_widget(Paragraph::new(left), left_area);
    frame.render_widget(Paragraph::new(right), right_area);
}
