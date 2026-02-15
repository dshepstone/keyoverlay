use eframe::egui;
use keyoverlay_core::{OverlayLayout, OverlayPosition, Theme};

use crate::theme::{c2e, e2c};
use crate::App;

// ── Settings Tabs ────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Appearance,
    Behavior,
    Position,
    About,
}

impl SettingsTab {
    const ALL: [SettingsTab; 4] = [
        SettingsTab::Appearance,
        SettingsTab::Behavior,
        SettingsTab::Position,
        SettingsTab::About,
    ];

    fn label(self) -> &'static str {
        match self {
            SettingsTab::Appearance => "Appearance",
            SettingsTab::Behavior => "Behavior",
            SettingsTab::Position => "Position",
            SettingsTab::About => "About",
        }
    }
}

// ── Color constants for the clean white UI ────────────────────────────────

const BG: egui::Color32 = egui::Color32::from_rgb(252, 252, 255);
const CARD_BG: egui::Color32 = egui::Color32::WHITE;
const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(30, 30, 40);
const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(100, 100, 120);
const ACCENT: egui::Color32 = egui::Color32::from_rgb(100, 80, 220);
const BORDER: egui::Color32 = egui::Color32::from_rgb(225, 225, 235);
const SUCCESS: egui::Color32 = egui::Color32::from_rgb(40, 170, 80);
const TAB_ACTIVE_BG: egui::Color32 = egui::Color32::from_rgb(100, 80, 220);
const TAB_INACTIVE_BG: egui::Color32 = egui::Color32::from_rgb(240, 240, 248);

// ── Draw settings in the main window ────────────────────────────────────

