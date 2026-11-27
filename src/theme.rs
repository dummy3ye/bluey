//! Colour palettes. Every colour used by the UI comes from here.

use ratatui::style::Color;

use crate::bluetooth::Level;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub on_accent: Color,
    pub fg: Color,
    pub dim: Color,
    pub accent: Color,
    pub accent2: Color,
    pub good: Color,
    pub warn: Color,
    pub bad: Color,
    pub border: Color,
    pub sel: Color,
}

impl Theme {
    pub fn level(&self, level: Level) -> Color {
        match level {
            Level::Info => self.accent,
            Level::Success => self.good,
            Level::Warn => self.warn,
            Level::Error => self.bad,
        }
    }

    pub fn with_transparent_background(mut self) -> Self {
        self.bg = Color::Reset;
        self.sel = Color::Reset;
        self.on_accent = Color::Reset;
        self
    }
}

pub static THEMES: [Theme; 3] = [
    Theme {
        name: "Midnight",
        bg: Color::Rgb(15, 17, 26),
        on_accent: Color::Rgb(15, 17, 26),
        fg: Color::Rgb(200, 208, 232),
        dim: Color::Rgb(97, 106, 140),
        accent: Color::Rgb(122, 162, 247),
        accent2: Color::Rgb(187, 154, 247),
        good: Color::Rgb(115, 218, 160),
        warn: Color::Rgb(224, 175, 104),
        bad: Color::Rgb(247, 118, 142),
        border: Color::Rgb(59, 66, 97),
        sel: Color::Rgb(36, 42, 66),
    },
    Theme {
        name: "Nord",
        bg: Color::Rgb(46, 52, 64),
        on_accent: Color::Rgb(46, 52, 64),
        fg: Color::Rgb(216, 222, 233),
        dim: Color::Rgb(125, 137, 160),
        accent: Color::Rgb(136, 192, 208),
        accent2: Color::Rgb(129, 161, 193),
        good: Color::Rgb(163, 190, 140),
        warn: Color::Rgb(235, 203, 139),
        bad: Color::Rgb(191, 97, 106),
        border: Color::Rgb(76, 86, 106),
        sel: Color::Rgb(67, 76, 94),
    },
    Theme {
        name: "Ember",
        bg: Color::Rgb(26, 20, 18),
        on_accent: Color::Rgb(26, 20, 18),
        fg: Color::Rgb(235, 219, 196),
        dim: Color::Rgb(135, 118, 100),
        accent: Color::Rgb(250, 189, 47),
        accent2: Color::Rgb(254, 128, 25),
        good: Color::Rgb(184, 187, 38),
        warn: Color::Rgb(254, 128, 25),
        bad: Color::Rgb(251, 73, 52),
        border: Color::Rgb(80, 73, 69),
        sel: Color::Rgb(60, 50, 44),
    },
];
