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

fn default_show_mouse_event_text() -> bool {
    true
}

fn default_display_width_px() -> f32 {
    1920.0
}

fn default_display_height_px() -> f32 {
    1080.0
}

fn default_display_scale() -> f32 {
    1.25
}

fn default_overlay_x() -> f32 {
    1216.0
}

fn default_overlay_y() -> f32 {
    604.0
}

fn default_overlay_scale() -> f32 {
    1.0
}

fn default_cursor_ring_size_px() -> f32 {
    56.0
}

fn default_cursor_ring_thickness_px() -> f32 {
    4.0
}

fn default_cursor_ring_opacity() -> f32 {
    0.9
}

fn default_cursor_glow() -> f32 {
    0.0
}

fn default_cursor_hide_after_ms() -> u32 {
    0
}

fn default_cursor_hotspot_center() -> bool {
    true
}

fn default_sound_volume() -> f32 {
    0.5
}

fn default_sound_preset() -> String {
    "Typewriter".to_string()
}

// ── Cursor Theme ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorTheme {
    GreenRing,
    BlueGlow,
    RedDot,
    YellowPulse,
    PurpleHaze,
    WhiteCircle,
}

/// Whether a cursor theme renders as a ring (stroke) or filled circle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    Ring,
    FilledDot,
}

/// Full rendering description for a cursor theme.
#[derive(Debug, Clone, Copy)]
pub struct CursorThemeDesc {
    pub base_color: Color,
    pub shape: CursorShape,
    /// Default glow intensity (0 = none, >0 = soft outer glow).
    pub default_glow: f32,
    /// Accent color used for click animation "pop".
    pub click_accent: Color,
}

