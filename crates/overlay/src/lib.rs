mod mouse_icon;
mod overlay_window;
mod settings_window;
mod theme;

use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::Instant;

use anyhow::Result;
use eframe::egui;
use keyoverlay_core::{AppConfig, SharedConfig};
use keyoverlay_input::{InputEvent, MouseButton};

use mouse_icon::{draw_mouse_icon, MouseHighlight};
use overlay_window::{draw_event_pill, DisplayEvent, EventKind};
use theme::Palette;

const OVERLAY_VIEWPORT_TITLE: &str = "KeyOverlayOverlay";

#[cfg(target_os = "windows")]
fn ensure_windows_overlay_transparency() {
    use std::sync::atomic::{AtomicBool, Ordering};

    use windows_sys::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
    use windows_sys::Win32::UI::Controls::MARGINS;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE,
        LWA_ALPHA, WS_EX_LAYERED, WS_EX_TRANSPARENT,
    };

    static APPLIED: AtomicBool = AtomicBool::new(false);
    if APPLIED.load(Ordering::Relaxed) {
        return;
    }

    let mut title_wide: Vec<u16> = OVERLAY_VIEWPORT_TITLE.encode_utf16().collect();
    title_wide.push(0);

    // Fallback for Windows where compositor path can still produce an opaque backdrop:
    // force layered+per-pixel alpha on the overlay HWND only.
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title_wide.as_ptr()) };
    if hwnd.is_null() {
        return;
    }

    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let layered_style = ex_style | WS_EX_LAYERED | WS_EX_TRANSPARENT;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, layered_style as isize);
        SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);

        // Extend the DWM frame to the whole client area to keep the window glass/transparent.
        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
    }

    APPLIED.store(true, Ordering::Relaxed);
}

#[cfg(not(target_os = "windows"))]
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

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Clear to alpha=0 so only explicitly painted widgets remain visible.
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

        // ── Render settings (main window) ──
        settings_window::draw_settings(ctx, self);

        // ── Render overlay (child viewport) ──
        if self.draft.overlay_enabled {
            let cfg = self.draft.clone();
            let palette = Palette::from_config(&cfg);
            let events: Vec<DisplayEvent> = self.events.iter().rev().cloned().collect();
            let mouse_hl = self.mouse_highlight();
            let mouse_a = self.mouse_alpha();
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("overlay"),
                egui::ViewportBuilder::default()
                    .with_inner_size([cfg.overlay_width, cfg.overlay_height])
                    .with_title(OVERLAY_VIEWPORT_TITLE)
                    .with_decorations(false)
                    .with_always_on_top()
                    .with_resizable(false)
                    .with_transparent(true) // Request per-pixel alpha for the overlay viewport.
                    .with_mouse_passthrough(true),
                move |ctx, _class| {
                    ensure_windows_overlay_transparency();

                    let overlay_size = egui::vec2(cfg.overlay_width, cfg.overlay_height);

                    egui::CentralPanel::default()
                        // Keep the overlay viewport fully transparent outside explicit widgets.
                        .frame(egui::Frame::none().fill(egui::Color32::TRANSPARENT))
                        .show(ctx, |ui| {
                            let now = Instant::now();
                            let d = cfg.display_duration_secs as f64;
                            let f = cfg.fade_duration_secs as f64;
                            let rounding = cfg.pill_rounding;
                            let opacity = cfg.overlay_opacity;

                            ui.set_min_size(overlay_size);
                            ui.horizontal(|ui| {
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
            // Required for transparent child viewports on eframe's native backends.
            // Without this, the renderer may composite with an opaque surface (black).
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
