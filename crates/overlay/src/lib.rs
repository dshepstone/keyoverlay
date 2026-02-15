mod mouse_icon;
mod overlay_window;
mod settings_window;
mod theme;
mod win_region;

use std::collections::HashSet;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::Result;
use eframe::egui;
use eframe::epaint::Rgba;
use keyoverlay_core::{AppConfig, OverlayPosition, SharedConfig};
use keyoverlay_input::{InputEvent, Key, MouseButton};

use mouse_icon::{draw_mouse_icon, MouseHighlight};
use overlay_window::{draw_chord_pill, draw_event_pill};
use theme::Palette;
use win_region::{apply_pill_region, apply_test_region, PillRect};

const OVERLAY_VIEWPORT_TITLE: &str = "KeyOverlayOverlay";
const OVERLAY_IDLE_HIDE_MS: u64 = 700;
const MOUSE_ICON_IDLE_HIDE_MS: u64 = 450;
const OVERLAY_PADDING: f32 = 14.0;
const LARGE_KEY_FONT_BOOST: f32 = 14.0;

fn ensure_windows_overlay_transparency() {}

#[derive(Clone)]
struct InputState {
    pressed_keys: HashSet<Key>,
    pressed_mouse_buttons: HashSet<MouseButton>,
    last_any_activity: Instant,
    last_mouse_activity: Instant,
    last_mouse_event_label: Option<String>,
}

impl InputState {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            pressed_keys: HashSet::new(),
            pressed_mouse_buttons: HashSet::new(),
            last_any_activity: now,
            last_mouse_activity: now,
            last_mouse_event_label: None,
        }
    }

    fn apply_event(&mut self, event: InputEvent) {
        let now = event.timestamp();
        match event {
            InputEvent::KeyDown(e) => {
                self.pressed_keys.insert(e.key);
                self.last_any_activity = now;
            }
            InputEvent::KeyUp(e) => {
                self.pressed_keys.remove(&e.key);
                self.last_any_activity = now;
            }
            InputEvent::MouseDown(e) => {
                self.pressed_mouse_buttons.insert(e.button);
                self.last_any_activity = now;
                self.last_mouse_activity = now;
                self.last_mouse_event_label = Some(format!("{}", e.button));
            }
            InputEvent::MouseUp(e) => {
                self.pressed_mouse_buttons.remove(&e.button);
                self.last_any_activity = now;
                self.last_mouse_activity = now;
            }
            InputEvent::MouseWheel(e) => {
                self.last_any_activity = now;
                self.last_mouse_activity = now;
                self.last_mouse_event_label = Some(format!("{}", e.direction));
            }
            InputEvent::MouseMove(_) => {
                self.last_any_activity = now;
                self.last_mouse_activity = now;
            }
        }
    }

    fn overlay_visible(&self, now: Instant) -> bool {
        if !self.pressed_keys.is_empty() || !self.pressed_mouse_buttons.is_empty() {
            return true;
        }
        now.duration_since(self.last_any_activity) < Duration::from_millis(OVERLAY_IDLE_HIDE_MS)
    }

    fn mouse_visible(&self, now: Instant) -> bool {
        if !self.pressed_mouse_buttons.is_empty() {
            return true;
        }
        now.duration_since(self.last_mouse_activity)
            < Duration::from_millis(MOUSE_ICON_IDLE_HIDE_MS)
    }

    fn mouse_event_visible(&self, now: Instant) -> bool {
        self.last_mouse_event_label.is_some() && self.mouse_visible(now)
    }

    fn current_chord(&self) -> Option<String> {
        if self.pressed_keys.is_empty() {
            return None;
        }

        let mut keys: Vec<Key> = self.pressed_keys.iter().copied().collect();
        keys.sort_by_key(|k| key_sort_rank(*k));

        Some(
            keys.into_iter()
                .map(|k| k.to_string())
                .collect::<Vec<_>>()
                .join(" + "),
        )
    }
}

fn key_sort_rank(key: Key) -> (u8, String) {
    let modifier_rank = match key {
        Key::Ctrl => 0,
        Key::Shift => 1,
        Key::Alt => 2,
        Key::Win => 3,
        _ => 10,
    };
    (modifier_rank, key.to_string())
}

#[derive(Clone, Debug, PartialEq)]
struct OverlayGeometry {
    width: f32,
    height: f32,
    pill_rects: Vec<PillRect>,
}

