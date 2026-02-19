use std::collections::HashSet;

use eframe::egui;
use keyoverlay_core::{CursorTheme, OverlayLayout, OverlayPosition, Theme};

use crate::theme::{c2e, e2c};
use crate::App;

const DISPLAY_PRESETS: &[(f32, f32, &str)] = &[
    (1920.0, 1080.0, "1920 x 1080"),
    (2560.0, 1440.0, "2560 x 1440"),
    (3840.0, 2160.0, "3840 x 2160"),
    (1600.0, 900.0, "1600 x 900"),
    (1366.0, 768.0, "1366 x 768"),
];

const SCALE_PRESETS: &[(f32, &str)] = &[
    (1.0, "100%"),
    (1.25, "125%"),
    (1.5, "150%"),
    (1.75, "175%"),
    (2.0, "200%"),
];

const SOUND_PRESETS: &[&str] = &[
    "Typewriter",
    "Mechanical",
    "Soft Click",
    "Pop",
    "None",
];

// ── Colors ──────────────────────────────────────────────────────────────

const SIDEBAR_BG: egui::Color32 = egui::Color32::from_rgb(24, 24, 32);
const SIDEBAR_TEXT: egui::Color32 = egui::Color32::from_rgb(160, 160, 180);
const SIDEBAR_ACTIVE_TEXT: egui::Color32 = egui::Color32::WHITE;
const SIDEBAR_ACTIVE_BG: egui::Color32 = egui::Color32::from_rgb(50, 50, 70);
const SIDEBAR_HOVER_BG: egui::Color32 = egui::Color32::from_rgb(38, 38, 52);

const MAIN_BG: egui::Color32 = egui::Color32::from_rgb(30, 30, 40);
const CARD_BG: egui::Color32 = egui::Color32::from_rgb(38, 38, 52);
const CARD_BORDER: egui::Color32 = egui::Color32::from_rgb(55, 55, 72);
const HEADING_TEXT: egui::Color32 = egui::Color32::from_rgb(220, 220, 240);
const BODY_TEXT: egui::Color32 = egui::Color32::from_rgb(180, 180, 200);
const MUTED_TEXT: egui::Color32 = egui::Color32::from_rgb(120, 120, 150);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(100, 120, 255);
const SUCCESS: egui::Color32 = egui::Color32::from_rgb(80, 200, 120);

const TOGGLE_ON_BG: egui::Color32 = egui::Color32::from_rgb(80, 200, 120);
const TOGGLE_OFF_BG: egui::Color32 = egui::Color32::from_rgb(70, 70, 90);
const TOGGLE_KNOB: egui::Color32 = egui::Color32::WHITE;

const SIDEBAR_WIDTH: f32 = 180.0;
const CARD_ROUNDING: f32 = 12.0;
const CARD_PADDING: f32 = 16.0;

// ── Settings Tabs ────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Keystroke,
    Cursor,
    Sounds,
    Position,
    License,
    About,
}

impl SettingsTab {
    const ALL: [SettingsTab; 7] = [
        SettingsTab::General,
        SettingsTab::Keystroke,
        SettingsTab::Cursor,
        SettingsTab::Sounds,
        SettingsTab::Position,
        SettingsTab::License,
        SettingsTab::About,
    ];

    fn label(self) -> &'static str {
        match self {
            SettingsTab::General => "General",
            SettingsTab::Keystroke => "Keystroke",
            SettingsTab::Cursor => "Cursor",
            SettingsTab::Sounds => "Sounds",
            SettingsTab::Position => "Position",
            SettingsTab::License => "License",
            SettingsTab::About => "About",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            SettingsTab::General => "\u{2699}",     // ⚙
            SettingsTab::Keystroke => "\u{2328}",   // ⌨
            SettingsTab::Cursor => "\u{1F5B1}",     // 🖱  (fallback: pointer)
            SettingsTab::Sounds => "\u{266B}",      // ♫
            SettingsTab::Position => "\u{2316}",    // ⌖
            SettingsTab::License => "\u{1F4C4}",    // 📄
            SettingsTab::About => "\u{2139}",       // ℹ
        }
    }
}

fn load_header_icon_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let texture_id = egui::Id::new("settings_header_icon_texture");

    if let Some(texture) = ctx.data_mut(|data| data.get_temp::<egui::TextureHandle>(texture_id)) {
        return Some(texture);
    }

    let bytes = include_bytes!("../../app/icon.png");
    let rgba = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());

    let texture = ctx.load_texture(
        "settings_header_icon_texture",
        color_image,
        egui::TextureOptions {
            magnification: egui::TextureFilter::Nearest,
            minification: egui::TextureFilter::Linear,
            ..Default::default()
        },
    );

    ctx.data_mut(|data| data.insert_temp(texture_id, texture.clone()));
    Some(texture)
}

