mod mouse_icon;
mod settings_window;
mod theme;
mod win_region;

use std::collections::HashSet;
use std::env;
use std::sync::mpsc::Receiver;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use anyhow::Result;
use eframe::egui;
use eframe::epaint::Rgba;
use keyoverlay_core::{AppConfig, OverlayPosition, SharedConfig};
use keyoverlay_input::{InputEvent, Key, MouseButton};
use mouse_icon::{draw_mouse_icon, MouseHighlight};

use win_region::{apply_test_region, apply_tray_region, PillRect};

const OVERLAY_VIEWPORT_TITLE: &str = "KeyOverlayOverlay";
const OVERLAY_IDLE_HIDE_MS: u64 = 700;
const MOUSE_ICON_IDLE_HIDE_MS: u64 = 450;
const LARGE_KEY_FONT_BOOST: f32 = 14.0;
const GAP_BETWEEN_PILLS: i32 = 10;
const TRAY_PAD_X: i32 = 22;
const TRAY_PAD_Y: i32 = 18;
const TRAY_RADIUS: i32 = 22;
const PILL_PAD_X: i32 = 24;
const PILL_PAD_Y: i32 = 16;
const PILL_MIN_H: i32 = 46;
const PILL_MIN_W: i32 = 88;
const PILL_RADIUS: i32 = 18;
const MOUSE_ICON_TOKEN: &str = "__MOUSE_ICON__";

fn ensure_windows_overlay_transparency() {}

#[derive(Clone)]
struct InputState {
    pressed_keys: HashSet<Key>,
    pressed_mouse_buttons: HashSet<MouseButton>,
    last_any_activity: Instant,
    last_mouse_activity: Instant,
    mouse_icon_active: bool,
    mouse_label: Option<String>,
    pending_left_press_at: Option<Instant>,
    pending_left_press_moved: bool,
}

impl InputState {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            pressed_keys: HashSet::new(),
            pressed_mouse_buttons: HashSet::new(),
            last_any_activity: now,
            last_mouse_activity: now,
            mouse_icon_active: false,
            mouse_label: None,
            pending_left_press_at: None,
            pending_left_press_moved: false,
        }
    }

    fn apply_event(&mut self, event: InputEvent) {
        let now = event.timestamp();

        if mouse_debug_enabled() {
            match &event {
                InputEvent::MouseDown(e) => {
                    eprintln!("[mouse-debug] down button={} t={now:?}", e.button)
                }
                InputEvent::MouseUp(e) => {
                    eprintln!("[mouse-debug] up button={} t={now:?}", e.button)
                }
                InputEvent::MouseWheel(e) => {
                    eprintln!("[mouse-debug] wheel dir={} t={now:?}", e.direction)
                }
                InputEvent::MouseMove(_) => eprintln!("[mouse-debug] move t={now:?}"),
                _ => {}
            }
        }

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
                self.mouse_icon_active = true;

                if e.button == MouseButton::Left {
                    self.pending_left_press_at = Some(now);
                    self.pending_left_press_moved = false;
                } else {
                    self.mouse_label = Some(format!("{} Click", e.button));
                }
            }
            InputEvent::MouseUp(e) => {
                self.pressed_mouse_buttons.remove(&e.button);
                self.last_any_activity = now;
                self.last_mouse_activity = now;
                self.mouse_icon_active = true;

                if e.button == MouseButton::Left {
                    if let Some(press_at) = self.pending_left_press_at {
                        if !self.pending_left_press_moved
                            && now.duration_since(press_at) <= Duration::from_millis(250)
                        {
                            self.mouse_label = Some("Left Click".to_string());
                        }
                    }
                    self.pending_left_press_at = None;
                    self.pending_left_press_moved = false;
                } else {
                    self.mouse_label = Some(format!("{} Click", e.button));
                }
            }
            InputEvent::MouseWheel(_) => {
                self.last_any_activity = now;
                self.last_mouse_activity = now;
                self.mouse_icon_active = true;
                self.mouse_label = Some("Scroll".to_string());
            }
            InputEvent::MouseMove(_) => {
                self.last_any_activity = now;
                self.last_mouse_activity = now;
                self.mouse_icon_active = true;
                if self.pending_left_press_at.is_some() {
                    self.pending_left_press_moved = true;
                }
                self.mouse_label = None;
            }
        }
    }

    fn tick(&mut self, now: Instant) {
        if let Some(press_at) = self.pending_left_press_at {
            if now.duration_since(press_at) > Duration::from_millis(250) {
                self.pending_left_press_at = None;
                self.pending_left_press_moved = false;
                self.pressed_mouse_buttons.remove(&MouseButton::Left);
            }
        }
    }

    fn overlay_visible(&self, now: Instant) -> bool {
        if !self.pressed_keys.is_empty() || !self.pressed_mouse_buttons.is_empty() {
            return true;
        }
        now.duration_since(self.last_any_activity) < Duration::from_millis(OVERLAY_IDLE_HIDE_MS)
    }

    fn mouse_icon_visible(&self, now: Instant) -> bool {
        if !self.mouse_icon_active {
            return false;
        }
        now.duration_since(self.last_mouse_activity)
            < Duration::from_millis(MOUSE_ICON_IDLE_HIDE_MS)
    }

    fn mouse_label_visible(&self, now: Instant) -> bool {
        self.mouse_label.is_some() && self.mouse_icon_visible(now)
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

    fn mouse_highlight(&self) -> MouseHighlight {
        if self.pressed_mouse_buttons.contains(&MouseButton::Left) {
            MouseHighlight::Left
        } else if self.pressed_mouse_buttons.contains(&MouseButton::Middle) {
            MouseHighlight::Middle
        } else if self.pressed_mouse_buttons.contains(&MouseButton::Right) {
            MouseHighlight::Right
        } else {
            MouseHighlight::None
        }
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
    width_px: i32,
    height_px: i32,
    width_points: f32,
    height_points: f32,
    pill_rects: Vec<PillRect>,
    pill_labels: Vec<String>,
    pill_font_sizes: Vec<f32>,
}

fn region_debug_bounds_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| env::var("REGION_DEBUG_BOUNDS").is_ok_and(|v| v == "1"))
}