pub fn draw_settings(ctx: &egui::Context, app: &mut App) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = BG;
    visuals.window_fill = CARD_BG;
    visuals.widgets.noninteractive.fg_stroke.color = TEXT_PRIMARY;
    visuals.widgets.inactive.fg_stroke.color = TEXT_PRIMARY;
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(240, 240, 248);
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(240, 240, 248);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(230, 230, 242);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(220, 220, 236);
    visuals.widgets.noninteractive.bg_stroke.color = BORDER;
    visuals.widgets.inactive.bg_stroke.color = BORDER;
    ctx.set_visuals(visuals);

    egui::CentralPanel::default()
        .frame(
            egui::Frame::central_panel(&ctx.style())
                .fill(BG)
                .inner_margin(egui::Margin::same(24.0)),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 8.0);

            // ── Header ──
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("KeyOverlay")
                        .color(TEXT_PRIMARY)
                        .size(22.0)
                        .strong(),
                );
                ui.label(
                    egui::RichText::new("v0.2.0")
                        .color(TEXT_SECONDARY)
                        .size(12.0),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let enabled = app.draft.overlay_enabled;
                    let (btn_text, btn_fg, btn_bg) = if enabled {
                        ("ON", egui::Color32::WHITE, SUCCESS)
                    } else {
                        (
                            "OFF",
                            TEXT_SECONDARY,
                            egui::Color32::from_rgb(210, 210, 220),
                        )
                    };

                    let btn = egui::Button::new(
                        egui::RichText::new(btn_text)
                            .color(btn_fg)
                            .size(13.0)
                            .strong(),
                    )
                    .fill(btn_bg)
                    .rounding(egui::Rounding::same(14.0))
                    .min_size(egui::vec2(54.0, 28.0));

                    if ui.add(btn).clicked() {
                        app.draft.overlay_enabled = !app.draft.overlay_enabled;
                        app.apply();
                    }

                    // Status label
                    let status_color = if enabled { SUCCESS } else { TEXT_SECONDARY };
                    let status_text = if enabled { "Active" } else { "Inactive" };
                    ui.label(
                        egui::RichText::new(status_text)
                            .color(status_color)
                            .size(12.0),
                    );

                    // Status dot
                    let (dot_rect, _) =
                        ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    let dot_color = if enabled {
                        SUCCESS
                    } else {
                        egui::Color32::from_rgb(180, 180, 195)
                    };
                    ui.painter()
                        .circle_filled(dot_rect.center(), 4.0, dot_color);
                });
            });

            ui.add_space(16.0);

            // ── Tab bar ──
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                for tab in SettingsTab::ALL {
                    let selected = app.active_tab == tab;
                    let (text_color, bg_color) = if selected {
                        (egui::Color32::WHITE, TAB_ACTIVE_BG)
                    } else {
                        (TEXT_SECONDARY, TAB_INACTIVE_BG)
                    };

                    let btn = egui::Button::new(
                        egui::RichText::new(tab.label())
                            .size(13.0)
                            .color(text_color),
                    )
                    .fill(bg_color)
                    .rounding(egui::Rounding::same(8.0))
                    .min_size(egui::vec2(84.0, 32.0));

                    if ui.add(btn).clicked() {
                        app.active_tab = tab;
                    }
                }
            });

            ui.add_space(12.0);

            // ── Separator ──
            let sep_rect = ui.available_rect_before_wrap();
            let sep_rect =
                egui::Rect::from_min_size(sep_rect.min, egui::vec2(sep_rect.width(), 1.0));
            ui.painter().rect_filled(sep_rect, 0.0, BORDER);
            ui.add_space(16.0);

            // ── Tab content (inside a card-like area) ──
            egui::Frame::none()
                .fill(CARD_BG)
                .rounding(egui::Rounding::same(12.0))
                .stroke(egui::Stroke::new(1.0, BORDER))
                .inner_margin(egui::Margin::same(20.0))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| match app.active_tab {
                            SettingsTab::Appearance => tab_appearance(app, ui),
                            SettingsTab::Behavior => tab_behavior(app, ui),
                            SettingsTab::Position => tab_position(app, ui),
                            SettingsTab::About => tab_about(ui),
                        });
                });

            // ── Bottom bar ──
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                let save_btn = egui::Button::new(
                    egui::RichText::new("Save & Apply")
                        .color(egui::Color32::WHITE)
                        .size(13.0)
                        .strong(),
                )
                .fill(ACCENT)
                .rounding(egui::Rounding::same(8.0))
                .min_size(egui::vec2(110.0, 34.0));

                if ui.add(save_btn).clicked() {
                    app.apply();
                }

                let reset_btn = egui::Button::new(
                    egui::RichText::new("Reset Defaults")
                        .color(TEXT_SECONDARY)
                        .size(13.0),
                )
                .fill(egui::Color32::from_rgb(240, 240, 248))
                .stroke(egui::Stroke::new(1.0, BORDER))
                .rounding(egui::Rounding::same(8.0))
                .min_size(egui::vec2(110.0, 34.0));

                if ui.add(reset_btn).clicked() {
                    app.reset_defaults();
                }

                if let Some((msg, t)) = &app.status_msg {
                    if t.elapsed().as_secs() < 3 {
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(msg).color(SUCCESS).size(12.0));
                    }
                }
            });
        });
}

// ── Tab Implementations ─────────────────────────────────────────────────