// ── Command press detection (prevents double-fire) ──────────────────────

fn command_press_memory_key() -> egui::Id {
    egui::Id::new("overlay_command_pressed_ids")
}

fn reset_command_press_state_if_needed(ui: &egui::Ui) {
    if !ui.input(|i| i.pointer.primary_down()) {
        ui.ctx().data_mut(|data| {
            data.insert_temp(command_press_memory_key(), HashSet::<egui::Id>::new())
        });
    }
}

fn command_activated_on_press(
    ui: &egui::Ui,
    button_id: egui::Id,
    response: &egui::Response,
) -> bool {
    let pressed_now = response.hovered() && ui.input(|i| i.pointer.primary_pressed());
    if pressed_now {
        let should_fire = ui.ctx().data_mut(|data| {
            let mut fired = data
                .get_temp::<HashSet<egui::Id>>(command_press_memory_key())
                .unwrap_or_default();
            let fresh = fired.insert(button_id);
            data.insert_temp(command_press_memory_key(), fired);
            fresh
        });
        if should_fire {
            return true;
        }
    }

    if response.clicked() {
        if response.clicked_by(egui::PointerButton::Primary) {
            let was_press_fired = ui.ctx().data_mut(|data| {
                let mut fired = data
                    .get_temp::<HashSet<egui::Id>>(command_press_memory_key())
                    .unwrap_or_default();
                let had = fired.remove(&button_id);
                data.insert_temp(command_press_memory_key(), fired);
                had
            });
            if !was_press_fired {
                return true;
            }
        } else {
            return true;
        }
    }

    false
}

// ── Main draw entry point ───────────────────────────────────────────────

pub fn draw_settings(ctx: &egui::Context, app: &mut App) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = MAIN_BG;
    ctx.set_visuals(visuals);

    egui::CentralPanel::default()
        .frame(egui::Frame::central_panel(&ctx.style()).fill(MAIN_BG).inner_margin(0.0))
        .show(ctx, |ui| {
            reset_command_press_state_if_needed(ui);
            let available = ui.available_rect_before_wrap();

            // ── Left Sidebar ──
            let sidebar_rect = egui::Rect::from_min_size(
                available.min,
                egui::vec2(SIDEBAR_WIDTH, available.height()),
            );
            let main_rect = egui::Rect::from_min_size(
                egui::pos2(available.min.x + SIDEBAR_WIDTH, available.min.y),
                egui::vec2(available.width() - SIDEBAR_WIDTH, available.height()),
            );

            // Paint sidebar background
            ui.painter().rect_filled(sidebar_rect, 0.0, SIDEBAR_BG);

            // Sidebar content
            let mut sidebar_ui =
                ui.child_ui(sidebar_rect.shrink2(egui::vec2(0.0, 0.0)), egui::Layout::top_down(egui::Align::LEFT));
            draw_sidebar(ctx, &mut sidebar_ui, app);

            // ── Main Panel ──
            let mut main_ui = ui.child_ui(
                main_rect.shrink2(egui::vec2(24.0, 0.0)),
                egui::Layout::top_down(egui::Align::LEFT),
            );
            draw_main_panel(ctx, &mut main_ui, app);
        });
}

// ── Sidebar ─────────────────────────────────────────────────────────────

