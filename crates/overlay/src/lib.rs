mod mouse_icon;
mod overlay_window;
mod settings_window;
mod theme;

use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use anyhow::Result;
use eframe::egui;
use keyoverlay_core::{AppConfig, OverlayPosition, SharedConfig};
use keyoverlay_input::{InputEvent, MouseButton};

use mouse_icon::{draw_mouse_icon, MouseHighlight};
use overlay_window::{draw_event_pill, DisplayEvent, EventKind};
use theme::Palette;

const OVERLAY_VIEWPORT_TITLE: &str = "KeyOverlayOverlay";

// On Windows the overlay transparency is handled entirely by eframe's
// `.with_transparent(true)` and `.with_mouse_passthrough(true)`.
// Previous attempts to manually apply WS_EX_LAYERED + WS_EX_TRANSPARENT
// via Win32 APIs conflicted with eframe's own compositing and caused the
// window to be invisible on the desktop (only visible in the taskbar
// preview).  We intentionally leave this as a no-op now.
fn ensure_windows_overlay_transparency() {}

// ── Combined App ────────────────────────────────────────────────────────
// Single eframe application: settings is the main window, overlay is a
// child viewport shown/hidden via the on/off toggle.

struct App {
    // Settings state.
    config: SharedConfig,
    draft: AppConfig,
    active_tab: settings_window::SettingsTab,
    status_msg: Option<(String, Instant)>,

    // Overlay state.
    rx: Receiver<InputEvent>,
    events: VecDeque<DisplayEvent>,
    last_mouse_btn: Option<(MouseButton, Instant)>,

    /// Detected screen size (updated each frame from the main window context).
    screen_size: [f32; 2],
}

impl App {
    fn new(rx: Receiver<InputEvent>, config: SharedConfig) -> Self {
        let draft = config.lock().unwrap().clone();
        Self {
            config,
            draft,
            active_tab: settings_window::SettingsTab::Appearance,
            status_msg: None,
            rx,
            events: VecDeque::with_capacity(16),
            last_mouse_btn: None,
            screen_size: [1920.0, 1080.0],
        }
    }

    fn apply(&mut self) {
        *self.config.lock().unwrap() = self.draft.clone();
        self.draft.save();
        self.status_msg = Some(("Settings saved.".into(), Instant::now()));
    }

    fn reset_defaults(&mut self) {
        self.draft = AppConfig::default();
        self.apply();
    }

    fn push_event(&mut self, event: InputEvent) {
        let cfg = &self.draft;

        match &event {
            InputEvent::Key(_) if !cfg.show_keyboard => return,
            InputEvent::MouseClick(_) if !cfg.show_mouse_clicks => return,
            InputEvent::Scroll(_) if !cfg.show_scroll => return,
            _ => {}
        }

        if let InputEvent::MouseClick(mc) = &event {
            self.last_mouse_btn = Some((mc.button, Instant::now()));
        }

        let (label, kind) = match &event {
            InputEvent::Key(ke) => {
                let has_mods = !ke.modifiers.is_empty();
                (
                    event.display_string(),
                    EventKind::Key {
                        has_modifiers: has_mods,
                    },
                )
            }
            InputEvent::MouseClick(_) => (event.display_string(), EventKind::Mouse),
            InputEvent::Scroll(_) => (event.display_string(), EventKind::Scroll),
        };

        let max = self.draft.max_visible_events;
        while self.events.len() >= max {
            self.events.pop_front();
        }

        self.events.push_back(DisplayEvent {
            label,
            kind,
            created: Instant::now(),
        });
    }

    fn prune_expired(&mut self) {
        let now = Instant::now();
        let d = self.draft.display_duration_secs as f64;
        let f = self.draft.fade_duration_secs as f64;
        while self.events.front().is_some_and(|e| e.is_expired(now, d, f)) {
            self.events.pop_front();
        }
    }

    fn mouse_highlight(&self) -> MouseHighlight {
        let cfg = &self.draft;
        if let Some((btn, t)) = self.last_mouse_btn {
            let age = Instant::now().duration_since(t).as_secs_f64();
            if age < (cfg.display_duration_secs + cfg.fade_duration_secs) as f64 {
                return MouseHighlight::from_button(btn);
            }
        }
        MouseHighlight::None
    }

