use ratatui::{
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::widgets::{centered, panel, pill};
use crate::theme::{Theme, THEMES};

fn section(title: &str, rows: &[(&str, &str)], t: &Theme) -> Vec<Line<'static>> {
    let mut lines =
        vec![Line::from(Span::styled(title.to_owned(), Style::new().fg(t.accent2).bold()))];
    for (key, description) in rows {
        lines.push(Line::from(vec![
            Span::styled(format!("{key:<9}"), Style::new().fg(t.accent).bold()),
            Span::styled((*description).to_owned(), Style::new().fg(t.fg)),
        ]));
    }
    lines
}

pub fn help(frame: &mut Frame, area: Rect, t: &Theme) {
    let rect = centered(area, 68, 16);
    frame.render_widget(Clear, rect);

    let hint = Line::from(Span::styled(" Esc to close ", Style::new().fg(t.dim))).right_aligned();
    let block = panel(t, "Keys", true).title_bottom(hint);
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    let inner = inner.inner(Margin { vertical: 1, horizontal: 2 });

    let mut left = section(
        "Navigate",
        &[("j / k", "move down / up"), ("g / G", "first / last"), ("PgUp/Dn", "jump by five")],
        t,
    );
    left.push(Line::raw(""));
    left.extend(section(
        "Device",
        &[
            ("Enter", "connect / disconnect"),
            ("p", "pair"),
            ("t", "toggle trust"),
            ("b", "toggle blocked"),
            ("x", "remove (forget)"),
            ("f", "cycle filter"),
        ],
        t,
    ));

    let mut right = section(
        "Adapter",
        &[("s", "start / stop scan"), ("o", "power on / off"), ("d", "toggle discoverable")],
        t,
    );
    right.push(Line::raw(""));
    right.extend(section("General", &[("T", "cycle theme"), ("?", "this help"), ("q", "quit")], t));
    right.push(Line::raw(""));
    right.push(Line::from(Span::styled(
        format!("Themes: {}", THEMES.map(|th| th.name).join(", ")),
        Style::new().fg(t.dim),
    )));

    let [l, r] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(inner);
    frame.render_widget(Paragraph::new(left), l);
    frame.render_widget(Paragraph::new(right), r);
}

pub fn confirm_remove(frame: &mut Frame, area: Rect, name: &str, t: &Theme) {
    let rect = centered(area, 52, 7);
    frame.render_widget(Clear, rect);
    let block = panel(t, "Remove device", true);
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    let lines = vec![
        Line::raw(""),
        Line::from(Span::styled(super::widgets::truncate(name, 44), Style::new().fg(t.fg).bold())),
        Line::from(Span::styled("will be forgotten and unpaired.", Style::new().fg(t.dim))),
        Line::raw(""),
        Line::from(vec![
            pill("y  remove", t.on_accent, t.bad),
            Span::raw("   "),
            pill("n  cancel", t.fg, t.sel),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}