fn draw_sidebar(ctx: &egui::Context, ui: &mut egui::Ui, app: &mut App) {
    ui.add_space(16.0);

    // App icon + name
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        if let Some(icon_texture) = load_header_icon_texture(ctx) {
            ui.add(egui::Image::new((icon_texture.id(), egui::vec2(28.0, 28.0))));
            ui.add_space(8.0);
        }
        ui.label(
            egui::RichText::new("KeyOverlay")
                .color(HEADING_TEXT)
                .size(17.0)
                .strong(),
        );
    });

    ui.add_space(20.0);

    // "SETTINGS" label
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        ui.label(
            egui::RichText::new("SETTINGS")
                .color(MUTED_TEXT)
                .size(10.0)
                .strong(),
        );
    });
    ui.add_space(6.0);

    // Nav items
    for tab in SettingsTab::ALL {
        let selected = app.active_tab == tab;
        let item_rect = egui::Rect::from_min_size(
            egui::pos2(ui.min_rect().min.x, ui.cursor().min.y),
            egui::vec2(SIDEBAR_WIDTH, 36.0),
        );

        let response = ui.allocate_rect(item_rect, egui::Sense::click());
        let hovered = response.hovered();

        // Background
        if selected {
            ui.painter().rect_filled(item_rect, 0.0, SIDEBAR_ACTIVE_BG);
            // Active indicator bar
            let bar = egui::Rect::from_min_size(
                item_rect.min,
                egui::vec2(3.0, item_rect.height()),
            );
            ui.painter().rect_filled(bar, 0.0, ACCENT);
        } else if hovered {
            ui.painter().rect_filled(item_rect, 0.0, SIDEBAR_HOVER_BG);
        }

        // Icon + label
        let text_color = if selected { SIDEBAR_ACTIVE_TEXT } else { SIDEBAR_TEXT };
        let text_pos = egui::pos2(item_rect.min.x + 20.0, item_rect.center().y);
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_CENTER,
            format!("{}  {}", tab.icon(), tab.label()),
            egui::FontId::proportional(13.0),
            text_color,
        );

        if response.clicked() {
            app.active_tab = tab;
        }
    }

    // Push the status area to the bottom
    let bottom_y = ui.min_rect().max.y - 80.0;
    let current_y = ui.cursor().min.y;
    if bottom_y > current_y {
        ui.add_space(bottom_y - current_y);
    }

    // Status indicator at bottom of sidebar
    ui.add_space(8.0);
    let sep_rect = egui::Rect::from_min_size(
        egui::pos2(ui.min_rect().min.x + 16.0, ui.cursor().min.y),
        egui::vec2(SIDEBAR_WIDTH - 32.0, 1.0),
    );
    ui.painter().rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(45, 45, 62));
    ui.add_space(12.0);

    ui.horizontal(|ui| {
        ui.add_space(20.0);

        let enabled = app.draft.overlay_enabled;
        let status_color = if enabled { SUCCESS } else { MUTED_TEXT };

        // Pulsing dot
        let (dot_rect, _) =
            ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        if enabled {
            let pulse = ((ctx.input(|i| i.time) * 2.0).sin() * 0.5 + 0.5) as f32;
            let a = (150.0 + 105.0 * pulse) as u8;
            ui.painter().circle_filled(
                dot_rect.center(),
                4.0,
                egui::Color32::from_rgba_unmultiplied(80, 200, 120, a),
            );
        } else {
            ui.painter().circle_filled(dot_rect.center(), 4.0, MUTED_TEXT);
        }

        ui.add_space(6.0);
        let status_text = if enabled { "Overlay active" } else { "Overlay off" };
        ui.label(
            egui::RichText::new(status_text)
                .color(status_color)
                .size(11.0),
        );
    });
}

// ── Main Panel ──────────────────────────────────────────────────────────

fn draw_main_panel(_ctx: &egui::Context, ui: &mut egui::Ui, app: &mut App) {
    // Page title
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(app.active_tab.label())
                .color(HEADING_TEXT)
                .size(24.0)
                .strong(),
        );

        // ON/OFF toggle in top-right
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            let enabled = app.draft.overlay_enabled;
            let (btn_text, btn_color, btn_bg) = if enabled {
                ("ON", egui::Color32::WHITE, TOGGLE_ON_BG)
            } else {
                ("OFF", BODY_TEXT, TOGGLE_OFF_BG)
            };
            let btn = egui::Button::new(
                egui::RichText::new(btn_text).color(btn_color).size(12.0).strong(),
            )
            .fill(btn_bg)
            .rounding(egui::Rounding::same(12.0))
            .min_size(egui::vec2(52.0, 26.0));

            let toggle_id = egui::Id::new("cmd::overlay_toggle");
            let toggle_resp = ui.push_id(toggle_id, |ui| ui.add(btn)).inner;
            if command_activated_on_press(ui, toggle_id, &toggle_resp) {
                app.draft.overlay_enabled = !app.draft.overlay_enabled;
                app.apply();
            }
        });
    });

    ui.add_space(16.0);

    // Scrollable tab content
    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            match app.active_tab {
                SettingsTab::General => tab_general(app, ui),
                SettingsTab::Keystroke => tab_keystroke(app, ui),
                SettingsTab::Cursor => tab_cursor(app, ui),
                SettingsTab::Sounds => tab_sounds(app, ui),
                SettingsTab::Position => tab_position(app, ui),
                SettingsTab::License => tab_license(ui),
                SettingsTab::About => tab_about(ui),
            }

            // Bottom status message
            ui.add_space(16.0);
            if let Some((msg, t)) = &app.status_msg {
                if t.elapsed().as_secs() < 3 {
                    ui.label(
                        egui::RichText::new(msg).color(SUCCESS).size(12.0),
                    );
                }
            }
            ui.add_space(8.0);
        });
}

