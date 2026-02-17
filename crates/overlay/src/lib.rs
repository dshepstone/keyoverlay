mod mouse_icon;
mod overlay_startup_diagnostics;
mod settings_window;
mod theme;
mod win_region;

use std::collections::{HashMap, HashSet};
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
    apply_tray_region_hwnd_with_redraw, disable_dwm_transitions, find_hwnd_by_title, force_redraw,
    hwnd_is_valid, snapshot_hwnd_state, OverlayHwnd, PillRect,
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
fn overlay_viewport_title() -> String {
    if overlay_startup_diagnostics::enabled() {
        format!("{OVERLAY_VIEWPORT_TITLE} (pid={})", std::process::id())
    } else {
        OVERLAY_VIEWPORT_TITLE.to_string()
    }
}

#[derive(Clone, Copy)]
struct ScrollHighlight {
    dir: ScrollArrowDirection,
    last_event: Instant,
}

#[derive(Clone)]
struct RenderToken {
    label: String,
    is_mouse: bool,
    mouse_highlight: MouseHighlight,
    alpha: f32,
    scale: f32,
    pulse: f32,
    font_size: f32,
}

#[derive(Clone)]
struct SingleTileState {
    label: String,
    is_down: bool,
    down_until: Option<Instant>,
    last_event_at: Instant,
    anim_down: f32,
    visible_alpha: f32,
}

impl SingleTileState {
    fn new(now: Instant) -> Self {
        Self {
            label: String::new(),
            is_down: false,
            down_until: None,
            last_event_at: now,
            anim_down: 0.0,
            visible_alpha: 0.0,
        }
    }

    fn register_press(
        &mut self,
        label: String,
        now: Instant,
        auto_release_after: Option<Duration>,
    ) {
        self.label = label;
        self.is_down = true;
        self.down_until = auto_release_after.map(|d| now + d);
        self.last_event_at = now;
        self.visible_alpha = 1.0;
    }

    fn register_release(&mut self, now: Instant) {
        self.is_down = false;
        self.down_until = None;
        self.last_event_at = now;
    }

    fn tick(&mut self, now: Instant, dt: f32, idle_ms: u64) {
        if self.is_down && self.down_until.is_some_and(|until| now >= until) {
            self.is_down = false;
            self.down_until = None;
        }

        let down_target = if self.is_down { 1.0 } else { 0.0 };
        let down_factor = 1.0 - (-38.0 * dt.max(0.0)).exp();
        self.anim_down += (down_target - self.anim_down) * down_factor;

        let alpha_target = if !self.label.is_empty()
            && now.duration_since(self.last_event_at) <= Duration::from_millis(idle_ms)
        {
            1.0
        } else {
            0.0
        };
        let alpha_factor = 1.0 - (-12.0 * dt.max(0.0)).exp();
        self.visible_alpha += (alpha_target - self.visible_alpha) * alpha_factor;
    }
}

#[derive(Clone)]
struct MousePressVisualState {
    is_down: bool,
    pressed_at: Instant,
    released_at: Option<Instant>,
    anim_down: f32,
    alpha: f32,
}

#[derive(Clone)]
struct OverlayTrayState {
    key_tile: SingleTileState,
    scroll_tile: SingleTileState,
    active_mouse: HashMap<MouseButton, MousePressVisualState>,
    pill_w: f32,
    pill_w_target: f32,
    tray_alpha: f32,
    left_anchor: Option<egui::Pos2>,
}

impl OverlayTrayState {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            key_tile: SingleTileState::new(now),
            scroll_tile: SingleTileState::new(now),
            active_mouse: HashMap::new(),
            pill_w: 0.0,
            pill_w_target: 0.0,
            tray_alpha: 0.0,
            left_anchor: None,
        }
    }
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
                    eprintln!(
                        "[mouse-debug] click button={} down={} t={now:?}",
                        e.button, e.is_down
                    )
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
                if e.is_down {
                    self.pressed_keys.insert(e.key);
                }
                self.key_sequence_started_at = now;
                self.display_chord = self.chord_from_pressed_keys();
                if e.is_down && self.display_chord.is_none() {
                    self.display_chord = Some(e.display_string());
                }
            }
            InputEvent::MouseClick(e) => {
                self.pressed_mouse_buttons.clear();
                self.last_mouse_activity = now;
                self.last_mouse_was_scroll = false;
                self.mouse_label = Some(format!("{} Click", e.button));
                self.pending_left_press_at = None;
                self.pending_left_press_moved = false;
                if e.is_down {
                    self.mouse_icon_active = true;
                    self.last_mouse_highlight = MouseHighlight::from_button(e.button);
                } else {
                    self.last_mouse_highlight = MouseHighlight::None;
                }
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
    pill_is_mouse: Vec<bool>,
    pill_font_sizes: Vec<f32>,
    pill_alphas: Vec<f32>,
    pill_scales: Vec<f32>,
    pill_pulses: Vec<f32>,
    pill_mouse_highlights: Vec<MouseHighlight>,
}