fn mouse_debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| env::var("MOUSE_DEBUG").is_ok_and(|v| v == "1"))
}

fn paint_rect_stroke_inside(
    ui: &egui::Ui,
    rect: egui::Rect,
    rounding: egui::Rounding,
    stroke: egui::Stroke,
) {
    // egui 0.27.2: rect_stroke(rect, rounding, stroke)
    // Emulate "inside stroke" by shrinking half the stroke width
    let inset = stroke.width * 0.5;
    let r = rect.shrink(inset);
    ui.painter().rect_stroke(r, rounding, stroke);
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
        mouse_icon_visible: bool,
        mouse_label: Option<&str>,
    ) -> OverlayGeometry {
        let px_scale: f32 = ctx.pixels_per_point();
        let chord_font: f32 = self.draft.font_size + LARGE_KEY_FONT_BOOST;
        let event_font: f32 = self.draft.font_size + 4.0;

        let mut labels: Vec<(String, f32)> = Vec::new();
        if mouse_icon_visible {
            labels.push((MOUSE_ICON_TOKEN.to_owned(), event_font));
        }
        if let Some(chord) = chord {
            labels.extend(chord.split(" + ").map(|part| (part.to_owned(), chord_font)));
        }
        if let Some(label) = mouse_label {
            labels.push((label.to_owned(), event_font));
        }

        let pill_count = labels.len();
        let mut pill_rects = Vec::with_capacity(pill_count);
        let mut pill_labels = Vec::with_capacity(pill_count);
        let mut pill_font_sizes = Vec::with_capacity(pill_count);
        let mut x_px = 0;
        let mut content_h_px = 0;

        for (idx, (label, font_points)) in labels.into_iter().enumerate() {
            let text_size = Self::measure_text(ctx, &label, font_points);
            let (text_w_px, text_h_px) = if label == MOUSE_ICON_TOKEN {
                (
                    (52.0 * px_scale).round() as i32,
                    (68.0 * px_scale).round() as i32,
                )
            } else {
                (
                    (text_size.x * px_scale).round() as i32,
                    (text_size.y * px_scale).round() as i32,
                )
            };
            let pill_w = (text_w_px + PILL_PAD_X * 2).max(PILL_MIN_W);
            let pill_h = (text_h_px + PILL_PAD_Y * 2).max(PILL_MIN_H);

            pill_rects.push(PillRect {
                x: x_px,
                y: 0,
                w: pill_w,
                h: pill_h,
                radius: PILL_RADIUS,
            });
            pill_labels.push(label);
            pill_font_sizes.push(font_points);

            x_px += pill_w;
            if idx + 1 < pill_count {
                x_px += GAP_BETWEEN_PILLS;
            }
            content_h_px = content_h_px.max(pill_h);
        }

        if pill_rects.is_empty() {
            let width_px = (220.0 * px_scale).round() as i32;
            let height_px = (76.0 * px_scale).round() as i32;
            return OverlayGeometry {
                width_px,
                height_px,
                width_points: width_px as f32 / px_scale,
                height_points: height_px as f32 / px_scale,
                pill_rects,
                pill_labels,
                pill_font_sizes,
            };
        }

        let content_w_px = pill_rects.last().map(|r| r.x + r.w).unwrap_or(0);
        let tray_w = (content_w_px + TRAY_PAD_X * 2).max(1);
        let tray_h = (content_h_px + TRAY_PAD_Y * 2).max(1);

        let content_origin_x = (tray_w - content_w_px) / 2;
        let content_origin_y = (tray_h - content_h_px) / 2;
        for rect in &mut pill_rects {
            rect.x += content_origin_x;
            rect.y = content_origin_y + (content_h_px - rect.h) / 2;
        }

        OverlayGeometry {
            width_px: tray_w,
            height_px: tray_h,
            width_points: tray_w as f32 / px_scale,
            height_points: tray_h as f32 / px_scale,
            pill_rects,
            pill_labels,
            pill_font_sizes,
        }
    }

    fn maybe_apply_window_region(&mut self, geometry: &OverlayGeometry) {
        if self.last_region_geometry.as_ref() == Some(geometry) {
            return;
        }

        let width = geometry.width_px;
        let height = geometry.height_px;

        let hwnd = if !self.region_test_applied {
            let hwnd = apply_test_region(OVERLAY_VIEWPORT_TITLE);
            self.region_test_applied = true;
            hwnd
        } else {
            apply_tray_region(OVERLAY_VIEWPORT_TITLE, width, height, TRAY_RADIUS)
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
        Rgba::from_rgb(1.0, 1.0, 1.0).to_array()
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
            self.input_state.tick(now);

            let overlay_visible = self.input_state.overlay_visible(now);
            let mouse_icon_visible =
                self.draft.show_mouse_icon && self.input_state.mouse_icon_visible(now);
            let mouse_label_visible = self.input_state.mouse_label_visible(now);
            let chord = self.input_state.current_chord();
            let mouse_label = self.input_state.mouse_label.clone();
            let mouse_highlight = self.input_state.mouse_highlight();

            let cfg = self.draft.clone();
            let visible_mouse_label = if mouse_label_visible {
                mouse_label.as_deref()
            } else {
                None
            };
            let geometry = self.build_overlay_geometry(
                ctx,
                chord.as_deref(),
                mouse_icon_visible,
                visible_mouse_label,
            );

            let win_pos = self.compute_overlay_position(
                [geometry.width_points, geometry.height_points],
                self.screen_size,
            );
            let overlay_id = egui::ViewportId::from_hash_of("overlay");

            if !overlay_visible {
                ctx.send_viewport_cmd_to(overlay_id, egui::ViewportCommand::Visible(false));
                self.last_region_geometry = None;
            } else {
                ctx.send_viewport_cmd_to(
                    overlay_id,
                    egui::ViewportCommand::InnerSize(egui::vec2(
                        geometry.width_points,
                        geometry.height_points,
                    )),
                );
                ctx.send_viewport_cmd_to(overlay_id, egui::ViewportCommand::OuterPosition(win_pos));
                // Resize/reposition first, then apply region in window-local coordinates.
                self.maybe_apply_window_region(&geometry);
                ctx.send_viewport_cmd_to(overlay_id, egui::ViewportCommand::Visible(true));
            }

            ctx.show_viewport_immediate(
                overlay_id,
                egui::ViewportBuilder::default()
                    .with_inner_size([geometry.width_points, geometry.height_points])
                    .with_position(win_pos)
                    .with_title(OVERLAY_VIEWPORT_TITLE)
                    .with_decorations(false)
                    .with_always_on_top()
                    .with_resizable(false)
                    .with_transparent(true)
                    .with_mouse_passthrough(true),
                move |ctx, _class| {
                    ensure_windows_overlay_transparency();
                    let palette = theme::palette_for(&cfg);

                    let mut vis = egui::Visuals::light();
                    vis.panel_fill = egui::Color32::WHITE;
                    vis.window_fill = egui::Color32::WHITE;
                    vis.extreme_bg_color = egui::Color32::WHITE;
                    ctx.set_visuals(vis);

                    egui::CentralPanel::default()
                        .frame(egui::Frame::none().fill(egui::Color32::WHITE))
                        .show(ctx, |ui| {
                            if !overlay_visible {
                                return;
                            }

                            let panel_rect = ui.available_rect_before_wrap();
                            let tray_rounding =
                                egui::Rounding::same(TRAY_RADIUS as f32 / ctx.pixels_per_point());
                            ui.painter().rect_filled(
                                panel_rect,
                                tray_rounding,
                                egui::Color32::WHITE,
                            );

                            if region_debug_bounds_enabled() {
                                let window_debug_stroke =
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 255, 0));
                                let rounding = egui::Rounding::same(0.0);
                                paint_rect_stroke_inside(
                                    ui,
                                    panel_rect,
                                    rounding,
                                    window_debug_stroke,
                                );

                                let pill_debug_stroke =
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 0, 255));
                                let rounding = egui::Rounding::same(0.0);
                                let px_scale = ctx.pixels_per_point();
                                for rect in &geometry.pill_rects {
                                    let min = egui::pos2(
                                        rect.x as f32 / px_scale,
                                        rect.y as f32 / px_scale,
                                    );
                                    let size = egui::vec2(
                                        rect.w as f32 / px_scale,
                                        rect.h as f32 / px_scale,
                                    );
                                    let pill_rect = egui::Rect::from_min_size(min, size);
                                    paint_rect_stroke_inside(
                                        ui,
                                        pill_rect,
                                        rounding,
                                        pill_debug_stroke,
                                    );
                                }
                            }

                            let px_scale = ctx.pixels_per_point();
                            let pill_fill = egui::Color32::from_rgb(122, 71, 255);
                            let pill_text = egui::Color32::WHITE;

                            for ((rect, label), font_size) in geometry
                                .pill_rects
                                .iter()
                                .zip(geometry.pill_labels.iter())
                                .zip(geometry.pill_font_sizes.iter())
                            {
                                let pill_rect = egui::Rect::from_min_size(
                                    egui::pos2(rect.x as f32 / px_scale, rect.y as f32 / px_scale),
                                    egui::vec2(rect.w as f32 / px_scale, rect.h as f32 / px_scale),
                                );
                                let rounding = egui::Rounding::same(rect.radius as f32 / px_scale);
                                ui.painter().rect_filled(pill_rect, rounding, pill_fill);
                                if label == MOUSE_ICON_TOKEN {
                                    draw_mouse_icon(
                                        ui.painter(),
                                        pill_rect,
                                        mouse_highlight,
                                        1.0,
                                        &palette,
                                    );
                                } else {
                                    ui.painter().text(
                                        pill_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        label,
                                        egui::FontId::proportional(*font_size),
                                        pill_text,
                                    );
                                }
                            }
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
