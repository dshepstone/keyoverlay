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

// `#[serde(default)]` on the container means a config file with missing
// fields (older version, hand-edited, or partially corrupted) falls back to
// per-field defaults instead of failing wholesale and losing every setting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
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

    /// Clamp all numeric settings into safe ranges and replace non-finite
    /// values (NaN/inf from a hand-edited or corrupted file) with defaults,
    /// so a bad config file can never produce a broken layout or a panic.
    pub fn sanitize(&mut self) {
        let d = AppConfig::default();
        fn sane(v: f32, default: f32, min: f32, max: f32) -> f32 {
            if v.is_finite() {
                v.clamp(min, max)
            } else {
                default
            }
        }

        self.font_size = sane(self.font_size, d.font_size, 8.0, 72.0);
        self.overlay_opacity = sane(self.overlay_opacity, d.overlay_opacity, 0.0, 1.0);
        self.background_opacity = sane(self.background_opacity, d.background_opacity, 0.0, 1.0);
        self.pill_rounding = sane(self.pill_rounding, d.pill_rounding, 0.0, 40.0);
        self.overlay_scale = sane(self.overlay_scale, d.overlay_scale, 0.6, 2.0);

        self.display_duration_secs = sane(
            self.display_duration_secs,
            d.display_duration_secs,
            0.0,
            30.0,
        );
        self.fade_duration_secs = sane(self.fade_duration_secs, d.fade_duration_secs, 0.0, 10.0);
        self.max_visible_events = self.max_visible_events.clamp(1, 64);

        self.cursor_ring_size_px =
            sane(self.cursor_ring_size_px, d.cursor_ring_size_px, 24.0, 200.0);
        self.cursor_ring_thickness_px = sane(
            self.cursor_ring_thickness_px,
            d.cursor_ring_thickness_px,
            2.0,
            12.0,
        );
        self.cursor_ring_opacity = sane(self.cursor_ring_opacity, d.cursor_ring_opacity, 0.2, 1.0);
        self.cursor_glow = sane(self.cursor_glow, d.cursor_glow, 0.0, 1.0);
        self.cursor_hide_after_ms = self.cursor_hide_after_ms.min(600_000);

        self.sound_volume = sane(self.sound_volume, d.sound_volume, 0.0, 1.0);

        self.margin_x = sane(self.margin_x, d.margin_x, 0.0, 1000.0);
        self.margin_y = sane(self.margin_y, d.margin_y, 0.0, 1000.0);
        self.overlay_width = sane(self.overlay_width, d.overlay_width, 50.0, 2000.0);
        self.overlay_height = sane(self.overlay_height, d.overlay_height, 40.0, 2000.0);

        self.display_width_px = sane(self.display_width_px, d.display_width_px, 320.0, 16384.0);
        self.display_height_px = sane(self.display_height_px, d.display_height_px, 240.0, 16384.0);
        self.display_scale = sane(self.display_scale, d.display_scale, 0.5, 4.0);

        // Manual coordinates may legitimately be negative (monitors left of or
        // above the primary), so only bound them loosely.
        self.overlay_x = sane(self.overlay_x, d.overlay_x, -16384.0, 16384.0);
        self.overlay_y = sane(self.overlay_y, d.overlay_y, -16384.0, 16384.0);
    }

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("keyoverlay").join("config.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        let mut config = match fs::read_to_string(&path) {
            Ok(data) => match serde_json::from_str(&data) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!(
                        "[keyoverlay] failed to parse {}: {err}; using defaults",
                        path.display()
                    );
                    Self::default()
                }
            },
            // Missing file on first run is the normal case; other read errors
            // also fall back to defaults.
            Err(_) => Self::default(),
        };
        config.sanitize();
        config
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            if let Err(err) = fs::create_dir_all(parent) {
                eprintln!(
                    "[keyoverlay] failed to create config dir {}: {err}",
                    parent.display()
                );
                return;
            }
        }
        let data = match serde_json::to_string_pretty(self) {
            Ok(data) => data,
            Err(err) => {
                eprintln!("[keyoverlay] failed to serialize config: {err}");
                return;
            }
        };
        // Write to a temp file and rename so a crash or power loss mid-write
        // can never leave a truncated config.json behind.
        let tmp_path = path.with_extension("json.tmp");
        let result = fs::write(&tmp_path, data).and_then(|_| fs::rename(&tmp_path, &path));
        if let Err(err) = result {
            eprintln!(
                "[keyoverlay] failed to save config to {}: {err}",
                path.display()
            );
            let _ = fs::remove_file(&tmp_path);
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
        let config = AppConfig {
            enable_green_cursor_ring: true,
            cursor_theme: Some(CursorTheme::BlueGlow),
            cursor_ring_size_px: 80.0,
            cursor_ring_thickness_px: 6.0,
            cursor_ring_opacity: 0.7,
            cursor_glow: 0.5,
            cursor_hide_after_ms: 1500,
            enable_click_animation: true,
            enable_keystroke_sounds: true,
            sound_volume: 0.8,
            sound_preset: "Mechanical".to_string(),
            ..AppConfig::default()
        };

        let json = serde_json::to_string_pretty(&config).expect("serialize");
        let restored: AppConfig = serde_json::from_str(&json).expect("deserialize");

        assert!(restored.enable_green_cursor_ring);
        assert_eq!(restored.cursor_theme, Some(CursorTheme::BlueGlow));
        assert!((restored.cursor_ring_size_px - 80.0).abs() < f32::EPSILON);
        assert!((restored.cursor_ring_thickness_px - 6.0).abs() < f32::EPSILON);
        assert!((restored.cursor_ring_opacity - 0.7).abs() < f32::EPSILON);
        assert!((restored.cursor_glow - 0.5).abs() < f32::EPSILON);
        assert_eq!(restored.cursor_hide_after_ms, 1500);
        assert!(restored.enable_click_animation);
        assert!(restored.enable_keystroke_sounds);
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
    fn empty_json_deserializes_to_defaults() {
        // Simulates an old or minimal config file: every missing field must
        // fall back to its default instead of failing deserialization.
        let restored: AppConfig = serde_json::from_str("{}").expect("deserialize empty object");
        // show_mouse_event_text has a deliberate migration default of `true`
        // (configs from versions that always showed the text keep doing so),
        // which differs from the `false` used for fresh installs.
        let expected = AppConfig {
            show_mouse_event_text: true,
            ..AppConfig::default()
        };
        assert_eq!(restored, expected);
    }

    #[test]
    fn partial_json_keeps_known_fields_and_defaults_rest() {
        let restored: AppConfig =
            serde_json::from_str(r#"{ "font_size": 30.0, "overlay_enabled": false }"#)
                .expect("deserialize partial object");
        assert!((restored.font_size - 30.0).abs() < f32::EPSILON);
        assert!(!restored.overlay_enabled);
        assert_eq!(restored.theme, AppConfig::default().theme);
        assert_eq!(restored.position, AppConfig::default().position);
    }

    #[test]
    fn sanitize_clamps_out_of_range_values() {
        let mut config = AppConfig {
            font_size: 500.0,
            overlay_opacity: 7.0,
            background_opacity: -3.0,
            overlay_scale: 99.0,
            max_visible_events: 0,
            cursor_ring_size_px: 1.0,
            cursor_ring_thickness_px: 100.0,
            cursor_ring_opacity: 0.0,
            sound_volume: 2.5,
            display_scale: 0.0,
            display_width_px: -100.0,
            cursor_hide_after_ms: u32::MAX,
            ..AppConfig::default()
        };
        config.sanitize();

        assert!((config.font_size - 72.0).abs() < f32::EPSILON);
        assert!((config.overlay_opacity - 1.0).abs() < f32::EPSILON);
        assert!(config.background_opacity.abs() < f32::EPSILON);
        assert!((config.overlay_scale - 2.0).abs() < f32::EPSILON);
        assert_eq!(config.max_visible_events, 1);
        assert!((config.cursor_ring_size_px - 24.0).abs() < f32::EPSILON);
        assert!((config.cursor_ring_thickness_px - 12.0).abs() < f32::EPSILON);
        assert!((config.cursor_ring_opacity - 0.2).abs() < f32::EPSILON);
        assert!((config.sound_volume - 1.0).abs() < f32::EPSILON);
        assert!((config.display_scale - 0.5).abs() < f32::EPSILON);
        assert!((config.display_width_px - 320.0).abs() < f32::EPSILON);
        assert_eq!(config.cursor_hide_after_ms, 600_000);
    }

    #[test]
    fn sanitize_replaces_non_finite_values_with_defaults() {
        let mut config = AppConfig {
            font_size: f32::NAN,
            overlay_scale: f32::INFINITY,
            overlay_x: f32::NEG_INFINITY,
            display_scale: f32::NAN,
            ..AppConfig::default()
        };
        config.sanitize();

        let d = AppConfig::default();
        assert!((config.font_size - d.font_size).abs() < f32::EPSILON);
        assert!((config.overlay_scale - d.overlay_scale).abs() < f32::EPSILON);
        assert!((config.overlay_x - d.overlay_x).abs() < f32::EPSILON);
        assert!((config.display_scale - d.display_scale).abs() < f32::EPSILON);
    }

    #[test]
    fn sanitize_allows_negative_manual_coordinates() {
        // Monitors left of or above the primary have negative coordinates;
        // sanitize must not clamp those to zero.
        let mut config = AppConfig {
            overlay_x: -1920.0,
            overlay_y: -500.0,
            ..AppConfig::default()
        };
        config.sanitize();
        assert!((config.overlay_x + 1920.0).abs() < f32::EPSILON);
        assert!((config.overlay_y + 500.0).abs() < f32::EPSILON);
    }

    #[test]
    fn sanitize_keeps_valid_values_unchanged() {
        let mut config = AppConfig::default();
        let original = config.clone();
        config.sanitize();
        assert_eq!(config, original);
    }

    #[test]
    fn scaled_display_size_points_matches_scale() {
        let config = AppConfig {
            display_width_px: 2560.0,
            display_height_px: 1440.0,
            display_scale: 1.25,
            ..AppConfig::default()
        };
        let [w, h] = config.scaled_display_size_points();
        assert!((w - 2048.0).abs() < 0.01);
        assert!((h - 1152.0).abs() < 0.01);
    }

    #[test]
    fn manual_lower_right_position_accounts_for_size_and_margins() {
        let config = AppConfig {
            display_width_px: 1920.0,
            display_height_px: 1080.0,
            display_scale: 1.0,
            overlay_width: 280.0,
            overlay_height: 200.0,
            margin_x: 40.0,
            margin_y: 60.0,
            ..AppConfig::default()
        };
        let (x, y) = config.manual_lower_right_position();
        assert!((x - 1600.0).abs() < 0.01);
        assert!((y - 820.0).abs() < 0.01);
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
        assert_eq!(
            CursorTheme::WhiteCircle.desc().shape,
            CursorShape::FilledDot
        );
    }

    #[test]
    fn cursor_theme_glow_defaults() {
        // GreenRing has no default glow
        assert!((CursorTheme::GreenRing.desc().default_glow).abs() < f32::EPSILON);
        // BlueGlow has glow
        assert!(CursorTheme::BlueGlow.desc().default_glow > 0.0);
        // PurpleHaze has the most glow
        assert!(
            CursorTheme::PurpleHaze.desc().default_glow > CursorTheme::BlueGlow.desc().default_glow
        );
    }

    #[test]
    fn cursor_ring_settings_from_config_themes() {
        use super::*;
        let mut cfg = AppConfig {
            enable_green_cursor_ring: true,
            ..AppConfig::default()
        };

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
