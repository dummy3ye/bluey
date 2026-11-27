//! User configuration loaded from `$XDG_CONFIG_HOME/bluey/config.toml`.

use std::{fs, path::PathBuf};

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Use the terminal's default background instead of a solid theme color.
    pub transparent_background: bool,
    /// Theme name: "midnight", "nord", or "ember".
    pub theme: String,
}

impl Default for Config {
    fn default() -> Self {
        Self { transparent_background: true, theme: "midnight".to_owned() }
    }
}

impl Config {
    pub fn load() -> Self {
        Self::path().and_then(|path| Self::from_path(&path).ok()).unwrap_or_default()
    }

    pub fn path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
        base.map(|base| base.join("bluey").join("config.toml"))
    }

    fn from_path(path: &PathBuf) -> Result<Self> {
        let text = fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }
}