// ── Reusable Components ─────────────────────────────────────────────────

/// Begin a rounded card section. Returns a closure to end it.
fn begin_card(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(title)
            .color(HEADING_TEXT)
            .size(14.0)
            .strong(),
    );
    ui.add_space(6.0);
}

/// Wrap content in a card-styled frame.
fn card_frame(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(CARD_BG)
        .stroke(egui::Stroke::new(1.0, CARD_BORDER))
        .rounding(egui::Rounding::same(CARD_ROUNDING))
        .inner_margin(egui::Margin::same(CARD_PADDING))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add_contents(ui);
        });
}

/// A toggle row: icon + label on left, modern switch on right.
fn toggle_row(ui: &mut egui::Ui, id_str: &str, label: &str, value: &mut bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(BODY_TEXT).size(13.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            changed = modern_toggle(ui, id_str, value);
        });
    });
    changed
}

/// Modern toggle switch widget.
fn modern_toggle(ui: &mut egui::Ui, id_str: &str, value: &mut bool) -> bool {
    let desired_size = egui::vec2(40.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    let mut changed = false;

    if response.clicked() {
        *value = !*value;
        changed = true;
    }

    let _ = id_str; // id used for uniqueness via allocate

    let anim_t = ui.ctx().animate_bool(response.id, *value);
    let bg_color = egui::Color32::from_rgb(
        lerp_u8(TOGGLE_OFF_BG.r(), TOGGLE_ON_BG.r(), anim_t),
        lerp_u8(TOGGLE_OFF_BG.g(), TOGGLE_ON_BG.g(), anim_t),
        lerp_u8(TOGGLE_OFF_BG.b(), TOGGLE_ON_BG.b(), anim_t),
    );

    let knob_radius = 8.0;
    let knob_x = rect.left() + knob_radius + 3.0
        + anim_t * (rect.width() - 2.0 * knob_radius - 6.0);
    let knob_center = egui::pos2(knob_x, rect.center().y);

    ui.painter()
        .rect_filled(rect, egui::Rounding::same(11.0), bg_color);
    ui.painter()
        .circle_filled(knob_center, knob_radius, TOGGLE_KNOB);

    changed
}

/// A slider row with label.
fn slider_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(BODY_TEXT).size(13.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let slider = egui::Slider::new(value, range)
                .suffix(suffix)
                .clamp_to_range(true);
            changed = ui.add(slider).changed();
        });
    });
    changed
}

/// A slider row for integer values.
fn slider_row_i32(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut i32,
    range: std::ops::RangeInclusive<i32>,
    suffix: &str,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(BODY_TEXT).size(13.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let slider = egui::Slider::new(value, range)
                .suffix(suffix)
                .clamp_to_range(true);
            changed = ui.add(slider).changed();
        });
    });
    changed
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let t = t.clamp(0.0, 1.0);
    ((a as f32) * (1.0 - t) + (b as f32) * t).round() as u8
}

fn color_row(ui: &mut egui::Ui, label: &str, color: &mut keyoverlay_core::Color) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(BODY_TEXT).size(13.0));
        let ec = c2e(*color);
        let mut rgba = [ec.r(), ec.g(), ec.b()];
        if ui.color_edit_button_srgb(&mut rgba).changed() {
            let ec = egui::Color32::from_rgb(rgba[0], rgba[1], rgba[2]);
            *color = e2c(ec);
        }
    });
}

// ── Tab: General ────────────────────────────────────────────────────────

