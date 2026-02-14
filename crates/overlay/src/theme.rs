use eframe::egui;
use keyoverlay_core::{AppConfig, Color, Theme};

/// Resolved color palette from config + theme.
#[derive(Clone, Copy)]
#[allow(dead_code)] // Fields are part of the full theming API; used across overlay modules.
pub struct Palette {
    pub key_bg: egui::Color32,
    pub key_fg: egui::Color32,
    pub mod_bg: egui::Color32,
    pub mod_fg: egui::Color32,
    pub mouse_bg: egui::Color32,
    pub mouse_fg: egui::Color32,
    pub scroll_bg: egui::Color32,
    pub scroll_fg: egui::Color32,
    pub title_fg: egui::Color32,
    pub separator: egui::Color32,
    pub placeholder_fg: egui::Color32,
    pub live_color: egui::Color32,
    pub mouse_body: egui::Color32,
    pub mouse_outline: egui::Color32,
    pub mouse_highlight: egui::Color32,
}

impl Palette {
    pub fn from_config(cfg: &AppConfig) -> Self {
        match cfg.theme {
            Theme::Dark => Self::dark(),
            Theme::Light => Self::light(),
            Theme::Custom => Self::custom(cfg),
        }
    }

    fn dark() -> Self {
        Self {
            key_bg: egui::Color32::from_rgb(55, 55, 75),
            key_fg: egui::Color32::from_rgb(240, 240, 250),
            mod_bg: egui::Color32::from_rgb(80, 120, 200),
            mod_fg: egui::Color32::WHITE,
            mouse_bg: egui::Color32::from_rgb(200, 80, 100),
            mouse_fg: egui::Color32::WHITE,
            scroll_bg: egui::Color32::from_rgb(80, 170, 120),
            scroll_fg: egui::Color32::WHITE,
            title_fg: egui::Color32::from_rgb(140, 140, 170),
            separator: egui::Color32::from_rgb(60, 60, 80),
            placeholder_fg: egui::Color32::from_rgb(100, 100, 130),
            live_color: egui::Color32::from_rgb(100, 220, 100),
            mouse_body: egui::Color32::from_rgb(60, 60, 80),
            mouse_outline: egui::Color32::from_rgb(140, 140, 170),
            mouse_highlight: egui::Color32::from_rgb(100, 160, 255),
        }
    }

    fn light() -> Self {
        Self {
            key_bg: egui::Color32::from_rgb(220, 220, 230),
            key_fg: egui::Color32::from_rgb(30, 30, 40),
            mod_bg: egui::Color32::from_rgb(60, 100, 200),
            mod_fg: egui::Color32::WHITE,
            mouse_bg: egui::Color32::from_rgb(220, 70, 90),
            mouse_fg: egui::Color32::WHITE,
            scroll_bg: egui::Color32::from_rgb(50, 160, 100),
            scroll_fg: egui::Color32::WHITE,
            title_fg: egui::Color32::from_rgb(80, 80, 100),
            separator: egui::Color32::from_rgb(180, 180, 200),
            placeholder_fg: egui::Color32::from_rgb(140, 140, 160),
            live_color: egui::Color32::from_rgb(40, 180, 40),
            mouse_body: egui::Color32::from_rgb(210, 210, 220),
            mouse_outline: egui::Color32::from_rgb(80, 80, 100),
            mouse_highlight: egui::Color32::from_rgb(60, 120, 220),
        }
    }

    fn custom(cfg: &AppConfig) -> Self {
        let mut p = Self::dark(); // fallback base
        p.key_bg = c2e(cfg.custom_key_bg);
        p.key_fg = c2e(cfg.custom_key_fg);
        p.mod_bg = c2e(cfg.custom_mod_bg);
        p.mod_fg = c2e(cfg.custom_mod_fg);
        p.mouse_bg = c2e(cfg.custom_mouse_bg);
        p.mouse_fg = c2e(cfg.custom_mouse_fg);
        p.scroll_bg = c2e(cfg.custom_scroll_bg);
        p.scroll_fg = c2e(cfg.custom_scroll_fg);
        p
    }
}

pub fn c2e(c: Color) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}

pub fn e2c(c: egui::Color32) -> Color {
    Color::rgba(c.r(), c.g(), c.b(), c.a())
}

pub fn apply_alpha(c: egui::Color32, a: f32) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (c.a() as f32 * a) as u8)
}
