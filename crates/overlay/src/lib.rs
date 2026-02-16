mod mouse_icon;
mod settings_window;
mod startup_debug;
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
use keyoverlay_input::{InputEvent, Key, Modifiers, MouseButton, ScrollDirection};
use mouse_icon::{draw_mouse_icon, MouseHighlight, ScrollArrowDirection};

use win_region::{
    apply_test_region, apply_tray_region, apply_tray_region_with_redraw, disable_dwm_transitions,
    find_hwnd_by_title, force_redraw, log_win_state, OverlayHwnd, PillRect,
};

const OVERLAY_VIEWPORT_TITLE: &str = "KeyOverlayOverlay";
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

#[derive(Clone, Copy)]
struct ScrollHighlight {
    dir: ScrollArrowDirection,
    last_event: Instant,
}

fn ensure_windows_overlay_transparency() {}

#[derive(Clone)]
struct InputState {
    pressed_keys: HashSet<Key>,
    pressed_mouse_buttons: HashSet<MouseButton>,
    key_sequence_started_at: Instant,
    last_mouse_activity: Instant,
    mouse_icon_active: bool,
    mouse_label: Option<String>,
    last_mouse_was_scroll: bool,
    pending_left_press_at: Option<Instant>,
    pending_left_press_moved: bool,
    display_chord: Option<String>,
    last_mouse_highlight: MouseHighlight,
    scroll_highlight: Option<ScrollHighlight>,
}