fn tab_general(app: &mut App, ui: &mut egui::Ui) {
    begin_card(ui, "Appearance");
    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Color theme").color(BODY_TEXT).size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for theme in Theme::ALL.iter().rev() {
                    ui.selectable_value(&mut app.draft.theme, *theme, theme.label());
                }
            });
        });
        ui.add_space(4.0);
        slider_row(ui, "Font size", &mut app.draft.font_size, 10.0..=32.0, " px");
        ui.add_space(2.0);
        slider_row(ui, "Overlay scale", &mut app.draft.overlay_scale, 0.6..=2.0, "x");
        ui.add_space(2.0);
        slider_row(ui, "Content opacity", &mut app.draft.overlay_opacity, 0.1..=1.0, "");
        ui.add_space(2.0);
        slider_row(ui, "Background opacity", &mut app.draft.background_opacity, 0.0..=1.0, "");
        ui.add_space(2.0);
        slider_row(ui, "Pill rounding", &mut app.draft.pill_rounding, 0.0..=20.0, " px");
    });

    if app.draft.theme == Theme::Custom {
        ui.add_space(12.0);
        begin_card(ui, "Custom Colors");
        card_frame(ui, |ui| {
            color_row(ui, "Key background", &mut app.draft.custom_key_bg);
            color_row(ui, "Key text", &mut app.draft.custom_key_fg);
            color_row(ui, "Modifier background", &mut app.draft.custom_mod_bg);
            color_row(ui, "Modifier text", &mut app.draft.custom_mod_fg);
            color_row(ui, "Mouse click background", &mut app.draft.custom_mouse_bg);
            color_row(ui, "Mouse click text", &mut app.draft.custom_mouse_fg);
            color_row(ui, "Scroll background", &mut app.draft.custom_scroll_bg);
            color_row(ui, "Scroll text", &mut app.draft.custom_scroll_fg);
        });
    }

    ui.add_space(16.0);

    // Save / Reset buttons
    ui.horizontal(|ui| {
        let save_id = egui::Id::new("cmd::save_apply");
        let save_btn = egui::Button::new(
            egui::RichText::new("Save & Apply").color(egui::Color32::WHITE).size(13.0),
        )
        .fill(ACCENT)
        .rounding(egui::Rounding::same(8.0))
        .min_size(egui::vec2(110.0, 32.0));
        let save_resp = ui.push_id(save_id, |ui| ui.add(save_btn)).inner;
        if command_activated_on_press(ui, save_id, &save_resp) {
            app.apply();
        }

        ui.add_space(8.0);

        let reset_id = egui::Id::new("cmd::reset_defaults");
        let reset_btn = egui::Button::new(
            egui::RichText::new("Reset Defaults").color(BODY_TEXT).size(13.0),
        )
        .fill(CARD_BG)
        .stroke(egui::Stroke::new(1.0, CARD_BORDER))
        .rounding(egui::Rounding::same(8.0))
        .min_size(egui::vec2(110.0, 32.0));
        let reset_resp = ui.push_id(reset_id, |ui| ui.add(reset_btn)).inner;
        if command_activated_on_press(ui, reset_id, &reset_resp) {
            app.reset_defaults();
        }
    });
}

// ── Tab: Keystroke ──────────────────────────────────────────────────────

fn tab_keystroke(app: &mut App, ui: &mut egui::Ui) {
    begin_card(ui, "Keystroke Display");
    card_frame(ui, |ui| {
        let mut apply = false;
        apply |= toggle_row(ui, "toggle_show_keyboard", "Show keyboard strokes", &mut app.draft.show_keyboard);
        ui.add_space(4.0);
        apply |= toggle_row(ui, "toggle_show_mouse_clicks", "Show mouse clicks", &mut app.draft.show_mouse_clicks);
        ui.add_space(4.0);
        apply |= toggle_row(ui, "toggle_show_mouse_text", "Show mouse click text", &mut app.draft.show_mouse_event_text);
        ui.add_space(4.0);
        apply |= toggle_row(ui, "toggle_show_mouse_icon", "Show mouse icon in overlay", &mut app.draft.show_mouse_icon);
        ui.add_space(4.0);
        apply |= toggle_row(ui, "toggle_show_scroll", "Show scroll events", &mut app.draft.show_scroll);
        if apply {
            app.apply();
        }
    });

    ui.add_space(12.0);
    begin_card(ui, "Timing");
    card_frame(ui, |ui| {
        let mut dirty = false;
        dirty |= slider_row(ui, "Display duration", &mut app.draft.display_duration_secs, 0.5..=10.0, " s");
        ui.add_space(2.0);
        dirty |= slider_row(ui, "Fade duration", &mut app.draft.fade_duration_secs, 0.1..=3.0, " s");
        ui.add_space(2.0);
        let mut max_vis = app.draft.max_visible_events as i32;
        dirty |= slider_row_i32(ui, "Max visible events", &mut max_vis, 1..=20, "");
        app.draft.max_visible_events = max_vis as usize;
        if dirty {
            app.apply();
        }
    });
}