struct App {
    config: SharedConfig,
    draft: AppConfig,
    active_tab: settings_window::SettingsTab,
    status_msg: Option<(String, Instant)>,
    rx: Receiver<InputEvent>,
    input_state: InputState,
    screen_size: [f32; 2],
    last_region_geometry: Option<OverlayGeometry>,
    region_test_applied: bool,
    last_hwnd: Option<isize>,
}

impl App {
    fn new(rx: Receiver<InputEvent>, config: SharedConfig) -> Self {
        let mut draft = config.lock().unwrap().clone();
        if draft.font_size < 26.0 {
            draft.font_size = 26.0;
        }

        Self {
            config,
            draft,
            active_tab: settings_window::SettingsTab::Appearance,
            status_msg: None,
            rx,
            input_state: InputState::new(),
            screen_size: [1920.0, 1080.0],
            last_region_geometry: None,
            region_test_applied: false,
            last_hwnd: None,
        }
    }

    fn apply(&mut self) {
        if self.draft.font_size < 26.0 {
            self.draft.font_size = 26.0;
        }
        *self.config.lock().unwrap() = self.draft.clone();
        self.draft.save();
        self.status_msg = Some(("Settings saved.".into(), Instant::now()));
    }

    fn reset_defaults(&mut self) {
        self.draft = AppConfig::default();
        self.draft.font_size = 26.0;
        self.apply();
    }

    fn compute_overlay_position(&self, win_size: [f32; 2], screen: [f32; 2]) -> egui::Pos2 {
        let cfg = &self.draft;
        if cfg.position == OverlayPosition::Manual {
            return egui::pos2(cfg.overlay_x, cfg.overlay_y);
        }

        let sw = screen[0];
        let sh = screen[1];
        let w = win_size[0];
        let h = win_size[1];
        let mx = cfg.margin_x;
        let my = cfg.margin_y;

        match cfg.position {
            OverlayPosition::BottomCenter => egui::pos2((sw - w) / 2.0, sh - h - my),
            OverlayPosition::BottomLeft => egui::pos2(mx, sh - h - my),
            OverlayPosition::BottomRight => egui::pos2(sw - w - mx, sh - h - my),
            OverlayPosition::TopCenter => egui::pos2((sw - w) / 2.0, my),
            OverlayPosition::TopLeft => egui::pos2(mx, my),
            OverlayPosition::TopRight => egui::pos2(sw - w - mx, my),
            OverlayPosition::Center => egui::pos2((sw - w) / 2.0, (sh - h) / 2.0),
            OverlayPosition::Manual => egui::pos2(cfg.overlay_x, cfg.overlay_y),
        }
    }

    fn mouse_highlight(&self) -> MouseHighlight {
        if self
            .input_state
            .pressed_mouse_buttons
            .contains(&MouseButton::Left)
        {
            MouseHighlight::Left
        } else if self
            .input_state
            .pressed_mouse_buttons
            .contains(&MouseButton::Right)
        {
            MouseHighlight::Right
        } else if self
            .input_state
            .pressed_mouse_buttons
            .contains(&MouseButton::Middle)
        {
            MouseHighlight::Middle
        } else {
            MouseHighlight::None
        }
    }

    fn measure_text(ctx: &egui::Context, text: &str, font_size: f32) -> egui::Vec2 {
        let galley = ctx.fonts(|fonts| {
            fonts.layout_no_wrap(
                text.to_owned(),
                egui::FontId::proportional(font_size),
                egui::Color32::WHITE,
            )
        });
        galley.size()
    }

