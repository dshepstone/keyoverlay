use std::collections::HashSet;

use eframe::egui;
use keyoverlay_core::{OverlayLayout, OverlayPosition, Theme};

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

// ── Settings Tabs ────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

// ── Draw settings in the main window ────────────────────────────────────

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
            // Keyboard/accessibility activation path.
            return true;
        }
    }

    false
}

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
            reset_command_press_state_if_needed(ui);
            // ── Header with ON/OFF toggle ──
            ui.horizontal(|ui| {
                if let Some(icon_texture) = load_header_icon_texture(ctx) {
                    ui.add(egui::Image::new((
                        icon_texture.id(),
                        egui::vec2(26.0, 26.0),
                    )));
                    ui.add_space(10.0);
                }

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

                    let toggle_id = egui::Id::new("cmd::overlay_toggle");
                    let toggle_resp = ui.push_id(toggle_id, |ui| ui.add(btn)).inner;
                    if command_activated_on_press(ui, toggle_id, &toggle_resp) {
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
                    let tab_id = egui::Id::new(format!("cmd::tab::{:?}", tab));
                    let tab_resp = ui
                        .push_id(tab_id, |ui| {
                            ui.add(
                                egui::Button::new(text)
                                    .frame(false)
                                    .min_size(egui::vec2(80.0, 30.0)),
                            )
                        })
                        .inner;
                    if command_activated_on_press(ui, tab_id, &tab_resp) {
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
                let save_id = egui::Id::new("cmd::save_apply");
                let save_resp = ui.push_id(save_id, |ui| ui.button("Save & Apply")).inner;
                if command_activated_on_press(ui, save_id, &save_resp) {
                    app.apply();
                }

                let reset_id = egui::Id::new("cmd::reset_defaults");
                let reset_resp = ui.push_id(reset_id, |ui| ui.button("Reset Defaults")).inner;
                if command_activated_on_press(ui, reset_id, &reset_resp) {
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
        ui.label("Overlay scale:");
        ui.add(egui::Slider::new(&mut app.draft.overlay_scale, 0.60..=2.00).suffix("x"));
    });

    ui.horizontal(|ui| {
        ui.label("Content opacity:");
        ui.add(egui::Slider::new(&mut app.draft.overlay_opacity, 0.1..=1.0));
    });

    ui.horizontal(|ui| {
        ui.label("Background opacity:");
        ui.add(egui::Slider::new(
            &mut app.draft.background_opacity,
            0.0..=1.0,
        ));
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
    ui.horizontal(|ui| {
        section_heading(ui, "Display");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let reset_behavior_id = egui::Id::new("cmd::behavior_reset_defaults");
            let reset_behavior_resp = ui
                .push_id(reset_behavior_id, |ui| {
                    ui.button("Reset to default settings")
                })
                .inner;
            if command_activated_on_press(ui, reset_behavior_id, &reset_behavior_resp) {
                app.reset_defaults();
            }
        });
    });

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
    ui.checkbox(
        &mut app.draft.show_mouse_event_text,
        "Show mouse click text (Left/Right/Middle)",
    );
    ui.checkbox(&mut app.draft.show_mouse_icon, "Show mouse icon in overlay");
    ui.checkbox(&mut app.draft.show_scroll, "Show scroll events");

    ui.add_space(12.0);
    section_heading(ui, "Cursor Highlight");

    let ring_toggle = ui.checkbox(
        &mut app.draft.enable_green_cursor_ring,
        "Enable green cursor ring",
    );
    if ring_toggle.changed() {
        app.apply();
    }

    ui.add_enabled_ui(app.draft.enable_green_cursor_ring, |ui| {
        ui.horizontal(|ui| {
            ui.label("Ring size:");
            if ui
                .add(
                    egui::Slider::new(&mut app.draft.cursor_ring_size_px, 24.0..=120.0)
                        .suffix(" px"),
                )
                .changed()
            {
                app.apply();
            }
        });

        ui.horizontal(|ui| {
            ui.label("Ring thickness:");
            if ui
                .add(
                    egui::Slider::new(&mut app.draft.cursor_ring_thickness_px, 2.0..=12.0)
                        .suffix(" px"),
                )
                .changed()
            {
                app.apply();
            }
        });

        ui.horizontal(|ui| {
            ui.label("Ring opacity:");
            if ui
                .add(egui::Slider::new(
                    &mut app.draft.cursor_ring_opacity,
                    0.2..=1.0,
                ))
                .changed()
            {
                app.apply();
            }
        });
    });
}

fn tab_position(app: &mut App, ui: &mut egui::Ui) {
    section_heading(ui, "Display Baseline");

    let mut resolution_changed = false;
    let mut scale_changed = false;
    let mut position_dirty = false;

    ui.horizontal(|ui| {
        ui.label("Display resolution:");
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

    ui.horizontal(|ui| {
        ui.label("Display scale:");
        let selected_scale = SCALE_PRESETS
            .iter()
            .find(|(scale, _)| (app.draft.display_scale - *scale).abs() < f32::EPSILON)
            .map(|(_, label)| *label)
            .unwrap_or("Custom");

        egui::ComboBox::from_id_source("display_scale_combo")
            .selected_text(selected_scale)
            .show_ui(ui, |ui| {
                for (scale, label) in SCALE_PRESETS {
                    let selected = (app.draft.display_scale - *scale).abs() < f32::EPSILON;
                    if ui.selectable_label(selected, *label).clicked() {
                        app.draft.display_scale = *scale;
                        scale_changed = true;
                        position_dirty = true;
                    }
                }
            });
    });

    if (resolution_changed || scale_changed) && app.draft.position == OverlayPosition::Manual {
        app.sync_manual_to_lower_right();
        app.preview_manual_position();
    }

    ui.add_space(12.0);
    section_heading(ui, "Screen Position");

    ui.horizontal(|ui| {
        ui.label("Overlay position:");
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

    ui.add_space(12.0);
    section_heading(ui, "Position (X / Y)");

    let [display_w, display_h] = app.draft.scaled_display_size_points();

    let mut manual_slider_changed = false;

    let mut dragging_now = false;

    ui.horizontal(|ui| {
        ui.label("X position:");
        let response =
            ui.add(egui::Slider::new(&mut app.draft.overlay_x, 0.0..=display_w).suffix(" px"));
        manual_slider_changed |= response.changed();
        dragging_now |= response.dragged();
    });

    ui.horizontal(|ui| {
        ui.label("Y position:");
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

    // When the user touches X/Y sliders, auto-switch to Manual mode so the
    // values take effect immediately without requiring a separate dropdown change.
    if app.draft.position != OverlayPosition::Manual {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                "Tip: select \"Manual (X/Y)\" above to use these sliders for positioning.",
            )
            .size(11.0)
            .color(egui::Color32::from_rgb(140, 140, 170)),
        );
    } else {
        let snap_id = egui::Id::new("cmd::snap_manual_position");
        let snap_resp = ui
            .push_id(snap_id, |ui| {
                ui.button("Snap manual position to lower-right")
            })
            .inner;
        if command_activated_on_press(ui, snap_id, &snap_resp) {
            app.sync_manual_to_lower_right();
            app.preview_manual_position();
            position_dirty = true;
        }
    }

    ui.add_space(12.0);

    ui.horizontal(|ui| {
        ui.label("Layout direction:");
        for layout in OverlayLayout::ALL {
            let response = ui.selectable_value(&mut app.draft.layout, layout, layout.label());
            position_dirty |= response.changed();
        }
    });

    ui.add_space(12.0);
    section_heading(ui, "Margins");

    ui.horizontal(|ui| {
        ui.label("Horizontal margin:");
        let response =
            ui.add(egui::Slider::new(&mut app.draft.margin_x, 0.0..=200.0).suffix(" px"));
        position_dirty |= response.changed();
    });

    ui.horizontal(|ui| {
        ui.label("Vertical margin:");
        let response =
            ui.add(egui::Slider::new(&mut app.draft.margin_y, 0.0..=200.0).suffix(" px"));
        position_dirty |= response.changed();
    });

    ui.add_space(12.0);
    section_heading(ui, "Overlay Size");

    ui.horizontal(|ui| {
        ui.label("Width:");
        let response =
            ui.add(egui::Slider::new(&mut app.draft.overlay_width, 100.0..=800.0).suffix(" px"));
        position_dirty |= response.changed();
    });

    ui.horizontal(|ui| {
        ui.label("Height:");
        let response =
            ui.add(egui::Slider::new(&mut app.draft.overlay_height, 60.0..=600.0).suffix(" px"));
        position_dirty |= response.changed();
    });

    if position_dirty {
        app.apply();
    }
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