// ── Tab: Cursor ─────────────────────────────────────────────────────────

fn tab_cursor(app: &mut App, ui: &mut egui::Ui) {
    begin_card(ui, "Cursor Highlight");
    card_frame(ui, |ui| {
        if toggle_row(ui, "toggle_cursor_ring", "Enable cursor highlight", &mut app.draft.enable_green_cursor_ring) {
            app.apply();
        }
    });

    if app.draft.enable_green_cursor_ring {
        ui.add_space(12.0);
        begin_card(ui, "Theme");
        card_frame(ui, |ui| {
            // Theme selector grid — circular color previews
            ui.horizontal_wrapped(|ui| {
                for theme in CursorTheme::ALL {
                    let is_selected = app.draft.cursor_theme == Some(theme);
                    let color = theme.color();
                    let c = egui::Color32::from_rgb(color.r, color.g, color.b);

                    let desired_size = egui::vec2(48.0, 48.0);
                    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

                    // Outer ring for selection
                    if is_selected {
                        ui.painter().circle_stroke(
                            rect.center(),
                            22.0,
                            egui::Stroke::new(2.5, ACCENT),
                        );
                    }

                    // Color circle
                    ui.painter().circle_filled(rect.center(), 16.0, c);

                    // Label below
                    ui.painter().text(
                        egui::pos2(rect.center().x, rect.max.y + 4.0),
                        egui::Align2::CENTER_TOP,
                        theme.label(),
                        egui::FontId::proportional(9.0),
                        MUTED_TEXT,
                    );

                    if response.clicked() {
                        app.draft.cursor_theme = Some(theme);
                        app.apply();
                    }

                    // Hover highlight
                    if response.hovered() && !is_selected {
                        ui.painter().circle_stroke(
                            rect.center(),
                            22.0,
                            egui::Stroke::new(1.5, SIDEBAR_TEXT),
                        );
                    }
                }
            });
            ui.add_space(12.0);
        });

        ui.add_space(12.0);
        begin_card(ui, "Settings");
        card_frame(ui, |ui| {
            let mut dirty = false;
            dirty |= slider_row(ui, "Size", &mut app.draft.cursor_ring_size_px, 24.0..=120.0, " px");
            ui.add_space(2.0);
            dirty |= slider_row(ui, "Thickness", &mut app.draft.cursor_ring_thickness_px, 2.0..=12.0, " px");
            ui.add_space(2.0);
            dirty |= slider_row(ui, "Opacity", &mut app.draft.cursor_ring_opacity, 0.2..=1.0, "");
            ui.add_space(2.0);
            dirty |= slider_row(ui, "Glow", &mut app.draft.cursor_glow, 0.0..=1.0, "");
            ui.add_space(4.0);

            let mut hide_ms = app.draft.cursor_hide_after_ms as i32;
            dirty |= slider_row_i32(ui, "Hide cursor after", &mut hide_ms, 0..=10000, " ms");
            app.draft.cursor_hide_after_ms = hide_ms as u32;

            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("0 = never hide")
                        .color(MUTED_TEXT)
                        .size(10.0),
                );
            });
            ui.add_space(4.0);
            dirty |= toggle_row(ui, "toggle_click_anim", "Enable click animation", &mut app.draft.enable_click_animation);

            if dirty {
                app.apply();
            }
        });
    }
}

// ── Tab: Sounds ─────────────────────────────────────────────────────────

fn tab_sounds(app: &mut App, ui: &mut egui::Ui) {
    begin_card(ui, "Keystroke Sounds");
    card_frame(ui, |ui| {
        let mut dirty = false;
        dirty |= toggle_row(
            ui,
            "toggle_sounds",
            "Enable keystroke sounds",
            &mut app.draft.enable_keystroke_sounds,
        );
        if dirty {
            app.apply();
        }
    });

    if app.draft.enable_keystroke_sounds {
        ui.add_space(12.0);
        begin_card(ui, "Volume");
        card_frame(ui, |ui| {
            if slider_row(ui, "Volume", &mut app.draft.sound_volume, 0.0..=1.0, "") {
                app.apply();
            }
        });

        ui.add_space(12.0);
        begin_card(ui, "Sound Pack");
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Sound preset").color(BODY_TEXT).size(13.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_source("sound_preset_combo")
                        .selected_text(&app.draft.sound_preset)
                        .show_ui(ui, |ui| {
                            for preset in SOUND_PRESETS {
                                let selected = app.draft.sound_preset == *preset;
                                if ui.selectable_label(selected, *preset).clicked() {
                                    app.draft.sound_preset = preset.to_string();
                                    app.apply();
                                }
                            }
                        });
                });
            });
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Sound playback is a placeholder — audio engine not yet integrated.")
                    .color(MUTED_TEXT)
                    .size(11.0),
            );
        });
    }
}