fn region_debug_bounds_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| env::var("REGION_DEBUG_BOUNDS").is_ok_and(|v| v == "1"))
}

fn mouse_debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| env::var("MOUSE_DEBUG").is_ok_and(|v| v == "1"))
}

fn single_tile_debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| env::var("OVERLAY_SINGLE_TILE_DEBUG").is_ok_and(|v| v == "1"))
}

fn mouse_hold_debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| env::var("OVERLAY_MOUSE_HOLD_DEBUG").is_ok_and(|v| v == "1"))
}

fn ease_out_cubic(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0);
    1.0 - (1.0 - x).powi(3)
}

fn ease_in_cubic(t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0);
    x.powi(3)
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
    tray_state: OverlayTrayState,
    screen_size: [f32; 2],
    last_region_geometry: Option<OverlayGeometry>,
    last_hwnd: Option<OverlayHwnd>,
    transitions_disabled: bool,
    manual_preview_until: Option<Instant>,
    /// True until the first update frame has drained buffered startup events.
    startup_drain: bool,
    startup_experiment: overlay_startup_diagnostics::OverlayExperiment,
    startup_delay_until: Option<Instant>,
    startup_region_prepared: bool,
    first_frame_logged: bool,
    first_input_logged: bool,
    title_logged: bool,
    viewport_config_logged: bool,
    last_overlay_visible: bool,
    fixed_experiment_pos: Option<egui::Pos2>,
    first_show_done: bool,
    pending_region: Option<(i32, i32, i32)>,
    region_settle_deadline: Option<Instant>,
    region_redraw_needed: bool,
    region_debounce: Duration,
    last_sent_outer_pos: Option<egui::Pos2>,
    last_sent_inner_size: Option<egui::Vec2>,
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
            tray_state: OverlayTrayState::new(),
            screen_size: [1920.0, 1080.0],
            last_region_geometry: None,
            last_hwnd: None,
            transitions_disabled: false,
            manual_preview_until: None,
            startup_drain: true,
            startup_experiment: overlay_startup_diagnostics::OverlayExperiment::from_env(),
            startup_delay_until: None,
            startup_region_prepared: false,
            first_frame_logged: false,
            first_input_logged: false,
            title_logged: false,
            viewport_config_logged: false,
            last_overlay_visible: false,
            fixed_experiment_pos: None,
            first_show_done: false,
            pending_region: None,
            region_settle_deadline: None,
            region_redraw_needed: false,
            region_debounce: Duration::from_millis(120),
            last_sent_outer_pos: None,
            last_sent_inner_size: None,
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

    fn tray_on_event(&mut self, event: &InputEvent, now: Instant) {
        match event {
            InputEvent::Key(e) => {
                let label = e.key.to_string();
                if e.is_down {
                    self.tray_state
                        .key_tile
                        .register_press(label.clone(), now, None);
                    if single_tile_debug_enabled() {
                        eprintln!("[overlay-single] KeyDown label={label}");
                    }
                } else {
                    self.tray_state.key_tile.register_release(now);
                    if single_tile_debug_enabled() {
                        eprintln!("[overlay-single] KeyUp label={label}");
                    }
                }
            }
            InputEvent::MouseClick(e) => {
                if e.is_down {
                    self.tray_state.active_mouse.insert(
                        e.button,
                        MousePressVisualState {
                            is_down: true,
                            pressed_at: now,
                            released_at: None,
                            anim_down: 1.0,
                            alpha: 1.0,
                        },
                    );
                    if mouse_hold_debug_enabled() {
                        eprintln!(
                            "[overlay-mouse-hold] down {:?} -> held (expiry=none) active={}",
                            e.button,
                            self.tray_state.active_mouse.len()
                        );
                    }
                } else if let Some(state) = self.tray_state.active_mouse.get_mut(&e.button) {
                    state.is_down = false;
                    state.released_at = Some(now);
                    if mouse_hold_debug_enabled() {
                        eprintln!(
                            "[overlay-mouse-hold] up {:?} -> fade_start active={}",
                            e.button,
                            self.tray_state.active_mouse.len()
                        );
                    }
                }
            }
            InputEvent::Scroll(e) => {
                let label = match e.direction {
                    ScrollDirection::Up => "Wheel↑",
                    ScrollDirection::Down => "Wheel↓",
                }
                .to_string();
                self.tray_state.scroll_tile.register_press(
                    label.clone(),
                    now,
                    Some(Duration::from_millis(140)),
                );
                if mouse_hold_debug_enabled() {
                    eprintln!("[overlay-mouse-hold] scroll pulse label={label} expiry=140ms");
                }
                if single_tile_debug_enabled() {
                    eprintln!("[overlay-single] MouseWheel label={label}");
                }
            }
        }
    }

    fn build_render_tokens(&mut self, now: Instant, dt: f32) -> Vec<RenderToken> {
        const IDLE_FADE_MS: u64 = 450;
        const MOUSE_FADE_OUT_MS: f32 = 200.0;

        self.tray_state.key_tile.tick(now, dt, IDLE_FADE_MS);
        self.tray_state.scroll_tile.tick(now, dt, IDLE_FADE_MS);

        let mut released_buttons = Vec::new();
        for (button, state) in &mut self.tray_state.active_mouse {
            let _held_ms = now.duration_since(state.pressed_at).as_millis();
            let down_target = if state.is_down { 1.0 } else { 0.0 };
            let down_factor = 1.0 - (-38.0 * dt.max(0.0)).exp();
            state.anim_down += (down_target - state.anim_down) * down_factor;

            if state.is_down {
                state.alpha = 1.0;
            } else if let Some(released_at) = state.released_at {
                let elapsed_ms = now.duration_since(released_at).as_secs_f32() * 1000.0;
                let t = (elapsed_ms / MOUSE_FADE_OUT_MS).clamp(0.0, 1.0);
                state.alpha = 1.0 - t;
                if t >= 1.0 {
                    released_buttons.push(*button);
                }
            }
        }
        for button in released_buttons {
            self.tray_state.active_mouse.remove(&button);
            if mouse_hold_debug_enabled() {
                eprintln!("[overlay-mouse-hold] remove {:?} after fade", button);
            }
        }

        let mut tokens = Vec::with_capacity(4);

        if self.draft.show_mouse_icon {
            for button in [MouseButton::Left, MouseButton::Middle, MouseButton::Right] {
                if let Some(state) = self.tray_state.active_mouse.get(&button) {
                    tokens.push(RenderToken {
                        label: format!("{:?}", button),
                        is_mouse: true,
                        mouse_highlight: MouseHighlight::from_button(button),
                        alpha: state.alpha.clamp(0.0, 1.0),
                        scale: 1.0 - (state.anim_down * 0.015),
                        pulse: 0.0,
                        font_size: self.draft.font_size + 4.0,
                    });
                }
            }

            if self.tray_state.scroll_tile.visible_alpha > 0.01 {
                tokens.push(RenderToken {
                    label: self.tray_state.scroll_tile.label.clone(),
                    is_mouse: true,
                    mouse_highlight: MouseHighlight::None,
                    alpha: self.tray_state.scroll_tile.visible_alpha.clamp(0.0, 1.0),
                    scale: 1.0 - (self.tray_state.scroll_tile.anim_down * 0.015),
                    pulse: 0.0,
                    font_size: self.draft.font_size + 4.0,
                });
            }
        }

        if self.tray_state.key_tile.visible_alpha > 0.01 {
            tokens.push(RenderToken {
                label: self.tray_state.key_tile.label.clone(),
                is_mouse: false,
                mouse_highlight: MouseHighlight::None,
                alpha: self.tray_state.key_tile.visible_alpha.clamp(0.0, 1.0),
                scale: 1.0 - (self.tray_state.key_tile.anim_down * 0.015),
                pulse: 0.0,
                font_size: self.draft.font_size + LARGE_KEY_FONT_BOOST,
            });
        }

        if single_tile_debug_enabled() {
            eprintln!(
                "[overlay-single] tokens={} key_down={:.2} held_mouse={}",
                tokens.len(),
                self.tray_state.key_tile.anim_down,
                self.tray_state
                    .active_mouse
                    .values()
                    .filter(|m| m.is_down)
                    .count()
            );
        }

        tokens
    }

    fn build_overlay_geometry(
        &mut self,
        ctx: &egui::Context,
        tokens: &[RenderToken],
        dt: f32,
    ) -> OverlayGeometry {
        let px_scale: f32 = ctx.pixels_per_point();
        let overlay_scale = self.draft.overlay_scale.clamp(0.6, 2.0);
        let scaled_px = |value: i32| ((value as f32) * overlay_scale).round() as i32;

        let pill_count = tokens.len();
        let mut pill_rects = Vec::with_capacity(pill_count);
        let mut pill_labels = Vec::with_capacity(pill_count);
        let mut pill_is_mouse = Vec::with_capacity(pill_count);
        let mut pill_font_sizes = Vec::with_capacity(pill_count);
        let mut pill_alphas = Vec::with_capacity(pill_count);
        let mut pill_scales = Vec::with_capacity(pill_count);
        let mut pill_pulses = Vec::with_capacity(pill_count);
        let mut pill_mouse_highlights = Vec::with_capacity(pill_count);
        let mut content_h_px = 0;

        let mut token_widths = Vec::with_capacity(tokens.len());
        let mut token_heights = Vec::with_capacity(tokens.len());
        for token in tokens {
            let label = token.label.clone();
            let font_points = token.font_size * overlay_scale;
            let text_size = Self::measure_text(ctx, &label, font_points);
            let (text_w_px, text_h_px) = (
                (text_size.x * px_scale).round() as i32,
                (text_size.y * px_scale).round() as i32,
            );
            let pill_w = (text_w_px + scaled_px(PILL_PAD_X) * 2).max(scaled_px(PILL_MIN_W));
            let pill_h = (text_h_px + scaled_px(PILL_PAD_Y) * 2).max(scaled_px(PILL_MIN_H));

            token_widths.push(pill_w);
            token_heights.push(pill_h);
        }

        // Left-to-right anchored layout with fixed origin.
        let mut cursor_x = 0i32;

        for (idx, token) in tokens.iter().enumerate() {
            let label = token.label.clone();
            let font_points = token.font_size * overlay_scale;
            let pill_w = token_widths[idx];
            let pill_h = token_heights[idx];
            let item_x = cursor_x;

            pill_rects.push(PillRect {
                x: item_x,
                y: 0,
                w: pill_w,
                h: pill_h,
                radius: scaled_px(PILL_RADIUS),
            });
            pill_labels.push(label);
            pill_is_mouse.push(token.is_mouse);
            pill_font_sizes.push(font_points);
            pill_alphas.push(token.alpha);
            pill_scales.push(token.scale);
            pill_pulses.push(token.pulse);
            pill_mouse_highlights.push(token.mouse_highlight);

            content_h_px = content_h_px.max(pill_h);
            cursor_x += pill_w;
            if idx + 1 < tokens.len() {
                cursor_x += scaled_px(GAP_BETWEEN_PILLS);
            }
        }

        if pill_rects.is_empty() {
            let k = 16.0;
            let factor = 1.0 - (-k * dt.max(0.0)).exp();
            self.tray_state.pill_w += (0.0 - self.tray_state.pill_w) * factor;
            self.tray_state.tray_alpha += (0.0 - self.tray_state.tray_alpha) * factor;
            let width_px = (self.tray_state.pill_w.max(1.0) * px_scale).round() as i32;
            let height_px = (76.0 * overlay_scale * px_scale).round() as i32;
            return OverlayGeometry {
                width_px,
                height_px,
                width_points: width_px as f32 / px_scale,
                height_points: height_px as f32 / px_scale,
                pill_rects,
                pill_labels,
                pill_is_mouse,
                pill_font_sizes,
                pill_alphas,
                pill_scales,
                pill_pulses,
                pill_mouse_highlights,
            };
        }

        let content_w_px = pill_rects.last().map(|r| r.x + r.w).unwrap_or(0);
        let tray_w_raw = (content_w_px + scaled_px(TRAY_PAD_X) * 2).max(1);
        let tray_h = (content_h_px + scaled_px(TRAY_PAD_Y) * 2).max(1);

        let content_origin_x = scaled_px(TRAY_PAD_X);
        let content_origin_y = (tray_h - content_h_px) / 2;
        for rect in &mut pill_rects {
            rect.x += content_origin_x;
            rect.y = content_origin_y + (content_h_px - rect.h) / 2;
        }

        self.tray_state.pill_w_target = tray_w_raw as f32 / px_scale;
        let k = 20.0;
        let factor = 1.0 - (-k * dt.max(0.0)).exp();
        self.tray_state.pill_w += (self.tray_state.pill_w_target - self.tray_state.pill_w) * factor;
        self.tray_state.tray_alpha += (1.0 - self.tray_state.tray_alpha) * factor;

        if single_tile_debug_enabled() {
            let labels: Vec<&str> = tokens.iter().map(|t| t.label.as_str()).collect();
            eprintln!(
                "[overlay-single] tokens={labels:?} row_w={} pill_target={:.1} pill_cur={:.1}",
                tray_w_raw, self.tray_state.pill_w_target, self.tray_state.pill_w,
            );
        }

        OverlayGeometry {
            width_px: (self.tray_state.pill_w * px_scale).round() as i32,
            height_px: tray_h,
            width_points: self.tray_state.pill_w,
            height_points: tray_h as f32 / px_scale,
            pill_rects,
            pill_labels,
            pill_is_mouse,
            pill_font_sizes,
            pill_alphas,
            pill_scales,
            pill_pulses,
            pill_mouse_highlights,
        }
    }

    fn maybe_apply_window_region(
        &mut self,
        overlay_title: &str,
        geometry: &OverlayGeometry,
        currently_visible: bool,
    ) {
        let now = Instant::now();

        if self.startup_experiment == overlay_startup_diagnostics::OverlayExperiment::DelayRegion {
            let until = self
                .startup_delay_until
                .get_or_insert_with(|| now + Duration::from_millis(200));
            if now < *until {
                overlay_startup_diagnostics::log_event(
                    "experiment C: delaying SetWindowRgn for 200ms",
                );
                return;
            }
        }

        if self.startup_experiment == overlay_startup_diagnostics::OverlayExperiment::NoRegion {
            overlay_startup_diagnostics::log_event("experiment E: skipping SetWindowRgn");
            return;
        }

        let width = geometry.width_px;
        let height = geometry.height_px;
        let radius =
            ((TRAY_RADIUS as f32) * self.draft.overlay_scale.clamp(0.6, 2.0)).round() as i32;
        let desired = (width, height, radius);

        if self.pending_region != Some(desired) {
            self.pending_region = Some(desired);
            self.region_settle_deadline = Some(now + self.region_debounce);
            overlay_startup_diagnostics::log_event(format!(
                "region scheduler queued {}x{} radius={} settle={}ms",
                width,
                height,
                radius,
                self.region_debounce.as_millis()
            ));
        }

        if let Some(deadline) = self.region_settle_deadline {
            if now < deadline {
                return;
            }
        }

        if self.last_region_geometry.as_ref() == Some(geometry) {
            return;
        }

        let redraw = self.startup_experiment
            != overlay_startup_diagnostics::OverlayExperiment::RegionNoRedraw;

        if self.last_hwnd.is_none() {
            self.last_hwnd = find_hwnd_by_title(overlay_title);
        }

        if self.last_hwnd.is_some_and(|hwnd| !hwnd_is_valid(hwnd)) {
            overlay_startup_diagnostics::log_event("cached HWND invalid; reacquiring");
            self.last_hwnd = None;
            self.transitions_disabled = false;
        }

        let hwnd = self.last_hwnd;

        if let Some(hwnd_val) = hwnd {
            overlay_startup_diagnostics::log_event(format!(
                "region apply path hwnd={hwnd_val:?} currently_visible={currently_visible}"
            ));
            snapshot_hwnd_state(hwnd_val, "after-region-apply");

            let skip_dwm = self.startup_experiment
                == overlay_startup_diagnostics::OverlayExperiment::NoDwmDisable;
            if !self.transitions_disabled && !skip_dwm {
                if let Err(err) = disable_dwm_transitions(hwnd_val) {
                    eprintln!("[overlay-region] failed to disable DWM transitions: {err}");
                } else {
                    self.transitions_disabled = true;
                }
            } else if skip_dwm {
                overlay_startup_diagnostics::log_event(
                    "experiment F: skipping DwmSetWindowAttribute",
                );
            }

            let region_ok =
                apply_tray_region_hwnd_with_redraw(hwnd_val, width, height, radius, false);

            if region_ok {
                self.region_redraw_needed = true;
                self.region_settle_deadline = None;
                self.pending_region = None;

                if redraw || self.region_redraw_needed {
                    force_redraw(hwnd_val);
                    self.region_redraw_needed = false;
                }

                self.last_hwnd = Some(hwnd_val);
                self.startup_region_prepared = true;
                self.last_region_geometry = Some(geometry.clone());
            } else {
                overlay_startup_diagnostics::log_event(
                    "SetWindowRgn failed; dropping cached HWND and retrying with fresh handle",
                );
                self.last_hwnd = None;
                self.transitions_disabled = false;
                self.startup_region_prepared = false;
                self.last_region_geometry = None;
                self.region_settle_deadline = Some(now + self.region_debounce);
                self.pending_region = Some(desired);
            }
        } else {
            overlay_startup_diagnostics::log_event("FindWindowW did not return overlay HWND yet");
            self.last_region_geometry = None;
        }
    }

    fn maybe_log_hwnd_state(&mut self) {
        let observed = find_hwnd_by_title(&overlay_viewport_title());
        if let (Some(old), Some(new)) = (self.last_hwnd, observed) {
            if old != new {
                overlay_startup_diagnostics::log_event(format!(
                    "RECREATED hwnd old={old:?} new={new:?}; rebinding and re-preparing"
                ));
                snapshot_hwnd_state(old, "recreated-old");
                snapshot_hwnd_state(new, "recreated-new");
                self.transitions_disabled = false;
                self.startup_region_prepared = false;
                self.pending_region = None;
                self.region_settle_deadline = None;
                self.region_redraw_needed = false;
                self.first_show_done = false;
            }
        }
        if observed.is_some() {
            self.last_hwnd = observed;
        }

        if let Some(hwnd) = self.last_hwnd {
            snapshot_hwnd_state(hwnd, "frame-snapshot");
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Rgba::from_rgb(1.0, 1.0, 1.0).to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.first_frame_logged {
            overlay_startup_diagnostics::log_event("first update/frame");
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
                overlay_startup_diagnostics::log_event(format!(
                    "first input event received: {event:?}"
                ));
                self.first_input_logged = true;
            }
            let now = Instant::now();
            self.tray_on_event(&event, now);
            self.input_state.apply_event(event);
        }

        *self.config.lock().unwrap() = self.draft.clone();

        let screen_rect = ctx.input(|i| i.screen_rect());
        overlay_startup_diagnostics::log_event(format!(
            "frame input screen_rect={}x{} px_per_point={:.3}",
            screen_rect.width(),
            screen_rect.height(),
            ctx.pixels_per_point()
        ));
        if overlay_startup_diagnostics::enabled() {
            ctx.input(|i| {
                for ev in &i.events {
                    overlay_startup_diagnostics::log_event(format!("egui event: {ev:?}"));
                }
            });
        }
        if screen_rect.width() > 100.0 && screen_rect.height() > 100.0 {
            self.screen_size = [screen_rect.width(), screen_rect.height()];
        }

        settings_window::draw_settings(ctx, self);

        if self.draft.overlay_enabled {
            let now = Instant::now();
            let dt = 1.0 / 60.0;
            let overlay_hold_for = Duration::from_secs_f32(
                (self.draft.display_duration_secs + self.draft.fade_duration_secs).max(0.1),
            );
            let mouse_hold_for = overlay_hold_for;
            let display_duration =
                Duration::from_secs_f32(self.draft.display_duration_secs.max(0.0));
            let fade_duration = Duration::from_secs_f32(self.draft.fade_duration_secs.max(0.0));
            self.input_state.tick(now, mouse_hold_for, overlay_hold_for);

            let render_tokens = self.build_render_tokens(now, dt);
            let mut overlay_visible =
                !render_tokens.is_empty() || self.tray_state.tray_alpha > 0.01;
            if self.manual_preview_until.is_some_and(|until| now <= until)
                && self.active_tab == settings_window::SettingsTab::Position
                && self.draft.position == OverlayPosition::Manual
            {
                overlay_visible = true;
            }
            let scroll_arrow = self.input_state.scroll_arrow_for_display(
                now,
                display_duration,
                fade_duration,
                self.draft.show_mouse_icon,
                self.draft.show_scroll,
            );

            let cfg = self.draft.clone();
            let geometry = self.build_overlay_geometry(ctx, &render_tokens, dt);

            let computed_win_pos = self.compute_overlay_position(
                [geometry.width_points, geometry.height_points],
                self.screen_size,
            );
            let mut win_pos = if self.startup_experiment
                == overlay_startup_diagnostics::OverlayExperiment::NoAutoPosition
            {
                if self.fixed_experiment_pos.is_none() {
                    overlay_startup_diagnostics::log_event(
                        "experiment no_autoposition: freezing initial overlay position",
                    );
                    self.fixed_experiment_pos = Some(computed_win_pos);
                }
                self.fixed_experiment_pos.unwrap_or(computed_win_pos)
            } else {
                computed_win_pos
            };

            let overlay_id = egui::ViewportId::from_hash_of("overlay");
            let overlay_title = overlay_viewport_title();
            // Default to hide-until-ready on first activation to avoid user-visible
            // intermediate geometry/style/region transitions.
            let startup_hidden_mode = true;
            let should_show_overlay = overlay_visible;

            if should_show_overlay {
                if self.tray_state.left_anchor.is_none() {
                    self.tray_state.left_anchor = Some(win_pos);
                }
                if let Some(anchor) = self.tray_state.left_anchor {
                    win_pos = anchor;
                }
            } else {
                self.tray_state.left_anchor = None;
            }

            if self.startup_experiment
                == overlay_startup_diagnostics::OverlayExperiment::EarlyDwmDisable
                && !self.transitions_disabled
            {
                if let Some(hwnd) = find_hwnd_by_title(&overlay_title) {
                    overlay_startup_diagnostics::log_event(
                        "experiment A: disable DWM transitions as soon as HWND exists",
                    );
                    if disable_dwm_transitions(hwnd).is_ok() {
                        self.transitions_disabled = true;
                        self.last_hwnd = Some(hwnd);
                    }
                }
            }

            let overlay_visible = if startup_hidden_mode {
                should_show_overlay && self.startup_region_prepared && self.first_show_done
            } else {
                should_show_overlay
            };

            if self.last_overlay_visible != overlay_visible {
                overlay_startup_diagnostics::log_event(format!(
                    "visibility toggle {} -> {} (requested={should_show_overlay})",
                    self.last_overlay_visible, overlay_visible
                ));
                self.last_overlay_visible = overlay_visible;
            }

            if !should_show_overlay {
                overlay_startup_diagnostics::log_event("Hide requested (overlay not active)");
                // Keep the overlay viewport alive between activations to avoid
                // destroy/recreate startup animations. We hide it instead of
                // closing it so the next activation reuses the same window.
                ctx.send_viewport_cmd_to(overlay_id, egui::ViewportCommand::Visible(false));
                self.first_show_done = false;
            } else {
                let requested_inner = egui::vec2(geometry.width_points, geometry.height_points);
                if self.last_sent_inner_size != Some(requested_inner) {
                    ctx.send_viewport_cmd_to(
                        overlay_id,
                        egui::ViewportCommand::InnerSize(requested_inner),
                    );
                    self.last_sent_inner_size = Some(requested_inner);
                    if single_tile_debug_enabled() {
                        eprintln!(
                            "[overlay-tray] resize inner=({:.1},{:.1})",
                            requested_inner.x, requested_inner.y
                        );
                    }
                }
                if self.last_sent_outer_pos != Some(win_pos) {
                    ctx.send_viewport_cmd_to(
                        overlay_id,
                        egui::ViewportCommand::OuterPosition(win_pos),
                    );
                    self.last_sent_outer_pos = Some(win_pos);
                    if single_tile_debug_enabled() {
                        eprintln!(
                            "[overlay-tray] move x={:.1} y={:.1} w={:.1} h={:.1}",
                            win_pos.x, win_pos.y, requested_inner.x, requested_inner.y
                        );
                    }
                }
                overlay_startup_diagnostics::log_event(format!(
                    "startup geometry size_pt={:.1}x{:.1} size_px={}x{} pos=({:.1},{:.1})",
                    geometry.width_points,
                    geometry.height_points,
                    geometry.width_px,
                    geometry.height_px,
                    win_pos.x,
                    win_pos.y
                ));
                // Resize/reposition first, then apply region in window-local coordinates.
                self.maybe_apply_window_region(&overlay_title, &geometry, overlay_visible);

                if startup_hidden_mode && self.startup_region_prepared && !self.first_show_done {
                    if let Some(hwnd) = self.last_hwnd {
                        snapshot_hwnd_state(hwnd, "before-first-show");
                    }
                    overlay_startup_diagnostics::log_event("Show requested (first stable show)");
                    ctx.send_viewport_cmd_to(overlay_id, egui::ViewportCommand::Visible(true));
                    self.first_show_done = true;
                    if let Some(hwnd) = self.last_hwnd {
                        snapshot_hwnd_state(hwnd, "after-first-show");
                        force_redraw(hwnd);
                        snapshot_hwnd_state(hwnd, "after-first-redraw");
                    }
                }

                if self.startup_experiment
                    == overlay_startup_diagnostics::OverlayExperiment::HiddenUntilReady
                    && !self.startup_region_prepared
                {
                    overlay_startup_diagnostics::log_event(
                        "experiment D: keep hidden until region prepared",
                    );
                    overlay_startup_diagnostics::log_event("Hide requested (hidden_until_ready)");
                    ctx.send_viewport_cmd_to(overlay_id, egui::ViewportCommand::Visible(false));
                } else if self.first_show_done {
                    overlay_startup_diagnostics::log_event("Show requested");
                    ctx.send_viewport_cmd_to(
                        overlay_id,
                        egui::ViewportCommand::Visible(overlay_visible),
                    );
                }

                if overlay_startup_diagnostics::enabled() {
                    self.maybe_log_hwnd_state();
                }
            }

            if should_show_overlay {
                let transparent = if self.startup_experiment
                    == overlay_startup_diagnostics::OverlayExperiment::NoTransparencyUntilReady
                {
                    self.startup_region_prepared
                } else {
                    true
                };

                if !self.viewport_config_logged {
                    overlay_startup_diagnostics::log_event(format!(
                        "viewport config title={OVERLAY_VIEWPORT_TITLE} decorations=false transparent={transparent} always_on_top=true"
                    ));
                    self.viewport_config_logged = true;
                }

                if !self.title_logged {
                    overlay_startup_diagnostics::log_event(
                        "overlay title assigned in ViewportBuilder",
                    );
                    self.title_logged = true;
                }

                let tray_alpha = self.tray_state.tray_alpha;
                ctx.show_viewport_immediate(
                    overlay_id,
                    egui::ViewportBuilder::default()
                        .with_inner_size([geometry.width_points, geometry.height_points])
                        .with_position(win_pos)
                        .with_title(overlay_title.clone())
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
                        vis.panel_fill = egui::Color32::TRANSPARENT;
                        vis.window_fill = egui::Color32::TRANSPARENT;
                        vis.extreme_bg_color = egui::Color32::TRANSPARENT;
                        ctx.set_visuals(vis);

                        egui::CentralPanel::default()
                            .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
                            .show(ctx, |ui| {
                                let panel_rect = ui.available_rect_before_wrap();
                                let tray_rounding = egui::Rounding::same(
                                    (TRAY_RADIUS as f32 * cfg.overlay_scale.clamp(0.6, 2.0))
                                        / ctx.pixels_per_point(),
                                );
                                let tray_bg = egui::Color32::WHITE
                                    .linear_multiply(tray_alpha.clamp(0.0, 1.0));
                                if tray_alpha > 0.01 {
                                    ui.painter().rect_filled(panel_rect, tray_rounding, tray_bg);
                                }

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

                                for (
                                    (
                                        ((((rect, label), is_mouse), font_size), alpha),
                                        (scale, pulse),
                                    ),
                                    mouse_hl,
                                ) in geometry
                                    .pill_rects
                                    .iter()
                                    .zip(geometry.pill_labels.iter())
                                    .zip(geometry.pill_is_mouse.iter())
                                    .zip(geometry.pill_font_sizes.iter())
                                    .zip(geometry.pill_alphas.iter())
                                    .zip(
                                        geometry
                                            .pill_scales
                                            .iter()
                                            .zip(geometry.pill_pulses.iter()),
                                    )
                                    .zip(geometry.pill_mouse_highlights.iter())
                                {
                                    let base_rect = egui::Rect::from_min_size(
                                        egui::pos2(
                                            rect.x as f32 / px_scale,
                                            rect.y as f32 / px_scale,
                                        ),
                                        egui::vec2(
                                            rect.w as f32 / px_scale,
                                            rect.h as f32 / px_scale,
                                        ),
                                    );
                                    let pill_rect = egui::Rect::from_center_size(
                                        base_rect.center(),
                                        base_rect.size() * *scale,
                                    );
                                    let rounding =
                                        egui::Rounding::same(rect.radius as f32 / px_scale);
                                    let brighten = (*pulse * 40.0).round() as u8;
                                    let pulse_fill = egui::Color32::from_rgb(
                                        pill_fill.r().saturating_add(brighten),
                                        pill_fill.g().saturating_add(brighten),
                                        pill_fill.b().saturating_add(brighten),
                                    );
                                    let fill = pulse_fill.linear_multiply(*alpha);
                                    let text_color = pill_text.linear_multiply(*alpha);
                                    ui.painter().rect_filled(pill_rect, rounding, fill);
                                    if *is_mouse {
                                        draw_mouse_icon(
                                            ui.painter(),
                                            pill_rect,
                                            *mouse_hl,
                                            *alpha,
                                            scroll_arrow,
                                            &palette,
                                        );
                                    } else {
                                        ui.painter().text(
                                            pill_rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            label,
                                            egui::FontId::proportional(*font_size),
                                            text_color,
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
    overlay_startup_diagnostics::log_event(format!(
        "overlay diagnostics enabled; experiment={:?}",
        overlay_startup_diagnostics::OverlayExperiment::from_env()
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
