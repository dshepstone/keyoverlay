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
use theme::Palette;
use win_region::{apply_tray_region, PillRect};

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
/// Space reserved for the mouse icon in the pill layout (pixels).
const MOUSE_ICON_SLOT_W: i32 = 56;

fn ensure_windows_overlay_transparency() {}

/// Categories for pill coloring.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
enum PillKind {
    Modifier,
    Key,
    MouseClick,
    Scroll,
    MouseIcon,
}

#[derive(Clone)]
struct InputState {
    pressed_keys: HashSet<Key>,
    pressed_mouse_buttons: HashSet<MouseButton>,
    last_any_activity: Instant,
    last_mouse_activity: Instant,
    mouse_icon_active: bool,
    mouse_label: Option<String>,
    pending_left_press_at: Option<Instant>,
    /// Track whether the mouse has had a real button event (not just movement).
    had_button_event: bool,
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
            had_button_event: false,
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
                self.had_button_event = true;

                if e.button == MouseButton::Left {
                    self.pending_left_press_at = Some(now);
                } else {
                    self.mouse_label = Some(format!("{} Click", e.button));
                }
            }
            InputEvent::MouseUp(e) => {
                self.pressed_mouse_buttons.remove(&e.button);
                self.last_any_activity = now;
                self.last_mouse_activity = now;
                self.mouse_icon_active = true;
                self.had_button_event = true;

                if e.button == MouseButton::Left {
                    if let Some(press_at) = self.pending_left_press_at {
                        if now.duration_since(press_at) <= Duration::from_millis(250) {
                            self.mouse_label = Some("Left Click".to_string());
                        }
                    }
                    self.pending_left_press_at = None;
                } else {
                    self.mouse_label = Some(format!("{} Click", e.button));
                }
            }
            InputEvent::MouseWheel(_) => {
                self.last_any_activity = now;
                self.last_mouse_activity = now;
                self.mouse_icon_active = true;
                self.had_button_event = true;
                self.mouse_label = Some("Scroll".to_string());
            }
            InputEvent::MouseMove(_) => {
                // Mouse movement alone does NOT trigger overlay visibility,
                // does NOT set mouse_icon_active, and does NOT update
                // last_any_activity. This prevents false "Left Click"
                // labels from trackball/mouse movement without a real click.
            }
        }
    }

    fn tick(&mut self, now: Instant) {
        if let Some(press_at) = self.pending_left_press_at {
            if now.duration_since(press_at) > Duration::from_millis(250) {
                self.pending_left_press_at = None;
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
        if !self.mouse_icon_active || !self.had_button_event {
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

    /// Return the mouse highlight state (which button is currently pressed).
    fn mouse_highlight(&self) -> MouseHighlight {
        if self.pressed_mouse_buttons.contains(&MouseButton::Left) {
            MouseHighlight::Left
        } else if self.pressed_mouse_buttons.contains(&MouseButton::Right) {
            MouseHighlight::Right
        } else if self.pressed_mouse_buttons.contains(&MouseButton::Middle) {
            MouseHighlight::Middle
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
    pill_kinds: Vec<PillKind>,
    has_mouse_icon: bool,
    mouse_icon_rect: Option<PillRect>,
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
    last_hwnd: Option<isize>,
    fixed_origin: Option<egui::Pos2>,
    was_overlay_visible: bool,
    /// Fixed size established at activation, used for the entire visible period
    /// to prevent position drift when tray content changes width.
    fixed_size: Option<[f32; 2]>,
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
            last_hwnd: None,
            fixed_origin: None,
            was_overlay_visible: false,
            fixed_size: None,
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

        let sw = screen[0];
        let sh = screen[1];
        let w = win_size[0];
        let h = win_size[1];
        let mx = cfg.margin_x;
        let my = cfg.margin_y;

        let pos = match cfg.position {
            OverlayPosition::BottomCenter => egui::pos2((sw - w) / 2.0, sh - h - my),
            OverlayPosition::BottomLeft => egui::pos2(mx, sh - h - my),
            OverlayPosition::BottomRight => egui::pos2(sw - w - mx, sh - h - my),
            OverlayPosition::TopCenter => egui::pos2((sw - w) / 2.0, my),
            OverlayPosition::TopLeft => egui::pos2(mx, my),
            OverlayPosition::TopRight => egui::pos2(sw - w - mx, my),
            OverlayPosition::Center => egui::pos2((sw - w) / 2.0, (sh - h) / 2.0),
            OverlayPosition::Manual => egui::pos2(cfg.overlay_x, cfg.overlay_y),
        };

        // Clamp to visible screen area so the overlay never ends up off-screen.
        let x = pos.x.clamp(0.0, (sw - w).max(0.0));
        let y = pos.y.clamp(0.0, (sh - h).max(0.0));
        egui::pos2(x, y)
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

        let mut labels: Vec<(String, f32, PillKind)> = Vec::new();

        // Mouse icon gets a dedicated slot (not a text pill).
        let has_mouse_icon = mouse_icon_visible;

        if let Some(chord) = chord {
            for part in chord.split(" + ") {
                let kind = if is_modifier_key_label(part) {
                    PillKind::Modifier
                } else {
                    PillKind::Key
                };
                labels.push((part.to_owned(), chord_font, kind));
            }
        }
        if let Some(label) = mouse_label {
            let kind = if label.contains("Scroll") {
                PillKind::Scroll
            } else {
                PillKind::MouseClick
            };
            labels.push((label.to_owned(), event_font, kind));
        }

        let pill_count = labels.len();
        let mut pill_rects = Vec::with_capacity(pill_count);
        let mut pill_labels = Vec::with_capacity(pill_count);
        let mut pill_font_sizes = Vec::with_capacity(pill_count);
        let mut pill_kinds = Vec::with_capacity(pill_count);
        let mut x_px: i32 = 0;
        let mut content_h_px = 0;

        // Reserve space for mouse icon first if present.
        let mouse_icon_rect;
        if has_mouse_icon {
            let icon_h = PILL_MIN_H;
            mouse_icon_rect = Some(PillRect {
                x: x_px,
                y: 0,
                w: MOUSE_ICON_SLOT_W,
                h: icon_h,
                radius: PILL_RADIUS,
            });
            x_px += MOUSE_ICON_SLOT_W;
            if pill_count > 0 {
                x_px += GAP_BETWEEN_PILLS;
            }
            content_h_px = content_h_px.max(icon_h);
        } else {
            mouse_icon_rect = None;
        }

        for (idx, (label, font_points, kind)) in labels.into_iter().enumerate() {
            let text_size = Self::measure_text(ctx, &label, font_points);
            let text_w_px = (text_size.x * px_scale).round() as i32;
            let text_h_px = (text_size.y * px_scale).round() as i32;
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
            pill_kinds.push(kind);

            x_px += pill_w;
            if idx + 1 < pill_count {
                x_px += GAP_BETWEEN_PILLS;
            }
            content_h_px = content_h_px.max(pill_h);
        }

        if pill_rects.is_empty() && mouse_icon_rect.is_none() {
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
                pill_kinds,
                has_mouse_icon: false,
                mouse_icon_rect: None,
            };
        }

        let content_w_px = x_px;
        let tray_w = (content_w_px + TRAY_PAD_X * 2).max(1);
        let tray_h = (content_h_px + TRAY_PAD_Y * 2).max(1);

        let content_origin_x = (tray_w - content_w_px) / 2;
        let content_origin_y = (tray_h - content_h_px) / 2;
        for rect in &mut pill_rects {
            rect.x += content_origin_x;
            rect.y = content_origin_y + (content_h_px - rect.h) / 2;
        }

        // Adjust mouse icon rect into tray coordinates.
        let mouse_icon_rect = mouse_icon_rect.map(|mut r| {
            r.x += content_origin_x;
            r.y = content_origin_y + (content_h_px - r.h) / 2;
            r
        });

        OverlayGeometry {
            width_px: tray_w,
            height_px: tray_h,
            width_points: tray_w as f32 / px_scale,
            height_points: tray_h as f32 / px_scale,
            pill_rects,
            pill_labels,
            pill_font_sizes,
            pill_kinds,
            has_mouse_icon,
            mouse_icon_rect,
        }
    }

    fn maybe_apply_window_region(&mut self, geometry: &OverlayGeometry) {
        if self.last_region_geometry.as_ref() == Some(geometry) {
            return;
        }

        let width = geometry.width_px;
        let height = geometry.height_px;

        let hwnd = apply_tray_region(OVERLAY_VIEWPORT_TITLE, width, height, TRAY_RADIUS);

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

/// Check if a pill label corresponds to a modifier key.
fn is_modifier_key_label(label: &str) -> bool {
    matches!(label, "Ctrl" | "Shift" | "Alt" | "Super")
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

            let mouse_icon_visible =
                self.draft.show_mouse_icon && self.input_state.mouse_icon_visible(now);
            let mouse_label_visible = self.input_state.mouse_label_visible(now);
            let chord = self.input_state.current_chord();
            let mouse_label = self.input_state.mouse_label.clone();

            let visible_mouse_label = if mouse_label_visible {
                mouse_label.as_deref()
            } else {
                None
            };

            let has_content =
                chord.is_some() || mouse_icon_visible || visible_mouse_label.is_some();
            let overlay_visible = self.input_state.overlay_visible(now) && has_content;

            let overlay_id = egui::ViewportId::from_hash_of("overlay");

            if !overlay_visible {
                ctx.send_viewport_cmd_to(overlay_id, egui::ViewportCommand::Visible(false));
                self.last_region_geometry = None;
                self.fixed_origin = None;
                self.fixed_size = None;
                self.was_overlay_visible = false;
            } else {
                let palette = Palette::from_config(&self.draft);
                let mouse_highlight = self.input_state.mouse_highlight();

                let geometry = self.build_overlay_geometry(
                    ctx,
                    chord.as_deref(),
                    mouse_icon_visible,
                    visible_mouse_label,
                );

                // ── Activation: compute fixed position and size ONCE ──
                // The overlay appears instantly at this position with no
                // animation. Position and window size are locked for the
                // entire visible period to prevent any movement.
                let is_activation = !self.was_overlay_visible;
                if is_activation {
                    let size = [geometry.width_points, geometry.height_points];
                    let pos = self.compute_overlay_position(size, self.screen_size);
                    self.fixed_origin = Some(pos);
                    self.fixed_size = Some(size);
                }

                let win_pos = self.fixed_origin.unwrap();
                let win_size = self.fixed_size.unwrap();

                // Always use the fixed size from activation to prevent
                // the window from resizing (which causes repositioning).
                ctx.send_viewport_cmd_to(
                    overlay_id,
                    egui::ViewportCommand::InnerSize(egui::vec2(win_size[0], win_size[1])),
                );

                // Only reposition on the activation frame.
                if is_activation {
                    ctx.send_viewport_cmd_to(
                        overlay_id,
                        egui::ViewportCommand::OuterPosition(win_pos),
                    );
                }

                // Apply rounded region before showing.
                self.maybe_apply_window_region(&geometry);
                ctx.send_viewport_cmd_to(overlay_id, egui::ViewportCommand::Visible(true));

                self.was_overlay_visible = true;

                ctx.show_viewport_immediate(
                    overlay_id,
                    egui::ViewportBuilder::default()
                        .with_inner_size([win_size[0], win_size[1]])
                        .with_position(win_pos)
                        .with_title(OVERLAY_VIEWPORT_TITLE)
                        .with_decorations(false)
                        .with_always_on_top()
                        .with_resizable(false)
                        .with_transparent(true)
                        .with_mouse_passthrough(true),
                    move |ctx, _class| {
                        ensure_windows_overlay_transparency();

                        let mut vis = egui::Visuals::light();
                        vis.panel_fill = egui::Color32::TRANSPARENT;
                        vis.window_fill = egui::Color32::TRANSPARENT;
                        vis.extreme_bg_color = egui::Color32::TRANSPARENT;
                        ctx.set_visuals(vis);

                        egui::CentralPanel::default()
                            .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
                            .show(ctx, |ui| {
                                let panel_rect = ui.available_rect_before_wrap();
                                let tray_rounding = egui::Rounding::same(
                                    TRAY_RADIUS as f32 / ctx.pixels_per_point(),
                                );

                                // Draw tray background with theme color.
                                ui.painter().rect_filled(
                                    panel_rect,
                                    tray_rounding,
                                    palette.tray_bg,
                                );

                                // Draw tray border if the theme specifies one.
                                if palette.tray_border.a() > 0 {
                                    paint_rect_stroke_inside(
                                        ui,
                                        panel_rect,
                                        tray_rounding,
                                        egui::Stroke::new(1.5, palette.tray_border),
                                    );
                                }

                                if region_debug_bounds_enabled() {
                                    let window_debug_stroke = egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgb(0, 255, 0),
                                    );
                                    let rounding = egui::Rounding::same(0.0);
                                    paint_rect_stroke_inside(
                                        ui,
                                        panel_rect,
                                        rounding,
                                        window_debug_stroke,
                                    );

                                    let pill_debug_stroke = egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgb(255, 0, 255),
                                    );
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

                                // Draw mouse icon if present.
                                if let Some(icon_rect) = &geometry.mouse_icon_rect {
                                    let icon_area = egui::Rect::from_min_size(
                                        egui::pos2(
                                            icon_rect.x as f32 / px_scale,
                                            icon_rect.y as f32 / px_scale,
                                        ),
                                        egui::vec2(
                                            icon_rect.w as f32 / px_scale,
                                            icon_rect.h as f32 / px_scale,
                                        ),
                                    );

                                    let mut icon_ui = ui.child_ui(
                                        icon_area,
                                        egui::Layout::centered_and_justified(
                                            egui::Direction::TopDown,
                                        ),
                                    );
                                    draw_mouse_icon(
                                        &mut icon_ui,
                                        mouse_highlight,
                                        1.0,
                                        &palette,
                                    );
                                }

                                // Draw each pill with theme-aware colors.
                                for (idx, (rect, label)) in geometry
                                    .pill_rects
                                    .iter()
                                    .zip(geometry.pill_labels.iter())
                                    .enumerate()
                                {
                                    let kind = geometry.pill_kinds[idx];
                                    let font_size = geometry.pill_font_sizes[idx];

                                    let (pill_fill, pill_text) = match kind {
                                        PillKind::Modifier => (palette.mod_bg, palette.mod_fg),
                                        PillKind::Key => (palette.key_bg, palette.key_fg),
                                        PillKind::MouseClick => {
                                            (palette.mouse_bg, palette.mouse_fg)
                                        }
                                        PillKind::Scroll => (palette.scroll_bg, palette.scroll_fg),
                                        PillKind::MouseIcon => {
                                            // Mouse icon is drawn separately.
                                            continue;
                                        }
                                    };

                                    let pill_rect = egui::Rect::from_min_size(
                                        egui::pos2(
                                            rect.x as f32 / px_scale,
                                            rect.y as f32 / px_scale,
                                        ),
                                        egui::vec2(
                                            rect.w as f32 / px_scale,
                                            rect.h as f32 / px_scale,
                                        ),
                                    );
                                    let rounding =
                                        egui::Rounding::same(rect.radius as f32 / px_scale);
                                    ui.painter().rect_filled(pill_rect, rounding, pill_fill);
                                    ui.painter().text(
                                        pill_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        label,
                                        egui::FontId::proportional(font_size),
                                        pill_text,
                                    );
                                }
                            });
                    },
                );
            }
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