fn tab_appearance(app: &mut App, ui: &mut egui::Ui) {
    section_heading(ui, "Theme");

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Color theme:")
                .color(TEXT_PRIMARY)
                .size(13.0),
        );
        ui.add_space(4.0);
        for theme in Theme::ALL {
            let selected = app.draft.theme == theme;
            let (text_color, bg) = if selected {
                (egui::Color32::WHITE, ACCENT)
            } else {
                (TEXT_PRIMARY, egui::Color32::from_rgb(240, 240, 248))
            };
            let btn = egui::Button::new(
                egui::RichText::new(theme.label())
                    .color(text_color)
                    .size(12.0),
            )
            .fill(bg)
            .rounding(egui::Rounding::same(6.0))
            .min_size(egui::vec2(0.0, 26.0));

            if ui.add(btn).clicked() {
                app.draft.theme = theme;
            }
        }
    });

    ui.add_space(12.0);

    setting_row(ui, "Font size", |ui| {
        ui.add(
            egui::Slider::new(&mut app.draft.font_size, 10.0..=32.0)
                .suffix(" px")
                .min_decimals(0)
                .max_decimals(0),
        );
    });

    setting_row(ui, "Content opacity", |ui| {
        ui.add(egui::Slider::new(&mut app.draft.overlay_opacity, 0.1..=1.0));
    });

    setting_row(ui, "Background opacity", |ui| {
        ui.add(egui::Slider::new(
            &mut app.draft.background_opacity,
            0.0..=1.0,
        ));
    });

    setting_row(ui, "Pill rounding", |ui| {
        ui.add(
            egui::Slider::new(&mut app.draft.pill_rounding, 0.0..=20.0)
                .suffix(" px")
                .min_decimals(0)
                .max_decimals(0),
        );
    });

    if app.draft.theme == Theme::Custom {
        ui.add_space(16.0);
        section_heading(ui, "Custom Colors");

        color_row(ui, "Key background:", &mut app.draft.custom_key_bg);
        color_row(ui, "Key text:", &mut app.draft.custom_key_fg);
        color_row(ui, "Modifier background:", &mut app.draft.custom_mod_bg);
        color_row(ui, "Modifier text:", &mut app.draft.custom_mod_fg);
        color_row(
            ui,
            "Mouse click background:",
            &mut app.draft.custom_mouse_bg,
        );
        color_row(ui, "Mouse click text:", &mut app.draft.custom_mouse_fg);
        color_row(ui, "Scroll background:", &mut app.draft.custom_scroll_bg);
        color_row(ui, "Scroll text:", &mut app.draft.custom_scroll_fg);
        color_row(ui, "Tray background:", &mut app.draft.custom_tray_bg);
        color_row(ui, "Tray border:", &mut app.draft.custom_tray_border);
    }
}

fn tab_behavior(app: &mut App, ui: &mut egui::Ui) {
    section_heading(ui, "Display");

    setting_row(ui, "Display duration", |ui| {
        ui.add(egui::Slider::new(&mut app.draft.display_duration_secs, 0.5..=10.0).suffix(" s"));
    });

    setting_row(ui, "Fade duration", |ui| {
        ui.add(egui::Slider::new(&mut app.draft.fade_duration_secs, 0.1..=3.0).suffix(" s"));
    });

    setting_row(ui, "Max visible events", |ui| {
        let mut val = app.draft.max_visible_events as i32;
        ui.add(egui::Slider::new(&mut val, 1..=20));
        app.draft.max_visible_events = val as usize;
    });

    ui.add_space(16.0);
    section_heading(ui, "Input Filters");

    ui.add_space(4.0);
    ui.checkbox(&mut app.draft.show_keyboard, "Show keyboard strokes");
    ui.checkbox(&mut app.draft.show_mouse_clicks, "Show mouse clicks");
    ui.checkbox(&mut app.draft.show_mouse_icon, "Show mouse icon in overlay");
    ui.checkbox(&mut app.draft.show_scroll, "Show scroll events");

    ui.add_space(8.0);
    ui.checkbox(
        &mut app.draft.positioning_mode,
        "Keep overlay visible for positioning (locks when input is detected)",
    );
}