// ── Tab: Position ───────────────────────────────────────────────────────

fn tab_position(app: &mut App, ui: &mut egui::Ui) {
    let mut resolution_changed = false;
    let mut scale_changed = false;
    let mut position_dirty = false;

    begin_card(ui, "Display Baseline");
    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Display resolution").color(BODY_TEXT).size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let selected_resolution = DISPLAY_PRESETS
                    .iter()
                    .find(|(w, h, _)| {
                        (app.draft.display_width_px - *w).abs() < f32::EPSILON
                            && (app.draft.display_height_px - *h).abs() < f32::EPSILON
                    })
                    .map(|(_, _, label)| *label)
                    .unwrap_or("Custom");

                egui::ComboBox::from_id_source("display_resolution_combo")
                    .selected_text(selected_resolution)
                    .show_ui(ui, |ui| {
                        for (w, h, label) in DISPLAY_PRESETS {
                            let selected = (app.draft.display_width_px - *w).abs() < f32::EPSILON
                                && (app.draft.display_height_px - *h).abs() < f32::EPSILON;
                            if ui.selectable_label(selected, *label).clicked() {
                                app.draft.display_width_px = *w;
                                app.draft.display_height_px = *h;
                                resolution_changed = true;
                                position_dirty = true;
                            }
                        }
                    });
            });
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Display scale").color(BODY_TEXT).size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let selected_scale = SCALE_PRESETS
                    .iter()
                    .find(|(scale, _)| (app.draft.display_scale - *scale).abs() < f32::EPSILON)
                    .map(|(_, label)| *label)
                    .unwrap_or("Custom");

                egui::ComboBox::from_id_source("display_scale_combo")
                    .selected_text(selected_scale)
                    .show_ui(ui, |ui| {
                        for (scale, label) in SCALE_PRESETS {
                            let selected =
                                (app.draft.display_scale - *scale).abs() < f32::EPSILON;
                            if ui.selectable_label(selected, *label).clicked() {
                                app.draft.display_scale = *scale;
                                scale_changed = true;
                                position_dirty = true;
                            }
                        }
                    });
            });
        });
    });

    if (resolution_changed || scale_changed) && app.draft.position == OverlayPosition::Manual {
        app.sync_manual_to_lower_right();
        app.preview_manual_position();
    }

    ui.add_space(12.0);
    begin_card(ui, "Screen Position");
    card_frame(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Overlay position").color(BODY_TEXT).size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut selected_position = app.draft.position;
                egui::ComboBox::from_id_source("position_combo")
                    .selected_text(selected_position.label())
                    .show_ui(ui, |ui: &mut egui::Ui| {
                        for pos in OverlayPosition::ALL {
                            ui.selectable_value(&mut selected_position, pos, pos.label());
                        }
                    });

                if selected_position != app.draft.position {
                    app.draft.position = selected_position;
                    position_dirty = true;
                    if app.draft.position == OverlayPosition::Manual {
                        app.sync_manual_to_lower_right();
                        app.preview_manual_position();
                    }
                }
            });
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Layout direction").color(BODY_TEXT).size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for layout in OverlayLayout::ALL.iter().rev() {
                    let response =
                        ui.selectable_value(&mut app.draft.layout, *layout, layout.label());
                    position_dirty |= response.changed();
                }
            });
        });
    });

    ui.add_space(12.0);
    begin_card(ui, "Manual Position (X / Y)");
    card_frame(ui, |ui| {
        let [display_w, display_h] = app.draft.scaled_display_size_points();
        let mut manual_slider_changed = false;
        let mut dragging_now = false;

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("X position").color(BODY_TEXT).size(13.0));
            let response =
                ui.add(egui::Slider::new(&mut app.draft.overlay_x, 0.0..=display_w).suffix(" px"));
            manual_slider_changed |= response.changed();
            dragging_now |= response.dragged();
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Y position").color(BODY_TEXT).size(13.0));
            let response =
                ui.add(egui::Slider::new(&mut app.draft.overlay_y, 0.0..=display_h).suffix(" px"));
            manual_slider_changed |= response.changed();
            dragging_now |= response.dragged();
        });

        app.manual_slider_drag_ended = app.manual_slider_dragging && !dragging_now;
        app.manual_slider_dragging = dragging_now;

        if manual_slider_changed {
            if app.draft.position != OverlayPosition::Manual {
                app.draft.position = OverlayPosition::Manual;
            }
            app.preview_manual_position();
            position_dirty = true;
            if !app.manual_slider_dragging {
                app.manual_slider_drag_ended = true;
            }
        }

        if app.draft.position != OverlayPosition::Manual {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Select \"Manual (X/Y)\" above to use these sliders.")
                    .size(11.0)
                    .color(MUTED_TEXT),
            );
        } else {
            ui.add_space(4.0);
            let snap_id = egui::Id::new("cmd::snap_manual_position");
            let snap_btn = egui::Button::new(
                egui::RichText::new("Snap to lower-right").color(BODY_TEXT).size(12.0),
            )
            .fill(CARD_BG)
            .stroke(egui::Stroke::new(1.0, CARD_BORDER))
            .rounding(egui::Rounding::same(8.0));
            let snap_resp = ui.push_id(snap_id, |ui| ui.add(snap_btn)).inner;
            if command_activated_on_press(ui, snap_id, &snap_resp) {
                app.sync_manual_to_lower_right();
                app.preview_manual_position();
                position_dirty = true;
            }
        }
    });

    ui.add_space(12.0);
    begin_card(ui, "Margins & Size");
    card_frame(ui, |ui| {
        position_dirty |= slider_row(ui, "Horizontal margin", &mut app.draft.margin_x, 0.0..=200.0, " px");
        ui.add_space(2.0);
        position_dirty |= slider_row(ui, "Vertical margin", &mut app.draft.margin_y, 0.0..=200.0, " px");
        ui.add_space(6.0);
        position_dirty |= slider_row(ui, "Overlay width", &mut app.draft.overlay_width, 100.0..=800.0, " px");
        ui.add_space(2.0);
        position_dirty |= slider_row(ui, "Overlay height", &mut app.draft.overlay_height, 60.0..=600.0, " px");
    });

    if position_dirty {
        app.apply();
    }
}

