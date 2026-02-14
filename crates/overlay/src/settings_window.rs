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

// ── Draw settings in the main window ────────────────────────────────────

pub fn draw_settings(ctx: &egui::Context, app: &mut App) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(30, 30, 40);
    ctx.set_visuals(visuals);

    egui::CentralPanel::default()
        .frame(
            egui::Frame::central_panel(&ctx.style())
                .fill(egui::Color32::from_rgb(30, 30, 40))
                .inner_margin(egui::Margin::same(20.0)),
        )
        .show(ctx, |ui| {
            // ── Header with ON/OFF toggle ──
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("KeyOverlay")
                        .color(egui::Color32::from_rgb(220, 220, 240))
                        .size(20.0),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // ON / OFF toggle button.
                    let enabled = app.draft.overlay_enabled;
                    let (btn_text, btn_color, btn_bg) = if enabled {
                        (
                            "ON",
                            egui::Color32::WHITE,
                            egui::Color32::from_rgb(50, 160, 80),
                        )
                    } else {
                        (
                            "OFF",
                            egui::Color32::from_rgb(200, 200, 210),
                            egui::Color32::from_rgb(80, 80, 100),
                        )
                    };

                    let btn = egui::Button::new(
                        egui::RichText::new(btn_text)
                            .color(btn_color)
                            .size(14.0)
                            .strong(),
                    )
                    .fill(btn_bg)
                    .rounding(egui::Rounding::same(12.0))
                    .min_size(egui::vec2(56.0, 28.0));

                    if ui.add(btn).clicked() {
                        app.draft.overlay_enabled = !app.draft.overlay_enabled;
                        app.apply();
                    }

                    // Status dot + label.
                    let status_color = if enabled {
                        egui::Color32::from_rgb(100, 220, 100)
                    } else {
                        egui::Color32::from_rgb(120, 120, 140)
                    };
                    let status_text = if enabled {
                        "Overlay active"
                    } else {
                        "Overlay off"
                    };

                    ui.label(
                        egui::RichText::new(status_text)
                            .color(status_color)
                            .size(12.0),
                    );

                    let (dot_rect, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    if enabled {
                        let pulse = ((ctx.input(|i| i.time) * 2.0).sin() * 0.5 + 0.5) as f32;
                        let a = (150.0 + 105.0 * pulse) as u8;
                        ui.painter().circle_filled(
                            dot_rect.center(),
                            4.0,
                            egui::Color32::from_rgba_unmultiplied(100, 220, 100, a),
                        );
                    } else {
                        ui.painter().circle_filled(
                            dot_rect.center(),
                            4.0,
                            egui::Color32::from_rgb(80, 80, 100),
                        );
                    }
                });
            });

            ui.add_space(8.0);

            // ── Tab bar ──
            ui.horizontal(|ui| {
                for tab in SettingsTab::ALL {
                    let selected = app.active_tab == tab;
                    let text = egui::RichText::new(tab.label()).size(14.0);
                    let text = if selected {
                        text.color(egui::Color32::WHITE).strong()
                    } else {
                        text.color(egui::Color32::from_rgb(140, 140, 170))
                    };
                    if ui
                        .add(
                            egui::Button::new(text)
                                .frame(false)
                                .min_size(egui::vec2(80.0, 30.0)),
                        )
                        .clicked()
                    {
                        app.active_tab = tab;
                    }
                }
            });

            // Separator under tabs.
            ui.add_space(2.0);
            let sep_rect = ui.available_rect_before_wrap();
            let sep_rect =
                egui::Rect::from_min_size(sep_rect.min, egui::vec2(sep_rect.width(), 1.0));
            ui.painter()
                .rect_filled(sep_rect, 0.0, egui::Color32::from_rgb(60, 60, 80));
            ui.add_space(12.0);

            // ── Tab content ──
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| match app.active_tab {
                    SettingsTab::Appearance => tab_appearance(app, ui),
                    SettingsTab::Behavior => tab_behavior(app, ui),
                    SettingsTab::Position => tab_position(app, ui),
                    SettingsTab::About => tab_about(ui),
                });

            // ── Bottom bar ──
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Save & Apply").clicked() {
                    app.apply();
                }
                if ui.button("Reset Defaults").clicked() {
                    app.reset_defaults();
                }

                if let Some((msg, t)) = &app.status_msg {
                    if t.elapsed().as_secs() < 3 {
                        ui.label(
                            egui::RichText::new(msg)
                                .color(egui::Color32::from_rgb(100, 220, 100))
                                .size(12.0),
                        );
                    }
                }
            });
        });
}

// ── Tab Implementations ─────────────────────────────────────────────────