    fn build_overlay_geometry(
        &self,
        ctx: &egui::Context,
        chord: Option<&str>,
        mouse_visible: bool,
        mouse_event_label: Option<&str>,
    ) -> OverlayGeometry {
        let padding: f32 = OVERLAY_PADDING;
        let chord_font: f32 = self.draft.font_size + LARGE_KEY_FONT_BOOST;
        let event_font: f32 = self.draft.font_size + 4.0;

        let mut chord_width: f32 = 0.0;
        let mut chord_height: f32 = 0.0;
        let mut chord_rects: Vec<PillRect> = Vec::new();

        if let Some(chord) = chord {
            let parts: Vec<&str> = chord.split(" + ").collect();
            let mut x: f32 = 0.0;
            let row_h: f32 = chord_font + 20.0;
            chord_height = row_h;

            for (idx, part) in parts.iter().enumerate() {
                let size = Self::measure_text(ctx, part, chord_font);
                let seg_w: f32 = size.x + 24.0;
                chord_rects.push(PillRect {
                    x: x.round() as i32,
                    y: 0,
                    w: seg_w.ceil() as i32,
                    h: row_h.ceil() as i32,
                    radius: self.draft.pill_rounding.round() as i32,
                });
                x += seg_w;
                if idx < parts.len() - 1 {
                    x += 26.0;
                }
            }
            chord_width = x;
        }

        let mut event_width: f32 = 0.0;
        let mut event_height: f32 = 0.0;
        if let Some(label) = mouse_event_label {
            let size = Self::measure_text(ctx, label, event_font);
            event_width = size.x + 32.0;
            event_height = size.y + 16.0;
        }

        let text_width: f32 = chord_width.max(event_width);
        let mut text_height: f32 = 0.0;
        if chord_height > 0.0 {
            text_height += chord_height;
        }
        if chord_height > 0.0 && event_height > 0.0 {
            text_height += 10.0;
        }
        if event_height > 0.0 {
            text_height += event_height;
        }

        let mouse_width: f32 = if mouse_visible { 48.0 } else { 0.0 };
        let mouse_height: f32 = if mouse_visible { 68.0 } else { 0.0 };
        let mouse_gap: f32 = if mouse_visible && text_width > 0.0 {
            16.0
        } else {
            0.0
        };

        let content_width: f32 = mouse_width + mouse_gap + text_width;
        let content_height: f32 = mouse_height.max(text_height);

        let window_width: f32 = (content_width + padding * 2.0).ceil().max(1.0);
        let window_height: f32 = (content_height + padding * 2.0).ceil().max(1.0);

        let mut rects: Vec<PillRect> = Vec::new();
        let text_left: f32 = padding + mouse_width + mouse_gap;
        let text_top: f32 = padding + (content_height - text_height) / 2.0;

        for mut r in chord_rects {
            r.x = (text_left + r.x as f32).round() as i32;
            r.y = text_top.round() as i32;
            rects.push(r);
        }

        if mouse_event_label.is_some() {
            let event_y = text_top
                + if chord_height > 0.0 {
                    chord_height + 10.0
                } else {
                    0.0
                };
            rects.push(PillRect {
                x: text_left.round() as i32,
                y: event_y.round() as i32,
                w: event_width.ceil() as i32,
                h: event_height.ceil() as i32,
                radius: self.draft.pill_rounding.round() as i32,
            });
        }

        if mouse_visible {
            let mouse_y: f32 = padding + (content_height - mouse_height) / 2.0;
            rects.push(PillRect {
                x: padding.round() as i32,
                y: mouse_y.round() as i32,
                w: mouse_width as i32,
                h: mouse_height as i32,
                radius: 16,
            });
        }

        OverlayGeometry {
            width: window_width,
            height: window_height,
            pill_rects: rects,
        }
    }