// ── Tab: License ────────────────────────────────────────────────────────

fn tab_license(ui: &mut egui::Ui) {
    begin_card(ui, "License");
    card_frame(ui, |ui| {
        ui.label(
            egui::RichText::new("MIT License")
                .color(HEADING_TEXT)
                .size(16.0)
                .strong(),
        );
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(
                "Copyright (c) 2024 KeyOverlay Contributors\n\n\
                 Permission is hereby granted, free of charge, to any person obtaining a copy \
                 of this software and associated documentation files (the \"Software\"), to deal \
                 in the Software without restriction, including without limitation the rights \
                 to use, copy, modify, merge, publish, distribute, sublicense, and/or sell \
                 copies of the Software, and to permit persons to whom the Software is \
                 furnished to do so, subject to the following conditions:\n\n\
                 The above copyright notice and this permission notice shall be included in all \
                 copies or substantial portions of the Software.\n\n\
                 THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR \
                 IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, \
                 FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT."
            )
            .color(BODY_TEXT)
            .size(11.0),
        );
    });
}

// ── Tab: About ──────────────────────────────────────────────────────────

fn tab_about(ui: &mut egui::Ui) {
    begin_card(ui, "KeyOverlay");
    card_frame(ui, |ui| {
        ui.label(
            egui::RichText::new("A lightweight keystroke & mouse overlay for presentations.")
                .color(BODY_TEXT)
                .size(14.0),
        );
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Version 0.2.0").color(MUTED_TEXT).size(13.0));
    });

    ui.add_space(12.0);
    begin_card(ui, "Features");
    card_frame(ui, |ui| {
        let features = [
            "Real-time keyboard & mouse capture",
            "Transparent floating overlay",
            "Mouse icon with click highlighting",
            "Color-coded modifier + key pills",
            "Configurable themes (Dark / Light / Custom)",
            "Adjustable position, size & margins",
            "Cursor highlight ring with multiple themes",
            "Fade-out animations",
            "Cross-platform (Linux, macOS, Windows)",
        ];
        for feat in features {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("\u{2022}")
                        .size(13.0)
                        .color(ACCENT),
                );
                ui.label(egui::RichText::new(feat).size(13.0).color(BODY_TEXT));
            });
        }
    });
}
