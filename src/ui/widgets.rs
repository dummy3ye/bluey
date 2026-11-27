//! Small reusable building blocks shared by the panels.

use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType},
};

use crate::theme::Theme;

const SPINNER: [&str; 10] = [
    "\u{280b}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283c}", "\u{2834}", "\u{2826}", "\u{2827}",
    "\u{2807}", "\u{280f}",
];

pub fn spinner(frame: u64) -> &'static str {
    SPINNER[(frame as usize) % SPINNER.len()]
}

/// A rounded, titled panel. The focused panel gets the accent border.
pub fn panel<'a>(t: &Theme, title: &'a str, focused: bool) -> Block<'a> {
    let border = if focused { t.accent } else { t.border };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(border))
        .style(Style::new().bg(t.bg).fg(t.fg))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(title, Style::new().fg(t.accent).bold()),
            Span::raw(" "),
        ]))
}

/// A filled label, e.g. ` power on `.
pub fn pill(text: &str, fg: Color, bg: Color) -> Span<'static> {
    Span::styled(format!(" {text} "), Style::new().fg(fg).bg(bg).bold())
}

/// Signal quality from 0 (unknown) to 4 (excellent).
pub fn signal_level(rssi: Option<i16>) -> u8 {
    match rssi {
        None => 0,
        Some(r) if r >= -55 => 4,
        Some(r) if r >= -67 => 3,
        Some(r) if r >= -80 => 2,
        Some(_) => 1,
    }
}

pub fn signal_label(level: u8) -> &'static str {
    match level {
        4 => "excellent",
        3 => "good",
        2 => "fair",
        1 => "weak",
        _ => "unknown",
    }
}

pub fn signal_bars(t: &Theme, rssi: Option<i16>) -> Vec<Span<'static>> {
    let level = signal_level(rssi);
    let color = match level {
        3 | 4 => t.good,
        2 => t.warn,
        _ => t.bad,
    };
    ["\u{2582}", "\u{2584}", "\u{2586}", "\u{2588}"]
        .iter()
        .enumerate()
        .map(|(i, glyph)| {
            let filled = (i as u8) < level;
            Span::styled(*glyph, Style::new().fg(if filled { color } else { t.border }))
        })
        .collect()
}

pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

pub fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
        out.push('\u{2026}');
        out
    }
}

pub fn mmss(elapsed: std::time::Duration) -> String {
    let s = elapsed.as_secs();
    format!("{:02}:{:02}", s / 60 % 100, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_levels() {
        assert_eq!(signal_level(None), 0);
        assert_eq!(signal_level(Some(-40)), 4);
        assert_eq!(signal_level(Some(-60)), 3);
        assert_eq!(signal_level(Some(-75)), 2);
        assert_eq!(signal_level(Some(-95)), 1);
    }

    #[test]
    fn truncation_adds_ellipsis() {
        assert_eq!(truncate("abcdef", 4), "abc\u{2026}");
        assert_eq!(truncate("abc", 4), "abc");
    }
}
