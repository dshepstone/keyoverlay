use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub highlight_radius_px: u32,
    pub show_cursor_highlight: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            highlight_radius_px: 32,
            show_cursor_highlight: true,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ConfigError {
    NotImplemented,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NotImplemented => {
                write!(f, "configuration persistence is not implemented yet")
            }
        }
    }
}

impl Error for ConfigError {}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        // TODO: Load configuration from disk (likely TOML/JSON in %APPDATA%).
        Err(ConfigError::NotImplemented)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        // TODO: Persist configuration to disk once storage is defined.
        Err(ConfigError::NotImplemented)
    }
}