fn tab_appearance(app: &mut App, ui: &mut egui::Ui) {
    section_heading(ui, "Theme");

    ui.horizontal(|ui| {
        ui.label("Color theme:");
        for theme in Theme::ALL {
            ui.selectable_value(&mut app.draft.theme, theme, theme.label());
        }
    });

    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label("Font size:");
        ui.add(egui::Slider::new(&mut app.draft.font_size, 10.0..=32.0).suffix(" px"));
    });

    ui.horizontal(|ui| {
        ui.label("Overlay opacity:");
        ui.add(egui::Slider::new(&mut app.draft.overlay_opacity, 0.1..=1.0));
    });

    ui.horizontal(|ui| {
        ui.label("Pill rounding:");
        ui.add(egui::Slider::new(&mut app.draft.pill_rounding, 0.0..=20.0).suffix(" px"));
    });

    if app.draft.theme == Theme::Custom {
        ui.add_space(12.0);
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
    }
}

fn tab_behavior(app: &mut App, ui: &mut egui::Ui) {
    section_heading(ui, "Display");

    ui.horizontal(|ui| {
        ui.label("Display duration:");
        ui.add(egui::Slider::new(&mut app.draft.display_duration_secs, 0.5..=10.0).suffix(" s"));
    });

    ui.horizontal(|ui| {
        ui.label("Fade duration:");
        ui.add(egui::Slider::new(&mut app.draft.fade_duration_secs, 0.1..=3.0).suffix(" s"));
    });

    ui.horizontal(|ui| {
        ui.label("Max visible events:");
        let mut val = app.draft.max_visible_events as i32;
        ui.add(egui::Slider::new(&mut val, 1..=20));
        app.draft.max_visible_events = val as usize;
    });

    ui.add_space(12.0);
    section_heading(ui, "Input Filters");

    ui.checkbox(&mut app.draft.show_keyboard, "Show keyboard strokes");
    ui.checkbox(&mut app.draft.show_mouse_clicks, "Show mouse clicks");
    ui.checkbox(&mut app.draft.show_mouse_icon, "Show mouse icon in overlay");
    ui.checkbox(&mut app.draft.show_scroll, "Show scroll events");
}

fn tab_position(app: &mut App, ui: &mut egui::Ui) {
    section_heading(ui, "Screen Position");

    ui.horizontal(|ui| {
        ui.label("Overlay position:");
        egui::ComboBox::from_id_source("position_combo")
            .selected_text(app.draft.position.label())
            .show_ui(ui, |ui: &mut egui::Ui| {
                for pos in OverlayPosition::ALL {
                    ui.selectable_value(&mut app.draft.position, pos, pos.label());
                }
            });
    });

    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label("Layout direction:");
        for layout in OverlayLayout::ALL {
            ui.selectable_value(&mut app.draft.layout, layout, layout.label());
        }
    });

    ui.add_space(12.0);
    section_heading(ui, "Margins");

    ui.horizontal(|ui| {
        ui.label("Horizontal margin:");
        ui.add(egui::Slider::new(&mut app.draft.margin_x, 0.0..=200.0).suffix(" px"));
    });

    ui.horizontal(|ui| {
        ui.label("Vertical margin:");
        ui.add(egui::Slider::new(&mut app.draft.margin_y, 0.0..=200.0).suffix(" px"));
    });

    ui.add_space(12.0);
    section_heading(ui, "Overlay Size");

    ui.horizontal(|ui| {
        ui.label("Width:");
        ui.add(egui::Slider::new(&mut app.draft.overlay_width, 200.0..=800.0).suffix(" px"));
    });

    ui.horizontal(|ui| {
        ui.label("Height:");
        ui.add(egui::Slider::new(&mut app.draft.overlay_height, 100.0..=600.0).suffix(" px"));
    });
}

fn tab_about(ui: &mut egui::Ui) {
    section_heading(ui, "KeyOverlay");

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("A lightweight keystroke & mouse overlay for presentations.")
            .size(14.0),
    );
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Version 0.2.0").size(13.0));
    ui.label(egui::RichText::new("License: MIT").size(13.0));
    ui.add_space(16.0);

    ui.label(
        egui::RichText::new("Features")
            .size(15.0)
            .strong()
            .color(egui::Color32::from_rgb(180, 180, 210)),
    );
    ui.add_space(4.0);
    let features = [
        "Real-time keyboard & mouse capture",
        "Transparent floating overlay",
        "Mouse icon with click highlighting",
        "Color-coded modifier + key pills",
        "Configurable themes (Dark / Light / Custom)",
        "Adjustable position, size & margins",
        "Fade-out animations",
        "Cross-platform (Linux, macOS, Windows)",
    ];
    for feat in features {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("\u{2022}")
                    .size(13.0)
                    .color(egui::Color32::from_rgb(100, 160, 255)),
            );
            ui.label(egui::RichText::new(feat).size(13.0));
        });
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn section_heading(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(16.0)
            .strong()
            .color(egui::Color32::from_rgb(180, 180, 210)),
    );
    ui.add_space(4.0);
}

fn color_row(ui: &mut egui::Ui, label: &str, color: &mut keyoverlay_core::Color) {
    ui.horizontal(|ui| {
        ui.label(label);
        let ec = c2e(*color);
        let mut rgba = [ec.r(), ec.g(), ec.b()];
        if ui.color_edit_button_srgb(&mut rgba).changed() {
            let ec = egui::Color32::from_rgb(rgba[0], rgba[1], rgba[2]);
            *color = e2c(ec);
        }
    });
}