impl CursorTheme {
    pub const ALL: [CursorTheme; 6] = [
        CursorTheme::GreenRing,
        CursorTheme::BlueGlow,
        CursorTheme::RedDot,
        CursorTheme::YellowPulse,
        CursorTheme::PurpleHaze,
        CursorTheme::WhiteCircle,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CursorTheme::GreenRing => "Green Ring",
            CursorTheme::BlueGlow => "Blue Glow",
            CursorTheme::RedDot => "Red Dot",
            CursorTheme::YellowPulse => "Yellow Pulse",
            CursorTheme::PurpleHaze => "Purple Haze",
            CursorTheme::WhiteCircle => "White Circle",
        }
    }

    pub fn color(self) -> Color {
        self.desc().base_color
    }

    /// Full rendering descriptor for this theme.
    pub fn desc(self) -> CursorThemeDesc {
        match self {
            CursorTheme::GreenRing => CursorThemeDesc {
                base_color: Color::rgb(100, 220, 100),
                shape: CursorShape::Ring,
                default_glow: 0.0,
                click_accent: Color::rgb(180, 255, 180),
            },
            CursorTheme::BlueGlow => CursorThemeDesc {
                base_color: Color::rgb(80, 160, 255),
                shape: CursorShape::Ring,
                default_glow: 0.4,
                click_accent: Color::rgb(160, 210, 255),
            },
            CursorTheme::RedDot => CursorThemeDesc {
                base_color: Color::rgb(240, 60, 60),
                shape: CursorShape::FilledDot,
                default_glow: 0.0,
                click_accent: Color::rgb(255, 140, 140),
            },
            CursorTheme::YellowPulse => CursorThemeDesc {
                base_color: Color::rgb(255, 210, 60),
                shape: CursorShape::Ring,
                default_glow: 0.2,
                click_accent: Color::rgb(255, 240, 160),
            },
            CursorTheme::PurpleHaze => CursorThemeDesc {
                base_color: Color::rgb(180, 100, 255),
                shape: CursorShape::Ring,
                default_glow: 0.5,
                click_accent: Color::rgb(220, 180, 255),
            },
            CursorTheme::WhiteCircle => CursorThemeDesc {
                base_color: Color::rgb(240, 240, 240),
                shape: CursorShape::FilledDot,
                default_glow: 0.0,
                click_accent: Color::rgb(255, 255, 255),
            },
        }
    }
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
    #[serde(default = "default_overlay_scale")]
    pub overlay_scale: f32,

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
    #[serde(default = "default_show_mouse_event_text")]
    pub show_mouse_event_text: bool,
    pub show_scroll: bool,
    #[serde(default)]
    pub enable_green_cursor_ring: bool,
    #[serde(default = "default_cursor_ring_size_px")]
    pub cursor_ring_size_px: f32,
    #[serde(default = "default_cursor_ring_thickness_px")]
    pub cursor_ring_thickness_px: f32,
    #[serde(default = "default_cursor_ring_opacity")]
    pub cursor_ring_opacity: f32,
    #[serde(default)]
    pub cursor_theme: Option<CursorTheme>,
    #[serde(default = "default_cursor_glow")]
    pub cursor_glow: f32,
    #[serde(default = "default_cursor_hide_after_ms")]
    pub cursor_hide_after_ms: u32,
    #[serde(default)]
    pub enable_click_animation: bool,
    /// When true, the cursor hotspot (arrow tip) is at the center of the circle.
    /// When false, the cursor hotspot sits on the lower-right edge of the circle
    /// (the circle shifts up-left from the arrow tip at 45°).
    #[serde(default = "default_cursor_hotspot_center")]
    pub cursor_hotspot_center: bool,

    // ── Sounds ──
    #[serde(default)]
    pub enable_keystroke_sounds: bool,
    #[serde(default = "default_sound_volume")]
    pub sound_volume: f32,
    #[serde(default = "default_sound_preset")]
    pub sound_preset: String,

    // ── Position & Layout ──
    pub position: OverlayPosition,
    pub layout: OverlayLayout,
    pub margin_x: f32,
    pub margin_y: f32,
    pub overlay_width: f32,
    pub overlay_height: f32,

    // Display baseline used to compute preset and manual positions.
    #[serde(default = "default_display_width_px")]
    pub display_width_px: f32,
    #[serde(default = "default_display_height_px")]
    pub display_height_px: f32,
    #[serde(default = "default_display_scale")]
    pub display_scale: f32,

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
            overlay_scale: 1.0,

            custom_key_bg: Color::rgb(55, 55, 75),
            custom_key_fg: Color::rgb(240, 240, 250),
            custom_mod_bg: Color::rgb(80, 120, 200),
            custom_mod_fg: Color::rgb(255, 255, 255),
            custom_mouse_bg: Color::rgb(200, 80, 100),
            custom_mouse_fg: Color::rgb(255, 255, 255),
            custom_scroll_bg: Color::rgb(80, 170, 120),
            custom_scroll_fg: Color::rgb(255, 255, 255),

            overlay_enabled: true,

            display_duration_secs: 0.5,
            fade_duration_secs: 0.5,
            max_visible_events: 8,
            show_keyboard: true,
            show_mouse_clicks: true,
            show_mouse_icon: true,
            show_mouse_event_text: false,
            show_scroll: true,
            enable_green_cursor_ring: false,
            cursor_ring_size_px: default_cursor_ring_size_px(),
            cursor_ring_thickness_px: default_cursor_ring_thickness_px(),
            cursor_ring_opacity: default_cursor_ring_opacity(),
            cursor_theme: None,
            cursor_glow: default_cursor_glow(),
            cursor_hide_after_ms: default_cursor_hide_after_ms(),
            enable_click_animation: false,
            cursor_hotspot_center: true,

            enable_keystroke_sounds: false,
            sound_volume: default_sound_volume(),
            sound_preset: default_sound_preset(),

            position: OverlayPosition::TopRight,
            layout: OverlayLayout::Vertical,
            margin_x: 40.0,
            margin_y: 60.0,
            overlay_width: 280.0,
            overlay_height: 200.0,
            display_width_px: 1920.0,
            display_height_px: 1080.0,
            display_scale: 1.25,
            overlay_x: 1216.0,
            overlay_y: 604.0,
        }
    }
}

impl AppConfig {
    pub fn scaled_display_size_points(&self) -> [f32; 2] {
        let scale = self.display_scale.max(1.0);
        [
            (self.display_width_px / scale).max(1.0),
            (self.display_height_px / scale).max(1.0),
        ]
    }