    /// Compute the overlay window position based on config and actual screen size.
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

    fn mouse_alpha(&self) -> f32 {
        let cfg = &self.draft;
        if let Some((_btn, t)) = self.last_mouse_btn {
            let age = Instant::now().duration_since(t).as_secs_f64();
            let d = cfg.display_duration_secs as f64;
            let f = cfg.fade_duration_secs as f64;
            if age < d {
                return 1.0;
            } else if age < d + f {
                return (1.0 - ((age - d) / f) as f32).max(0.0);
            }
        }
        0.4
    }
}

/// Padding around the content inside the rounded overlay background.
const OVERLAY_PADDING: f32 = 10.0;

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Always drain input events (even when overlay is off, so the
        // channel doesn't fill up).
        while let Ok(event) = self.rx.try_recv() {
            if self.draft.overlay_enabled {
                self.push_event(event);
            }
        }
        self.prune_expired();

        // Keep live config in sync (overlay reads from draft directly).
        *self.config.lock().unwrap() = self.draft.clone();

        // ── Detect screen size from the main window ──
        let screen_rect = ctx.input(|i| i.screen_rect());
        if screen_rect.width() > 100.0 && screen_rect.height() > 100.0 {
            self.screen_size = [screen_rect.width(), screen_rect.height()];
        }

        // ── Render settings (main window) ──
        settings_window::draw_settings(ctx, self);

        // ── Render overlay (child viewport) ──
        // Always show the overlay when enabled so the mouse icon and
        // keystrokes are visible.
        if self.draft.overlay_enabled {
            let cfg = self.draft.clone();
            let palette = Palette::from_config(&cfg);
            let events: Vec<DisplayEvent> = self.events.iter().rev().cloned().collect();
            let mouse_hl = self.mouse_highlight();
            let mouse_a = self.mouse_alpha();

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

                    // Override visuals so the viewport background is transparent;
                    // we paint our own rounded background below.
                    let mut vis = egui::Visuals::dark();
                    vis.panel_fill = egui::Color32::TRANSPARENT;
                    vis.window_fill = egui::Color32::TRANSPARENT;
                    vis.extreme_bg_color = egui::Color32::TRANSPARENT;
                    ctx.set_visuals(vis);

                    egui::CentralPanel::default()
                        .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
                        .show(ctx, |ui| {
                            let now = Instant::now();
                            let d = cfg.display_duration_secs as f64;
                            let f = cfg.fade_duration_secs as f64;
                            let rounding = cfg.pill_rounding;
                            let opacity = cfg.overlay_opacity;
                            let panel_rect = ui.available_rect_before_wrap();

                            // Draw a rounded semi-transparent background behind
                            // all content so the overlay is always visible.
                            let bg_alpha =
                                (cfg.background_opacity * opacity * 255.0) as u8;
                            let bg_color =
                                egui::Color32::from_rgba_unmultiplied(30, 30, 40, bg_alpha);
                            ui.painter().rect_filled(
                                panel_rect,
                                rounding + 4.0,
                                bg_color,
                            );

                            // Content area inside padding.
                            let content_rect = panel_rect.shrink(OVERLAY_PADDING);
                            let mut content_ui = ui.child_ui(
                                content_rect,
                                egui::Layout::left_to_right(egui::Align::Min),
                            );

                            content_ui.horizontal(|ui| {
                                if cfg.show_mouse_icon {
                                    let a = mouse_a * opacity;
                                    draw_mouse_icon(ui, mouse_hl, a, &palette);
                                    ui.add_space(12.0);
                                }

                                ui.vertical(|ui| {
                                    for event in &events {
                                        let alpha = event.opacity(now, d, f) * opacity;
                                        if alpha <= 0.0 {
                                            continue;
                                        }
                                        ui.add_space(2.0);
                                        draw_event_pill(
                                            ui,
                                            event,
                                            alpha,
                                            &palette,
                                            rounding,
                                            cfg.font_size,
                                        );
                                        ui.add_space(2.0);
                                    }
                                });
                            });
                        });
                },
            );
        }

        // Repaint for animations.
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}

// ── Public entry point ──────────────────────────────────────────────────

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