    fn maybe_apply_window_region(&mut self, geometry: &OverlayGeometry) {
        if self.last_region_geometry.as_ref() == Some(geometry) {
            return;
        }

        let width = geometry.width.round() as i32;
        let height = geometry.height.round() as i32;

        let hwnd = if !self.region_test_applied {
            let hwnd = apply_test_region(OVERLAY_VIEWPORT_TITLE);
            self.region_test_applied = true;
            hwnd
        } else {
            apply_pill_region(OVERLAY_VIEWPORT_TITLE, width, height, &geometry.pill_rects)
        };

        if let Some(hwnd_val) = hwnd {
            if let Some(last) = self.last_hwnd {
                if last != hwnd_val {
                    eprintln!("[overlay-region] HWND changed old=0x{last:x} new=0x{hwnd_val:x}");
                }
            }
            self.last_hwnd = Some(hwnd_val);
        }

        self.last_region_geometry = Some(geometry.clone());
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(event) = self.rx.try_recv() {
            self.input_state.apply_event(event);
        }

        *self.config.lock().unwrap() = self.draft.clone();

        let screen_rect = ctx.input(|i| i.screen_rect());
        if screen_rect.width() > 100.0 && screen_rect.height() > 100.0 {
            self.screen_size = [screen_rect.width(), screen_rect.height()];
        }

        settings_window::draw_settings(ctx, self);

        if self.draft.overlay_enabled {
            let now = Instant::now();
            let overlay_visible = self.input_state.overlay_visible(now);
            let mouse_visible = self.draft.show_mouse_icon && self.input_state.mouse_visible(now);
            let mouse_event_visible = self.input_state.mouse_event_visible(now);
            let chord = self.input_state.current_chord();
            let mouse_highlight = self.mouse_highlight();
            let mouse_event_label = self.input_state.last_mouse_event_label.clone();

            let cfg = self.draft.clone();
            let palette = Palette::from_config(&cfg);

            let visible_mouse_label = if mouse_event_visible {
                mouse_event_label.as_deref()
            } else {
                None
            };
            let geometry = self.build_overlay_geometry(
                ctx,
                chord.as_deref(),
                mouse_visible,
                visible_mouse_label,
            );

            let win_pos =
                self.compute_overlay_position([geometry.width, geometry.height], self.screen_size);
            let overlay_id = egui::ViewportId::from_hash_of("overlay");

            if !overlay_visible {
                ctx.send_viewport_cmd_to(overlay_id, egui::ViewportCommand::Visible(false));
                self.last_region_geometry = None;
            } else {
                ctx.send_viewport_cmd_to(
                    overlay_id,
                    egui::ViewportCommand::InnerSize(egui::vec2(geometry.width, geometry.height)),
                );
                ctx.send_viewport_cmd_to(overlay_id, egui::ViewportCommand::OuterPosition(win_pos));
                // Resize/reposition first, then apply region in window-local coordinates.
                self.maybe_apply_window_region(&geometry);
                ctx.send_viewport_cmd_to(overlay_id, egui::ViewportCommand::Visible(true));
            }

            ctx.show_viewport_immediate(
                overlay_id,
                egui::ViewportBuilder::default()
                    .with_inner_size([geometry.width, geometry.height])
                    .with_position(win_pos)
                    .with_title(OVERLAY_VIEWPORT_TITLE)
                    .with_decorations(false)
                    .with_always_on_top()
                    .with_resizable(false)
                    .with_transparent(true)
                    .with_mouse_passthrough(true),
                move |ctx, _class| {
                    ensure_windows_overlay_transparency();

                    let mut vis = egui::Visuals::dark();
                    vis.panel_fill = egui::Color32::TRANSPARENT;
                    vis.window_fill = egui::Color32::TRANSPARENT;
                    vis.extreme_bg_color = egui::Color32::TRANSPARENT;
                    ctx.set_visuals(vis);

                    egui::CentralPanel::default()
                        .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
                        .show(ctx, |ui| {
                            if !overlay_visible {
                                return;
                            }

                            let panel_rect = ui.available_rect_before_wrap();
                            let bg_alpha =
                                (cfg.background_opacity * cfg.overlay_opacity * 255.0) as u8;
                            let bg_color =
                                egui::Color32::from_rgba_unmultiplied(30, 30, 40, bg_alpha);
                            ui.painter()
                                .rect_filled(panel_rect, cfg.pill_rounding + 6.0, bg_color);

                            let content_rect = panel_rect.shrink(OVERLAY_PADDING);
                            let mut content_ui = ui.child_ui(
                                content_rect,
                                egui::Layout::left_to_right(egui::Align::Center),
                            );

                            content_ui.horizontal_centered(|ui| {
                                if mouse_visible {
                                    draw_mouse_icon(
                                        ui,
                                        mouse_highlight,
                                        cfg.overlay_opacity,
                                        &palette,
                                    );
                                    ui.add_space(16.0);
                                }

                                ui.vertical_centered(|ui| {
                                    if let Some(chord) = chord {
                                        draw_chord_pill(
                                            ui,
                                            &chord,
                                            cfg.overlay_opacity,
                                            &palette,
                                            cfg.pill_rounding,
                                            cfg.font_size + LARGE_KEY_FONT_BOOST,
                                        );
                                    }

                                    if mouse_event_visible {
                                        if let Some(label) = mouse_event_label.as_deref() {
                                            ui.add_space(10.0);
                                            draw_event_pill(
                                                ui,
                                                label,
                                                cfg.overlay_opacity,
                                                palette.mouse_bg,
                                                palette.mouse_fg,
                                                cfg.pill_rounding,
                                                cfg.font_size + 4.0,
                                            );
                                        }
                                    }
                                });
                            });
                        });
                },
            );
        }

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

pub fn run(rx: Receiver<InputEvent>, config: SharedConfig) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 600.0])
            .with_resizable(true)
            .with_min_inner_size([420.0, 400.0])
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "KeyOverlay",
        options,
        Box::new(move |_cc| Box::new(App::new(rx, config))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