    pub fn manual_lower_right_position(&self) -> (f32, f32) {
        let [sw, sh] = self.scaled_display_size_points();
        (
            (sw - self.overlay_width - self.margin_x).max(0.0),
            (sh - self.overlay_height - self.margin_y).max(0.0),
        )
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_serialize_deserialize_roundtrip() {
        let mut config = AppConfig::default();
        config.enable_green_cursor_ring = true;
        config.cursor_theme = Some(CursorTheme::BlueGlow);
        config.cursor_ring_size_px = 80.0;
        config.cursor_ring_thickness_px = 6.0;
        config.cursor_ring_opacity = 0.7;
        config.cursor_glow = 0.5;
        config.cursor_hide_after_ms = 1500;
        config.enable_click_animation = true;
        config.enable_keystroke_sounds = true;
        config.sound_volume = 0.8;
        config.sound_preset = "Mechanical".to_string();

        let json = serde_json::to_string_pretty(&config).expect("serialize");
        let restored: AppConfig = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(restored.enable_green_cursor_ring, true);
        assert_eq!(restored.cursor_theme, Some(CursorTheme::BlueGlow));
        assert!((restored.cursor_ring_size_px - 80.0).abs() < f32::EPSILON);
        assert!((restored.cursor_ring_thickness_px - 6.0).abs() < f32::EPSILON);
        assert!((restored.cursor_ring_opacity - 0.7).abs() < f32::EPSILON);
        assert!((restored.cursor_glow - 0.5).abs() < f32::EPSILON);
        assert_eq!(restored.cursor_hide_after_ms, 1500);
        assert_eq!(restored.enable_click_animation, true);
        assert_eq!(restored.enable_keystroke_sounds, true);
        assert!((restored.sound_volume - 0.8).abs() < f32::EPSILON);
        assert_eq!(restored.sound_preset, "Mechanical");
    }

    #[test]
    fn config_default_values() {
        let config = AppConfig::default();
        assert!(!config.enable_green_cursor_ring);
        assert!((config.cursor_ring_size_px - 56.0).abs() < f32::EPSILON);
        assert!((config.cursor_ring_thickness_px - 4.0).abs() < f32::EPSILON);
        assert!((config.cursor_ring_opacity - 0.9).abs() < f32::EPSILON);
        assert!((config.cursor_glow).abs() < f32::EPSILON);
        assert_eq!(config.cursor_hide_after_ms, 0);
        assert!(!config.enable_click_animation);
        assert!(!config.enable_keystroke_sounds);
        assert!((config.sound_volume - 0.5).abs() < f32::EPSILON);
        assert_eq!(config.sound_preset, "Typewriter");
    }

    #[test]
    fn cursor_theme_all_have_labels_and_colors() {
        for theme in CursorTheme::ALL {
            let label = theme.label();
            assert!(!label.is_empty(), "{:?} has empty label", theme);

            let desc = theme.desc();
            // Every theme must have a non-zero color
            assert!(
                desc.base_color.r > 0 || desc.base_color.g > 0 || desc.base_color.b > 0,
                "{:?} has zero color",
                theme
            );
            // Click accent must also be non-zero
            assert!(
                desc.click_accent.r > 0 || desc.click_accent.g > 0 || desc.click_accent.b > 0,
                "{:?} has zero click accent",
                theme
            );
        }
    }

    #[test]
    fn cursor_theme_shapes() {
        assert_eq!(CursorTheme::GreenRing.desc().shape, CursorShape::Ring);
        assert_eq!(CursorTheme::BlueGlow.desc().shape, CursorShape::Ring);
        assert_eq!(CursorTheme::RedDot.desc().shape, CursorShape::FilledDot);
        assert_eq!(CursorTheme::YellowPulse.desc().shape, CursorShape::Ring);
        assert_eq!(CursorTheme::PurpleHaze.desc().shape, CursorShape::Ring);
        assert_eq!(CursorTheme::WhiteCircle.desc().shape, CursorShape::FilledDot);
    }

    #[test]
    fn cursor_theme_glow_defaults() {
        // GreenRing has no default glow
        assert!((CursorTheme::GreenRing.desc().default_glow).abs() < f32::EPSILON);
        // BlueGlow has glow
        assert!(CursorTheme::BlueGlow.desc().default_glow > 0.0);
        // PurpleHaze has the most glow
        assert!(CursorTheme::PurpleHaze.desc().default_glow > CursorTheme::BlueGlow.desc().default_glow);
    }

    #[test]
    fn cursor_ring_settings_from_config_themes() {
        use super::*;
        let mut cfg = AppConfig::default();
        cfg.enable_green_cursor_ring = true;

        // Default theme (None -> GreenRing)
        cfg.cursor_theme = None;
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.cursor_theme, None);

        // Each theme variant survives roundtrip
        for theme in CursorTheme::ALL {
            cfg.cursor_theme = Some(theme);
            let json = serde_json::to_string(&cfg).unwrap();
            let restored: AppConfig = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.cursor_theme, Some(theme));
        }
    }
}
