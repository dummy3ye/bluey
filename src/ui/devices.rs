use ratatui::{
    layout::{Alignment, Constraint, Margin, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table,
    },
    Frame,
};

use super::widgets::{panel, signal_bars, spinner};
use crate::{app::App, theme::Theme};

/// Below this inner width the ADDRESS column is dropped.
const ADDRESS_MIN_WIDTH: u16 = 64;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App, t: &Theme) {
    let show_address = area.width.saturating_sub(2) >= ADDRESS_MIN_WIDTH;

    let visible = app.visible();
    let shown = visible.len();
    let connected = app.devices.iter().filter(|d| d.connected).count();
    let paired = app.devices.iter().filter(|d| d.paired).count();

    let rows: Vec<Row> = visible
        .iter()
        .map(|d| {
            let (glyph, color) = if d.connected {
                ("\u{25cf}", t.good)
            } else if d.paired {
                ("\u{25c6}", t.accent)
            } else {
                ("\u{25cb}", t.dim)
            };
            let name_style = if d.has_name() {
                Style::new().fg(t.fg)
            } else {
                Style::new().fg(t.dim).add_modifier(Modifier::ITALIC)
            };
            let (state, state_color) = if d.connected {
                ("connected", t.good)
            } else if d.paired {
                ("paired", t.accent)
            } else {
                ("", t.dim)
            };

            let mut cells = vec![
                Cell::from(Span::styled(glyph, Style::new().fg(color))),
                Cell::from(Span::styled(d.alias.clone(), name_style)),
                Cell::from(Span::styled(d.kind(), Style::new().fg(t.dim))),
            ];
            if show_address {
                cells.push(Cell::from(Span::styled(d.address.to_string(), Style::new().fg(t.dim))));
            }
            cells.push(Cell::from(Line::from(signal_bars(t, d.rssi))));
            cells.push(Cell::from(Span::styled(state, Style::new().fg(state_color))));
            Row::new(cells)
        })
        .collect();

    let mut widths = vec![Constraint::Length(1), Constraint::Min(12), Constraint::Length(10)];
    let mut header = vec!["", "Name", "Type"];
    if show_address {
        widths.push(Constraint::Length(17));
        header.push("Address");
    }
    widths.push(Constraint::Length(4));
    header.push("Sig");
    widths.push(Constraint::Length(9));
    header.push("State");

    let footer = Line::from(vec![
        Span::styled(format!(" {shown} shown"), Style::new().fg(t.dim)),
        Span::styled(" \u{b7} ", Style::new().fg(t.border)),
        Span::styled(format!("{connected} connected"), Style::new().fg(t.good)),
        Span::styled(" \u{b7} ", Style::new().fg(t.border)),
        Span::styled(format!("{paired} paired"), Style::new().fg(t.accent)),
        Span::styled(" \u{b7} ", Style::new().fg(t.border)),
        Span::styled(format!("filter: {} ", app.filter.label()), Style::new().fg(t.dim)),
    ])
    .right_aligned();

    let block = panel(t, "Devices", true).title_bottom(footer);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if shown == 0 {
        empty_state(frame, inner, app, t);
        return;
    }

    let highlight = if t.sel == ratatui::style::Color::Reset {
        Style::new().fg(t.accent).add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::new().bg(t.sel).add_modifier(Modifier::BOLD)
    };

    let table = Table::new(rows, widths)
        .header(Row::new(header).style(Style::new().fg(t.dim).bold()))
        .column_spacing(2)
        .row_highlight_style(highlight)
        .highlight_symbol(Text::from(Line::styled("\u{258c} ", Style::new().fg(t.accent))))
        .highlight_spacing(HighlightSpacing::Always);
    frame.render_stateful_widget(table, inner, &mut app.table);

    // Scrollbar, only when the list overflows (one row is the header).
    if shown as u16 > inner.height.saturating_sub(1) {
        let mut state = ScrollbarState::new(shown).position(app.table.selected().unwrap_or(0));
        let bar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(None)
            .thumb_symbol("\u{2503}")
            .thumb_style(Style::new().fg(t.accent2));
        frame.render_stateful_widget(
            bar,
            area.inner(Margin { vertical: 1, horizontal: 0 }),
            &mut state,
        );
    }
}

fn empty_state(frame: &mut Frame, inner: Rect, app: &App, t: &Theme) {
    let scanning = app.adapter.as_ref().is_some_and(|a| a.discovering);
    let powered = app.adapter.as_ref().is_some_and(|a| a.powered);

    let (headline, hint) = if !powered {
        ("Adapter is powered off".to_owned(), "press o to turn it on".to_owned())
    } else if scanning {
        (format!("Scanning {}", spinner(app.frame)), "devices will appear here".to_owned())
    } else {
        ("No devices".to_owned(), "press s to start scanning".to_owned())
    };

    let text = vec![
        Line::from(Span::styled("\u{16d2}", Style::new().fg(t.accent).bold())),
        Line::raw(""),
        Line::from(Span::styled(headline, Style::new().fg(t.fg).bold())),
        Line::from(Span::styled(hint, Style::new().fg(t.dim))),
    ];
    let height = 4.min(inner.height);
    let rect = Rect { y: inner.y + inner.height.saturating_sub(height) / 2, height, ..inner };
    frame.render_widget(Paragraph::new(text).alignment(Alignment::Center), rect);
}
