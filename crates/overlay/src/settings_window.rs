use eframe::egui;
use keyoverlay_core::{AppConfig, OverlayLayout, OverlayPosition, SharedConfig, Theme};

use crate::theme::{c2e, e2c};

// ── Settings Tabs ────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsTab {
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

// ── Settings App ────────────────────────────────────────────────────────

pub struct SettingsApp {
    config: SharedConfig,
    /// Local working copy, flushed on save.
    draft: AppConfig,
    active_tab: SettingsTab,
    status_msg: Option<(String, std::time::Instant)>,
}

impl SettingsApp {
    pub fn new(config: SharedConfig) -> Self {
        let draft = config.lock().unwrap().clone();
        Self {
            config,
            draft,
            active_tab: SettingsTab::Appearance,
            status_msg: None,
        }
    }

    fn apply(&mut self) {
        *self.config.lock().unwrap() = self.draft.clone();
        self.draft.save();
        self.status_msg = Some(("Settings saved.".into(), std::time::Instant::now()));
    }

    fn reset_defaults(&mut self) {
        self.draft = AppConfig::default();
        self.apply();
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Dark theme for the settings window itself.
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
                // ── Header ──
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("KeyOverlay Settings")
                            .color(egui::Color32::from_rgb(220, 220, 240))
                            .size(20.0),
                    );
                });
                ui.add_space(8.0);

                // ── Tab bar ──
                ui.horizontal(|ui| {
                    for tab in SettingsTab::ALL {
                        let selected = self.active_tab == tab;
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
                            self.active_tab = tab;
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
                    .show(ui, |ui| match self.active_tab {
                        SettingsTab::Appearance => self.tab_appearance(ui),
                        SettingsTab::Behavior => self.tab_behavior(ui),
                        SettingsTab::Position => self.tab_position(ui),
                        SettingsTab::About => Self::tab_about(ui),
                    });

                // ── Bottom bar ──
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("Save & Apply").clicked() {
                        self.apply();
                    }
                    if ui.button("Reset Defaults").clicked() {
                        self.reset_defaults();
                    }

                    // Status message.
                    if let Some((msg, t)) = &self.status_msg {
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
}

// ── Tab Implementations ─────────────────────────────────────────────────

impl SettingsApp {
    fn tab_appearance(&mut self, ui: &mut egui::Ui) {
        section_heading(ui, "Theme");

        ui.horizontal(|ui| {
            ui.label("Color theme:");
            for theme in Theme::ALL {
                ui.selectable_value(&mut self.draft.theme, theme, theme.label());
            }
        });

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Font size:");
            ui.add(egui::Slider::new(&mut self.draft.font_size, 10.0..=32.0).suffix(" px"));
        });

        ui.horizontal(|ui| {
            ui.label("Overlay opacity:");
            ui.add(egui::Slider::new(
                &mut self.draft.overlay_opacity,
                0.1..=1.0,
            ));
        });

        ui.horizontal(|ui| {
            ui.label("Pill rounding:");
            ui.add(egui::Slider::new(&mut self.draft.pill_rounding, 0.0..=20.0).suffix(" px"));
        });

        // Custom colors section.
        if self.draft.theme == Theme::Custom {
            ui.add_space(12.0);
            section_heading(ui, "Custom Colors");

            color_row(ui, "Key background:", &mut self.draft.custom_key_bg);
            color_row(ui, "Key text:", &mut self.draft.custom_key_fg);
            color_row(ui, "Modifier background:", &mut self.draft.custom_mod_bg);
            color_row(ui, "Modifier text:", &mut self.draft.custom_mod_fg);
            color_row(
                ui,
                "Mouse click background:",
                &mut self.draft.custom_mouse_bg,
            );
            color_row(ui, "Mouse click text:", &mut self.draft.custom_mouse_fg);
            color_row(ui, "Scroll background:", &mut self.draft.custom_scroll_bg);
            color_row(ui, "Scroll text:", &mut self.draft.custom_scroll_fg);
        }
    }

    fn tab_behavior(&mut self, ui: &mut egui::Ui) {
        section_heading(ui, "Display");

        ui.horizontal(|ui| {
            ui.label("Display duration:");
            ui.add(
                egui::Slider::new(&mut self.draft.display_duration_secs, 0.5..=10.0).suffix(" s"),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Fade duration:");
            ui.add(egui::Slider::new(&mut self.draft.fade_duration_secs, 0.1..=3.0).suffix(" s"));
        });

        ui.horizontal(|ui| {
            ui.label("Max visible events:");
            let mut val = self.draft.max_visible_events as i32;
            ui.add(egui::Slider::new(&mut val, 1..=20));
            self.draft.max_visible_events = val as usize;
        });

        ui.add_space(12.0);
        section_heading(ui, "Input Filters");

        ui.checkbox(&mut self.draft.show_keyboard, "Show keyboard strokes");
        ui.checkbox(&mut self.draft.show_mouse_clicks, "Show mouse clicks");
        ui.checkbox(
            &mut self.draft.show_mouse_icon,
            "Show mouse icon in overlay",
        );
        ui.checkbox(&mut self.draft.show_scroll, "Show scroll events");
    }

    fn tab_position(&mut self, ui: &mut egui::Ui) {
        section_heading(ui, "Screen Position");

        ui.horizontal(|ui| {
            ui.label("Overlay position:");
            egui::ComboBox::from_id_source("position_combo")
                .selected_text(self.draft.position.label())
                .show_ui(ui, |ui: &mut egui::Ui| {
                    for pos in OverlayPosition::ALL {
                        ui.selectable_value(&mut self.draft.position, pos, pos.label());
                    }
                });
        });

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Layout direction:");
            for layout in OverlayLayout::ALL {
                ui.selectable_value(&mut self.draft.layout, layout, layout.label());
            }
        });

        ui.add_space(12.0);
        section_heading(ui, "Margins");

        ui.horizontal(|ui| {
            ui.label("Horizontal margin:");
            ui.add(egui::Slider::new(&mut self.draft.margin_x, 0.0..=200.0).suffix(" px"));
        });

        ui.horizontal(|ui| {
            ui.label("Vertical margin:");
            ui.add(egui::Slider::new(&mut self.draft.margin_y, 0.0..=200.0).suffix(" px"));
        });

        ui.add_space(12.0);
        section_heading(ui, "Overlay Size");

        ui.horizontal(|ui| {
            ui.label("Width:");
            ui.add(egui::Slider::new(&mut self.draft.overlay_width, 200.0..=800.0).suffix(" px"));
        });

        ui.horizontal(|ui| {
            ui.label("Height:");
            ui.add(egui::Slider::new(&mut self.draft.overlay_height, 100.0..=600.0).suffix(" px"));
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

        ui.add_space(16.0);
        ui.label(
            egui::RichText::new("Keyboard Shortcuts")
                .size(15.0)
                .strong()
                .color(egui::Color32::from_rgb(180, 180, 210)),
        );
        ui.add_space(4.0);
        ui.label(egui::RichText::new("Close overlay: close this settings window").size(13.0));
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
        let mut ec = c2e(*color);
        let mut rgba = [ec.r(), ec.g(), ec.b()];
        if ui.color_edit_button_srgb(&mut rgba).changed() {
            ec = egui::Color32::from_rgb(rgba[0], rgba[1], rgba[2]);
            *color = e2c(ec);
        }
    });
}
