use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

// ── Theme ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Dark,
    Light,
    Custom,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Dark, Theme::Light, Theme::Custom];

    pub fn label(self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Custom => "Custom",
        }
    }
}

// ── Overlay Position ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayPosition {
    BottomCenter,
    BottomLeft,
    BottomRight,
    TopCenter,
    TopLeft,
    TopRight,
    Center,
    Manual,
}

impl OverlayPosition {
    pub const ALL: [OverlayPosition; 8] = [
        OverlayPosition::BottomCenter,
        OverlayPosition::BottomLeft,
        OverlayPosition::BottomRight,
        OverlayPosition::TopCenter,
        OverlayPosition::TopLeft,
        OverlayPosition::TopRight,
        OverlayPosition::Center,
        OverlayPosition::Manual,
    ];

    pub fn label(self) -> &'static str {
        match self {
            OverlayPosition::BottomCenter => "Bottom Center",
            OverlayPosition::BottomLeft => "Bottom Left",
            OverlayPosition::BottomRight => "Bottom Right",
            OverlayPosition::TopCenter => "Top Center",
            OverlayPosition::TopLeft => "Top Left",
            OverlayPosition::TopRight => "Top Right",
            OverlayPosition::Center => "Center",
            OverlayPosition::Manual => "Manual (X/Y)",
        }
    }
}

fn default_overlay_x() -> f32 {
    500.0
}

fn default_overlay_y() -> f32 {
    500.0
}

// ── Overlay Layout ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayLayout {
    Horizontal,
    Vertical,
}

impl OverlayLayout {
    pub const ALL: [OverlayLayout; 2] = [OverlayLayout::Horizontal, OverlayLayout::Vertical];

    pub fn label(self) -> &'static str {
        match self {
            OverlayLayout::Horizontal => "Horizontal",
            OverlayLayout::Vertical => "Vertical",
        }
    }
}

// ── Color (serializable) ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

// ── App Config ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    // ── Appearance ──
    pub theme: Theme,
    pub font_size: f32,
    pub overlay_opacity: f32,
    pub background_opacity: f32,
    pub pill_rounding: f32,

    // Custom theme colors (used when theme == Custom).
    pub custom_key_bg: Color,
    pub custom_key_fg: Color,
    pub custom_mod_bg: Color,
    pub custom_mod_fg: Color,
    pub custom_mouse_bg: Color,
    pub custom_mouse_fg: Color,
    pub custom_scroll_bg: Color,
    pub custom_scroll_fg: Color,

    // ── State ──
    pub overlay_enabled: bool,

    // ── Behavior ──
    pub display_duration_secs: f32,
    pub fade_duration_secs: f32,
    pub max_visible_events: usize,
    pub show_keyboard: bool,
    pub show_mouse_clicks: bool,
    pub show_mouse_icon: bool,
    pub show_scroll: bool,

    // ── Position & Layout ──
    pub position: OverlayPosition,
    pub layout: OverlayLayout,
    pub margin_x: f32,
    pub margin_y: f32,
    pub overlay_width: f32,
    pub overlay_height: f32,

    // Manual pixel position (used when position == Manual).
    #[serde(default = "default_overlay_x")]
    pub overlay_x: f32,
    #[serde(default = "default_overlay_y")]
    pub overlay_y: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            font_size: 18.0,
            overlay_opacity: 1.0,
            background_opacity: 0.7,
            pill_rounding: 8.0,

            custom_key_bg: Color::rgb(55, 55, 75),
            custom_key_fg: Color::rgb(240, 240, 250),
            custom_mod_bg: Color::rgb(80, 120, 200),
            custom_mod_fg: Color::rgb(255, 255, 255),
            custom_mouse_bg: Color::rgb(200, 80, 100),
            custom_mouse_fg: Color::rgb(255, 255, 255),
            custom_scroll_bg: Color::rgb(80, 170, 120),
            custom_scroll_fg: Color::rgb(255, 255, 255),

            overlay_enabled: true,

            display_duration_secs: 2.0,
            fade_duration_secs: 0.5,
            max_visible_events: 8,
            show_keyboard: true,
            show_mouse_clicks: true,
            show_mouse_icon: true,
            show_scroll: true,

            position: OverlayPosition::TopRight,
            layout: OverlayLayout::Vertical,
            margin_x: 40.0,
            margin_y: 60.0,
            overlay_width: 280.0,
            overlay_height: 200.0,
            overlay_x: 500.0,
            overlay_y: 500.0,
        }
    }
}

impl AppConfig {
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("keyoverlay").join("config.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        match fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, data);
        }
    }
}

// ── Shared config handle ────────────────────────────────────────────────

pub type SharedConfig = Arc<Mutex<AppConfig>>;

pub fn shared_config() -> SharedConfig {
    Arc::new(Mutex::new(AppConfig::load()))
}
