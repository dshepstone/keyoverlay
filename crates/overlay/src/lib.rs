mod mouse_icon;
mod overlay_window;
mod settings_window;
mod theme;

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

struct App {
    config: SharedConfig,
    draft: AppConfig,
    active_tab: settings_window::SettingsTab,
    status_msg: Option<(String, Instant)>,
    rx: Receiver<InputEvent>,
    input_state: InputState,
    screen_size: [f32; 2],
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

            let win_w = cfg.overlay_width;
            let win_h = cfg.overlay_height;
            let win_pos = self.compute_overlay_position([win_w, win_h], self.screen_size);

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("overlay"),
                egui::ViewportBuilder::default()
                    .with_inner_size([win_w, win_h])
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
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "KeyOverlay",
        options,
        Box::new(move |_cc| Box::new(App::new(rx, config))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
