//! Rendering. Each panel lives in its own module; this file only composes them.

mod devices;
mod footer;
mod header;
mod popup;
mod sidebar;
mod widgets;

use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::{
    app::{App, Mode},
    theme::Theme,
};

const MIN_WIDTH: u16 = 72;
const MIN_HEIGHT: u16 = 20;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let t = app.theme();
    let area = frame.area();
    if t.bg != ratatui::style::Color::Reset {
        frame.render_widget(Block::new().style(Style::new().bg(t.bg).fg(t.fg)), area);
    }

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        too_small(frame, area, &t);
        return;
    }

    let [header_area, body_area, footer_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(8), Constraint::Length(1)])
            .areas(area);
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
            .areas(body_area);

    header::render(frame, header_area, app, &t);
    devices::render(frame, left, app, &t);
    sidebar::render(frame, right, app, &t);
    footer::render(frame, footer_area, app, &t);

    match &app.mode {
        Mode::Help => popup::help(frame, area, &t),
        Mode::ConfirmRemove { name, .. } => popup::confirm_remove(frame, area, name, &t),
        Mode::Normal => {}
    }
}

fn too_small(frame: &mut Frame, area: Rect, t: &Theme) {
    let text = vec![
        Line::from(Span::styled("bluey", Style::new().fg(t.accent))),
        Line::from(Span::styled(
            format!("needs at least {MIN_WIDTH}x{MIN_HEIGHT} (now {}x{})", area.width, area.height),
            Style::new().fg(t.dim),
        )),
    ];
    let y = area.y + area.height.saturating_sub(2) / 2;
    let rect = Rect { y, height: 2.min(area.height), ..area };
    frame.render_widget(Paragraph::new(text).alignment(Alignment::Center), rect);
}
