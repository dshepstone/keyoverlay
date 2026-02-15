use eframe::egui;
use keyoverlay_core::{AppConfig, Color, Theme};

/// Resolved color palette from config + theme.
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct Palette {
    pub key_bg: egui::Color32,
    pub key_fg: egui::Color32,
    pub mod_bg: egui::Color32,
    pub mod_fg: egui::Color32,
    pub mouse_bg: egui::Color32,
    pub mouse_fg: egui::Color32,
    pub scroll_bg: egui::Color32,
    pub scroll_fg: egui::Color32,
    pub tray_bg: egui::Color32,
    pub tray_border: egui::Color32,
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
            Theme::Minimal => Self::minimal(),
            Theme::HighContrast => Self::high_contrast(),
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
            tray_bg: egui::Color32::from_rgba_unmultiplied(30, 30, 42, 230),
            tray_border: egui::Color32::from_rgba_unmultiplied(70, 70, 90, 180),
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
            mod_bg: egui::Color32::from_rgb(122, 71, 255),
            mod_fg: egui::Color32::WHITE,
            mouse_bg: egui::Color32::from_rgb(220, 70, 90),
            mouse_fg: egui::Color32::WHITE,
            scroll_bg: egui::Color32::from_rgb(50, 160, 100),
            scroll_fg: egui::Color32::WHITE,
            tray_bg: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 240),
            tray_border: egui::Color32::from_rgba_unmultiplied(200, 200, 215, 120),
            title_fg: egui::Color32::from_rgb(80, 80, 100),
            separator: egui::Color32::from_rgb(180, 180, 200),
            placeholder_fg: egui::Color32::from_rgb(140, 140, 160),
            live_color: egui::Color32::from_rgb(40, 180, 40),
            mouse_body: egui::Color32::from_rgb(210, 210, 220),
            mouse_outline: egui::Color32::from_rgb(80, 80, 100),
            mouse_highlight: egui::Color32::from_rgb(60, 120, 220),
        }
    }

    fn minimal() -> Self {
        Self {
            key_bg: egui::Color32::from_rgba_unmultiplied(240, 240, 245, 200),
            key_fg: egui::Color32::from_rgb(60, 60, 70),
            mod_bg: egui::Color32::from_rgba_unmultiplied(90, 90, 110, 200),
            mod_fg: egui::Color32::WHITE,
            mouse_bg: egui::Color32::from_rgba_unmultiplied(120, 120, 140, 200),
            mouse_fg: egui::Color32::WHITE,
            scroll_bg: egui::Color32::from_rgba_unmultiplied(120, 120, 140, 200),
            scroll_fg: egui::Color32::WHITE,
            tray_bg: egui::Color32::from_rgba_unmultiplied(248, 248, 252, 210),
            tray_border: egui::Color32::TRANSPARENT,
            title_fg: egui::Color32::from_rgb(100, 100, 120),
            separator: egui::Color32::from_rgb(210, 210, 220),
            placeholder_fg: egui::Color32::from_rgb(170, 170, 185),
            live_color: egui::Color32::from_rgb(80, 180, 80),
            mouse_body: egui::Color32::from_rgb(220, 220, 230),
            mouse_outline: egui::Color32::from_rgb(150, 150, 170),
            mouse_highlight: egui::Color32::from_rgb(100, 140, 200),
        }
    }

    fn high_contrast() -> Self {
        Self {
            key_bg: egui::Color32::from_rgb(0, 0, 0),
            key_fg: egui::Color32::from_rgb(255, 255, 0),
            mod_bg: egui::Color32::from_rgb(0, 0, 200),
            mod_fg: egui::Color32::WHITE,
            mouse_bg: egui::Color32::from_rgb(200, 0, 0),
            mouse_fg: egui::Color32::WHITE,
            scroll_bg: egui::Color32::from_rgb(0, 160, 0),
            scroll_fg: egui::Color32::WHITE,
            tray_bg: egui::Color32::from_rgb(0, 0, 0),
            tray_border: egui::Color32::from_rgb(255, 255, 0),
            title_fg: egui::Color32::WHITE,
            separator: egui::Color32::from_rgb(255, 255, 0),
            placeholder_fg: egui::Color32::from_rgb(200, 200, 200),
            live_color: egui::Color32::from_rgb(0, 255, 0),
            mouse_body: egui::Color32::from_rgb(30, 30, 30),
            mouse_outline: egui::Color32::from_rgb(255, 255, 0),
            mouse_highlight: egui::Color32::from_rgb(255, 255, 0),
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
        p.tray_bg = c2e(cfg.custom_tray_bg);
        p.tray_border = c2e(cfg.custom_tray_border);
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