fn tab_position(app: &mut App, ui: &mut egui::Ui) {
    section_heading(ui, "Screen Position");

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Overlay position:")
                .color(TEXT_PRIMARY)
                .size(13.0),
        );
        egui::ComboBox::from_id_source("position_combo")
            .selected_text(app.draft.position.label())
            .show_ui(ui, |ui: &mut egui::Ui| {
                for pos in OverlayPosition::ALL {
                    ui.selectable_value(&mut app.draft.position, pos, pos.label());
                }
            });
    });

    ui.add_space(12.0);
    section_heading(ui, "Position (X / Y)");

    let max_x = app.screen_size[0].max(1.0);
    let max_y = app.screen_size[1].max(1.0);

    setting_row(ui, "X position", |ui| {
        ui.add(
            egui::Slider::new(&mut app.draft.overlay_x, 0.0..=max_x)
                .suffix(" px")
                .min_decimals(0)
                .max_decimals(0),
        );
    });

    setting_row(ui, "Y position", |ui| {
        ui.add(
            egui::Slider::new(&mut app.draft.overlay_y, 0.0..=max_y)
                .suffix(" px")
                .min_decimals(0)
                .max_decimals(0),
        );
    });

    ui.label(
        egui::RichText::new(format!(
            "Current screen: {:.0} x {:.0} px",
            app.screen_size[0], app.screen_size[1]
        ))
        .size(11.0)
        .color(TEXT_SECONDARY),
    );

    if app.draft.position != OverlayPosition::Manual {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Tip: select \"Manual (X/Y)\" above to use these sliders for positioning.",
            )
            .size(11.0)
            .color(TEXT_SECONDARY),
        );
    }

    ui.add_space(16.0);

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Layout direction:")
                .color(TEXT_PRIMARY)
                .size(13.0),
        );
        ui.add_space(4.0);
        for layout in OverlayLayout::ALL {
            let selected = app.draft.layout == layout;
            let (text_color, bg) = if selected {
                (egui::Color32::WHITE, ACCENT)
            } else {
                (TEXT_PRIMARY, egui::Color32::from_rgb(240, 240, 248))
            };
            let btn = egui::Button::new(
                egui::RichText::new(layout.label())
                    .color(text_color)
                    .size(12.0),
            )
            .fill(bg)
            .rounding(egui::Rounding::same(6.0))
            .min_size(egui::vec2(0.0, 26.0));

            if ui.add(btn).clicked() {
                app.draft.layout = layout;
            }
        }
    });

    ui.add_space(16.0);
    section_heading(ui, "Margins");

    setting_row(ui, "Horizontal margin", |ui| {
        ui.add(
            egui::Slider::new(&mut app.draft.margin_x, 0.0..=200.0)
                .suffix(" px")
                .min_decimals(0)
                .max_decimals(0),
        );
    });

    setting_row(ui, "Vertical margin", |ui| {
        ui.add(
            egui::Slider::new(&mut app.draft.margin_y, 0.0..=200.0)
                .suffix(" px")
                .min_decimals(0)
                .max_decimals(0),
        );
    });

    ui.add_space(16.0);
    section_heading(ui, "Overlay Size");

    setting_row(ui, "Width", |ui| {
        ui.add(
            egui::Slider::new(&mut app.draft.overlay_width, 100.0..=800.0)
                .suffix(" px")
                .min_decimals(0)
                .max_decimals(0),
        );
    });

    setting_row(ui, "Height", |ui| {
        ui.add(
            egui::Slider::new(&mut app.draft.overlay_height, 60.0..=600.0)
                .suffix(" px")
                .min_decimals(0)
                .max_decimals(0),
        );
    });
}

fn tab_about(ui: &mut egui::Ui) {
    section_heading(ui, "KeyOverlay");

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("A lightweight keystroke & mouse overlay for presentations.")
            .size(14.0)
            .color(TEXT_PRIMARY),
    );
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Version 0.2.0")
            .size(13.0)
            .color(TEXT_SECONDARY),
    );
    ui.label(
        egui::RichText::new("License: MIT")
            .size(13.0)
            .color(TEXT_SECONDARY),
    );
    ui.add_space(20.0);

    section_heading(ui, "Features");
    ui.add_space(4.0);
    let features = [
        "Real-time keyboard & mouse capture",
        "Transparent floating overlay",
        "Mouse icon with click highlighting",
        "Color-coded modifier + key pills",
        "Themes: Dark, Light, Minimal, High Contrast, Custom",
        "Adjustable position, size & margins",
        "No activation animation (instant appear)",
        "Cross-platform (Linux, macOS, Windows)",
    ];
    for feat in features {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("\u{2022}").size(13.0).color(ACCENT));
            ui.label(egui::RichText::new(feat).size(13.0).color(TEXT_PRIMARY));
        });
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn section_heading(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(15.0)
            .strong()
            .color(TEXT_PRIMARY),
    );
    ui.add_space(4.0);
}

fn setting_row(ui: &mut egui::Ui, label: &str, add_widget: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{}:", label))
                .color(TEXT_PRIMARY)
                .size(13.0),
        );
        add_widget(ui);
    });
}

fn color_row(ui: &mut egui::Ui, label: &str, color: &mut keyoverlay_core::Color) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(TEXT_PRIMARY).size(13.0));
        let ec = c2e(*color);
        let mut rgba = [ec.r(), ec.g(), ec.b()];
        if ui.color_edit_button_srgb(&mut rgba).changed() {
            let ec = egui::Color32::from_rgb(rgba[0], rgba[1], rgba[2]);
            *color = e2c(ec);
        }
    });
}