impl InputState {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            pressed_keys: HashSet::new(),
            pressed_mouse_buttons: HashSet::new(),
            key_sequence_started_at: now,
            last_mouse_activity: now,
            mouse_icon_active: false,
            mouse_label: None,
            last_mouse_was_scroll: false,
            pending_left_press_at: None,
            pending_left_press_moved: false,
            display_chord: None,
            last_mouse_highlight: MouseHighlight::None,
            scroll_highlight: None,
        }
    }

    fn chord_from_pressed_keys(&self) -> Option<String> {
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

    fn apply_event(&mut self, event: InputEvent) {
        let now = event.timestamp();

        if mouse_debug_enabled() {
            match &event {
                InputEvent::MouseClick(e) => {
                    eprintln!("[mouse-debug] click button={} t={now:?}", e.button)
                }
                InputEvent::Scroll(e) => {
                    eprintln!("[mouse-debug] wheel dir={} t={now:?}", e.direction)
                }
                _ => {}
            }
        }

        match event {
            InputEvent::Key(e) => {
                self.pressed_keys.clear();
                if e.modifiers.contains(Modifiers::CTRL) {
                    self.pressed_keys.insert(Key::Ctrl);
                }
                if e.modifiers.contains(Modifiers::SHIFT) {
                    self.pressed_keys.insert(Key::Shift);
                }
                if e.modifiers.contains(Modifiers::ALT) {
                    self.pressed_keys.insert(Key::Alt);
                }
                if e.modifiers.contains(Modifiers::WIN) {
                    self.pressed_keys.insert(Key::Win);
                }
                self.pressed_keys.insert(e.key);
                self.key_sequence_started_at = now;
                self.display_chord = self
                    .chord_from_pressed_keys()
                    .or_else(|| Some(e.display_string()));
            }
            InputEvent::MouseClick(e) => {
                self.pressed_mouse_buttons.clear();
                self.last_mouse_activity = now;
                self.mouse_icon_active = true;
                self.last_mouse_highlight = MouseHighlight::from_button(e.button);
                self.last_mouse_was_scroll = false;
                self.mouse_label = Some(format!("{} Click", e.button));
                self.pending_left_press_at = None;
                self.pending_left_press_moved = false;
            }
            InputEvent::Scroll(e) => {
                self.pressed_mouse_buttons.clear();
                self.last_mouse_activity = now;
                self.mouse_icon_active = true;
                self.last_mouse_highlight = MouseHighlight::None;
                self.last_mouse_was_scroll = true;
                self.mouse_label = Some("Scroll".to_string());
                self.scroll_highlight = Some(ScrollHighlight {
                    dir: match e.direction {
                        ScrollDirection::Up => ScrollArrowDirection::Up,
                        ScrollDirection::Down => ScrollArrowDirection::Down,
                    },
                    last_event: now,
                });
            }
        }
    }

    fn tick(&mut self, now: Instant, mouse_hold_for: Duration, keyboard_hold_for: Duration) {
        if !self.mouse_icon_visible(now, mouse_hold_for) {
            self.last_mouse_highlight = MouseHighlight::None;
            self.scroll_highlight = None;
        }

        if now.duration_since(self.key_sequence_started_at) >= keyboard_hold_for {
            self.pressed_keys.clear();
        }
        self.pressed_mouse_buttons.clear();
    }

    fn overlay_visible(
        &self,
        now: Instant,
        hold_for: Duration,
        keyboard_enabled: bool,
        mouse_enabled: bool,
    ) -> bool {
        let keyboard_active = keyboard_enabled
            && (!self.pressed_keys.is_empty()
                || (self.display_chord.is_some()
                    && now.duration_since(self.key_sequence_started_at) < hold_for));
        let mouse_active = mouse_enabled
            && (!self.pressed_mouse_buttons.is_empty()
                || (self.mouse_icon_active
                    && now.duration_since(self.last_mouse_activity) < hold_for));

        keyboard_active || mouse_active
    }

    fn mouse_icon_visible(&self, now: Instant, hold_for: Duration) -> bool {
        self.mouse_icon_active && now.duration_since(self.last_mouse_activity) < hold_for
    }

    fn mouse_label_for_display(
        &self,
        now: Instant,
        hold_for: Duration,
        show_mouse_clicks: bool,
        show_mouse_event_text: bool,
        show_scroll: bool,
    ) -> Option<String> {
        if !self.mouse_icon_visible(now, hold_for) {
            return None;
        }

        if self.last_mouse_was_scroll && !show_scroll {
            return None;
        }
        if !self.last_mouse_was_scroll && !show_mouse_clicks {
            return None;
        }
        if !show_mouse_event_text {
            return None;
        }

        self.mouse_label.clone()
    }

    fn chord_for_display(
        &self,
        now: Instant,
        hold_for: Duration,
        show_keyboard: bool,
    ) -> Option<String> {
        if !show_keyboard {
            return None;
        }

        if !self.pressed_keys.is_empty() {
            return self
                .display_chord
                .clone()
                .or_else(|| self.chord_from_pressed_keys());
        }

        if now.duration_since(self.key_sequence_started_at) < hold_for {
            return self.display_chord.clone();
        }

        None
    }

    fn mouse_highlight(&self, now: Instant, hold_for: Duration) -> MouseHighlight {
        if self.pressed_mouse_buttons.contains(&MouseButton::Left) {
            MouseHighlight::Left
        } else if self.pressed_mouse_buttons.contains(&MouseButton::Middle) {
            MouseHighlight::Middle
        } else if self.pressed_mouse_buttons.contains(&MouseButton::Right) {
            MouseHighlight::Right
        } else if self.mouse_icon_visible(now, hold_for) {
            self.last_mouse_highlight
        } else {
            MouseHighlight::None
        }
    }

    fn scroll_arrow_for_display(
        &mut self,
        now: Instant,
        display_duration: Duration,
        fade_duration: Duration,
        show_mouse_icon: bool,
        show_scroll: bool,
    ) -> Option<(ScrollArrowDirection, f32)> {
        if !show_mouse_icon || !show_scroll {
            return None;
        }

        let highlight = self.scroll_highlight?;
        let elapsed = now.duration_since(highlight.last_event);

        if elapsed <= display_duration {
            return Some((highlight.dir, 1.0));
        }

        if fade_duration.is_zero() {
            self.scroll_highlight = None;
            return None;
        }

        let fade_elapsed = elapsed - display_duration;
        if fade_elapsed >= fade_duration {
            self.scroll_highlight = None;
            return None;
        }

        let alpha = 1.0 - (fade_elapsed.as_secs_f32() / fade_duration.as_secs_f32());
        Some((highlight.dir, alpha.clamp(0.0, 1.0)))
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
    last_hwnd: Option<OverlayHwnd>,
    transitions_disabled: bool,
    manual_preview_until: Option<Instant>,
    /// True until the first update frame has drained buffered startup events.
    startup_drain: bool,
    startup_experiment: startup_debug::StartupExperiment,
    startup_delay_until: Option<Instant>,
    startup_region_prepared: bool,
    first_frame_logged: bool,
    first_input_logged: bool,
    title_logged: bool,
    viewport_config_logged: bool,
    last_overlay_visible: bool,
}

impl App {
    fn new(rx: Receiver<InputEvent>, config: SharedConfig) -> Self {
        let mut draft = config.lock().unwrap().clone();
        if draft.font_size < 26.0 {
            draft.font_size = 26.0;
        }
        draft.overlay_scale = draft.overlay_scale.clamp(0.6, 2.0);
        if draft.position == OverlayPosition::Manual
            && (draft.overlay_x - 500.0).abs() < f32::EPSILON
            && (draft.overlay_y - 500.0).abs() < f32::EPSILON
        {
            let (x, y) = draft.manual_lower_right_position();
            draft.overlay_x = x;
            draft.overlay_y = y;
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
            transitions_disabled: false,
            manual_preview_until: None,
            startup_drain: true,
            startup_experiment: startup_debug::StartupExperiment::from_env(),
            startup_delay_until: None,
            startup_region_prepared: false,
            first_frame_logged: false,
            first_input_logged: false,
            title_logged: false,
            viewport_config_logged: false,
            last_overlay_visible: false,
        }
    }

    fn apply(&mut self) {
        if self.draft.font_size < 26.0 {
            self.draft.font_size = 26.0;
        }
        self.draft.overlay_scale = self.draft.overlay_scale.clamp(0.6, 2.0);
        *self.config.lock().unwrap() = self.draft.clone();
        self.draft.save();
        self.status_msg = Some(("Settings saved.".into(), Instant::now()));
    }

    fn reset_defaults(&mut self) {
        self.draft = AppConfig::default();
        self.draft.font_size = 26.0;
        self.draft.overlay_scale = self.draft.overlay_scale.clamp(0.6, 2.0);
        self.apply();
    }

    fn sync_manual_to_lower_right(&mut self) {
        let (x, y) = self.draft.manual_lower_right_position();
        self.draft.overlay_x = x;
        self.draft.overlay_y = y;
    }

    fn preview_manual_position(&mut self) {
        self.manual_preview_until = Some(Instant::now() + Duration::from_secs(2));
    }

    fn compute_overlay_position(&self, win_size: [f32; 2], _screen: [f32; 2]) -> egui::Pos2 {
        let cfg = &self.draft;
        if cfg.position == OverlayPosition::Manual {
            return egui::pos2(cfg.overlay_x, cfg.overlay_y);
        }

        let [sw, sh] = cfg.scaled_display_size_points();
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
        let overlay_scale = self.draft.overlay_scale.clamp(0.6, 2.0);
        let scaled_px = |value: i32| ((value as f32) * overlay_scale).round() as i32;

        let chord_font: f32 = (self.draft.font_size + LARGE_KEY_FONT_BOOST) * overlay_scale;
        let event_font: f32 = (self.draft.font_size + 4.0) * overlay_scale;

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
                    (52.0 * overlay_scale * px_scale).round() as i32,
                    (68.0 * overlay_scale * px_scale).round() as i32,
                )
            } else {
                (
                    (text_size.x * px_scale).round() as i32,
                    (text_size.y * px_scale).round() as i32,
                )
            };
            let pill_w = (text_w_px + scaled_px(PILL_PAD_X) * 2).max(scaled_px(PILL_MIN_W));
            let pill_h = (text_h_px + scaled_px(PILL_PAD_Y) * 2).max(scaled_px(PILL_MIN_H));

            pill_rects.push(PillRect {
                x: x_px,
                y: 0,
                w: pill_w,
                h: pill_h,
                radius: scaled_px(PILL_RADIUS),
            });
            pill_labels.push(label);
            pill_font_sizes.push(font_points);

            x_px += pill_w;
            if idx + 1 < pill_count {
                x_px += scaled_px(GAP_BETWEEN_PILLS);
            }
            content_h_px = content_h_px.max(pill_h);
        }

        if pill_rects.is_empty() {
            let width_px = (220.0 * overlay_scale * px_scale).round() as i32;
            let height_px = (76.0 * overlay_scale * px_scale).round() as i32;
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
        let tray_w = (content_w_px + scaled_px(TRAY_PAD_X) * 2).max(1);
        let tray_h = (content_h_px + scaled_px(TRAY_PAD_Y) * 2).max(1);

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

    fn maybe_apply_window_region(&mut self, geometry: &OverlayGeometry, currently_visible: bool) {
        let now = Instant::now();

        if self.startup_experiment == startup_debug::StartupExperiment::CDelayRegion {
            let until = self
                .startup_delay_until
                .get_or_insert_with(|| now + Duration::from_millis(200));
            if now < *until {
                startup_debug::log("experiment C: delaying SetWindowRgn for 200ms");
                return;
            }
        }

        if self.startup_experiment == startup_debug::StartupExperiment::ESkipRegion {
            startup_debug::log("experiment E: skipping SetWindowRgn");
            return;
        }

        if self.last_region_geometry.as_ref() == Some(geometry) {
            return;
        }

        let width = geometry.width_px;
        let height = geometry.height_px;
        let redraw =
            self.startup_experiment != startup_debug::StartupExperiment::BRegionNoRedrawThenRedraw;

        let hwnd = if !self.region_test_applied {
            let hwnd = apply_test_region(OVERLAY_VIEWPORT_TITLE);
            self.region_test_applied = true;
            hwnd
        } else if redraw {
            apply_tray_region(
                OVERLAY_VIEWPORT_TITLE,
                width,
                height,
                ((TRAY_RADIUS as f32) * self.draft.overlay_scale.clamp(0.6, 2.0)).round() as i32,
            )
        } else {
            apply_tray_region_with_redraw(
                OVERLAY_VIEWPORT_TITLE,
                width,
                height,
                ((TRAY_RADIUS as f32) * self.draft.overlay_scale.clamp(0.6, 2.0)).round() as i32,
                false,
            )
        };

        if let Some(hwnd_val) = hwnd {
            startup_debug::log(format!(
                "region apply path hwnd={hwnd_val:?} currently_visible={currently_visible}"
            ));
            log_win_state(hwnd_val, "after-region-apply");

            if let Some(last) = self.last_hwnd {
                if last != hwnd_val {
                    eprintln!("[overlay-region] HWND changed old={last:?} new={hwnd_val:?}");
                    self.transitions_disabled = false;
                    startup_debug::log("HWND changed; transitions_disabled reset");
                }
            }

            let skip_dwm = self.startup_experiment == startup_debug::StartupExperiment::FSkipDwm;
            if !self.transitions_disabled && !skip_dwm {
                if let Err(err) = disable_dwm_transitions(hwnd_val) {
                    eprintln!("[overlay-region] failed to disable DWM transitions: {err}");
                } else {
                    self.transitions_disabled = true;
                }
            } else if skip_dwm {
                startup_debug::log("experiment F: skipping DwmSetWindowAttribute");
            }

            if !redraw {
                force_redraw(hwnd_val);
            }

            self.last_hwnd = Some(hwnd_val);
            self.startup_region_prepared = true;
        } else {
            startup_debug::log("FindWindowW did not return overlay HWND yet");
        }

        self.last_region_geometry = Some(geometry.clone());
    }

    fn maybe_log_hwnd_state(&mut self) {
        if self.last_hwnd.is_none() {
            self.last_hwnd = find_hwnd_by_title(OVERLAY_VIEWPORT_TITLE);
        }

        if let Some(hwnd) = self.last_hwnd {
            log_win_state(hwnd, "frame-snapshot");
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Rgba::from_rgb(1.0, 1.0, 1.0).to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.first_frame_logged {
            startup_debug::log("first update/frame");
            self.first_frame_logged = true;
        }

        if self.startup_drain {
            // Discard any events that buffered while eframe/wgpu was
            // initialising so they don't cause a freeze or phantom state.
            while self.rx.try_recv().is_ok() {}
            self.input_state = InputState::new();
            self.startup_drain = false;
        }

        while let Ok(event) = self.rx.try_recv() {
            if !self.first_input_logged {
                startup_debug::log(format!("first input event received: {event:?}"));
                self.first_input_logged = true;
            }
            self.input_state.apply_event(event);
        }

        *self.config.lock().unwrap() = self.draft.clone();

        let screen_rect = ctx.input(|i| i.screen_rect());
        startup_debug::log(format!(
            "frame input screen_rect={}x{} px_per_point={:.3}",
            screen_rect.width(),
            screen_rect.height(),
            ctx.pixels_per_point()
        ));
        if startup_debug::enabled() {
            ctx.input(|i| {
                for ev in &i.events {
                    startup_debug::log(format!("egui event: {ev:?}"));
                }
            });
        }
        if screen_rect.width() > 100.0 && screen_rect.height() > 100.0 {
            self.screen_size = [screen_rect.width(), screen_rect.height()];
        }

        settings_window::draw_settings(ctx, self);

        if self.draft.overlay_enabled {
            let now = Instant::now();
            let overlay_hold_for = Duration::from_secs_f32(
                (self.draft.display_duration_secs + self.draft.fade_duration_secs).max(0.1),
            );
            let mouse_hold_for = overlay_hold_for;
            let display_duration =
                Duration::from_secs_f32(self.draft.display_duration_secs.max(0.0));
            let fade_duration = Duration::from_secs_f32(self.draft.fade_duration_secs.max(0.0));
            let mouse_events_enabled = self.draft.show_mouse_clicks
                || self.draft.show_scroll
                || self.draft.show_mouse_icon;

            self.input_state.tick(now, mouse_hold_for, overlay_hold_for);

            let mut overlay_visible = self.input_state.overlay_visible(
                now,
                overlay_hold_for,
                self.draft.show_keyboard,
                mouse_events_enabled,
            );
            if self.manual_preview_until.is_some_and(|until| now <= until)
                && self.active_tab == settings_window::SettingsTab::Position
                && self.draft.position == OverlayPosition::Manual
            {
                overlay_visible = true;
            }
            let mouse_icon_visible = self.draft.show_mouse_icon
                && self.input_state.mouse_icon_visible(now, mouse_hold_for);
            let chord =
                self.input_state
                    .chord_for_display(now, overlay_hold_for, self.draft.show_keyboard);
            let mouse_label = self.input_state.mouse_label_for_display(
                now,
                mouse_hold_for,
                self.draft.show_mouse_clicks,
                self.draft.show_mouse_event_text,
                self.draft.show_scroll,
            );
            let mouse_highlight = self.input_state.mouse_highlight(now, mouse_hold_for);
            let scroll_arrow = self.input_state.scroll_arrow_for_display(
                now,
                display_duration,
                fade_duration,
                self.draft.show_mouse_icon,
                self.draft.show_scroll,
            );

            let cfg = self.draft.clone();
            let visible_mouse_label = mouse_label.as_deref();
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
            let startup_hidden_mode =
                self.startup_experiment == startup_debug::StartupExperiment::DHiddenUntilPrepared;
            let should_show_overlay = overlay_visible;

            if self.startup_experiment == startup_debug::StartupExperiment::ADisableDwmBeforeVisible
                && !self.transitions_disabled
            {
                if let Some(hwnd) = find_hwnd_by_title(OVERLAY_VIEWPORT_TITLE) {
                    startup_debug::log(
                        "experiment A: disable DWM transitions as soon as HWND exists",
                    );
                    if disable_dwm_transitions(hwnd).is_ok() {
                        self.transitions_disabled = true;
                        self.last_hwnd = Some(hwnd);
                    }
                }
            }

            let overlay_visible = if startup_hidden_mode {
                should_show_overlay && self.startup_region_prepared
            } else {
                should_show_overlay
            };

            if self.last_overlay_visible != overlay_visible {
                startup_debug::log(format!(
                    "visibility toggle {} -> {} (requested={should_show_overlay})",
                    self.last_overlay_visible, overlay_visible
                ));
                self.last_overlay_visible = overlay_visible;
            }

            if !should_show_overlay {
                ctx.send_viewport_cmd_to(overlay_id, egui::ViewportCommand::Close);
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
                startup_debug::log(format!(
                    "startup geometry size_pt={:.1}x{:.1} size_px={}x{} pos=({:.1},{:.1})",
                    geometry.width_points,
                    geometry.height_points,
                    geometry.width_px,
                    geometry.height_px,
                    win_pos.x,
                    win_pos.y
                ));
                // Resize/reposition first, then apply region in window-local coordinates.
                self.maybe_apply_window_region(&geometry, overlay_visible);

                if self.startup_experiment == startup_debug::StartupExperiment::DHiddenUntilPrepared
                    && !self.startup_region_prepared
                {
                    startup_debug::log("experiment D: keep hidden until region prepared");
                    ctx.send_viewport_cmd_to(overlay_id, egui::ViewportCommand::Visible(false));
                } else {
                    ctx.send_viewport_cmd_to(
                        overlay_id,
                        egui::ViewportCommand::Visible(overlay_visible),
                    );
                }

                if startup_debug::enabled() {
                    self.maybe_log_hwnd_state();
                }
            }

            if should_show_overlay {
                let transparent = if self.startup_experiment
                    == startup_debug::StartupExperiment::GEnableTransparencyAfterFirstShow
                {
                    self.startup_region_prepared
                } else {
                    true
                };

                if !self.viewport_config_logged {
                    startup_debug::log(format!(
                        "viewport config title={OVERLAY_VIEWPORT_TITLE} decorations=false transparent={transparent} always_on_top=true"
                    ));
                    self.viewport_config_logged = true;
                }

                if !self.title_logged {
                    startup_debug::log("overlay title assigned in ViewportBuilder");
                    self.title_logged = true;
                }

                ctx.show_viewport_immediate(
                    overlay_id,
                    egui::ViewportBuilder::default()
                        .with_inner_size([geometry.width_points, geometry.height_points])
                        .with_position(win_pos)
                        .with_title(OVERLAY_VIEWPORT_TITLE)
                        .with_decorations(false)
                        .with_titlebar_shown(false)
                        .with_titlebar_buttons_shown(false)
                        .with_taskbar(false)
                        .with_always_on_top()
                        .with_resizable(false)
                        .with_transparent(transparent)
                        .with_mouse_passthrough(true),
                    move |ctx, _class| {
                        ensure_windows_overlay_transparency();
                        let palette = theme::Palette::from_config(&cfg);

                        let mut vis = egui::Visuals::light();
                        vis.panel_fill = egui::Color32::WHITE;
                        vis.window_fill = egui::Color32::WHITE;
                        vis.extreme_bg_color = egui::Color32::WHITE;
                        ctx.set_visuals(vis);

                        egui::CentralPanel::default()
                            .frame(egui::Frame::none().fill(egui::Color32::WHITE))
                            .show(ctx, |ui| {
                                let panel_rect = ui.available_rect_before_wrap();
                                let tray_rounding = egui::Rounding::same(
                                    (TRAY_RADIUS as f32 * cfg.overlay_scale.clamp(0.6, 2.0))
                                        / ctx.pixels_per_point(),
                                );
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
                                let pill_fill = egui::Color32::from_rgb(122, 71, 255);
                                let pill_text = egui::Color32::WHITE;

                                for ((rect, label), font_size) in geometry
                                    .pill_rects
                                    .iter()
                                    .zip(geometry.pill_labels.iter())
                                    .zip(geometry.pill_font_sizes.iter())
                                {
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
                                    if label == MOUSE_ICON_TOKEN {
                                        draw_mouse_icon(
                                            ui.painter(),
                                            pill_rect,
                                            mouse_highlight,
                                            1.0,
                                            scroll_arrow,
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
        }

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

pub fn run(
    rx: Receiver<InputEvent>,
    config: SharedConfig,
    app_icon: Option<egui::IconData>,
) -> Result<()> {
    startup_debug::log(format!(
        "overlay process startup; experiment={:?}",
        startup_debug::StartupExperiment::from_env()
    ));

    let viewport = if let Some(app_icon) = app_icon {
        egui::ViewportBuilder::default()
            .with_inner_size([520.0, 600.0])
            .with_resizable(true)
            .with_min_inner_size([420.0, 400.0])
            .with_transparent(true)
            .with_icon(app_icon)
    } else {
        egui::ViewportBuilder::default()
            .with_inner_size([520.0, 600.0])
            .with_resizable(true)
            .with_min_inner_size([420.0, 400.0])
            .with_transparent(true)
    };

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "KeyOverlay",
        options,
        Box::new(move |_cc| Box::new(App::new(rx, config))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
