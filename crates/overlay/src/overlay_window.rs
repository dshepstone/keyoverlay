use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui;
use keyoverlay_core::{AppConfig, SharedConfig};
use keyoverlay_input::{InputEvent, MouseButton};

use crate::mouse_icon::{draw_mouse_icon, MouseHighlight};
use crate::theme::{apply_alpha, Palette};

// ── Display Event ────────────────────────────────────────────────────────

#[derive(Clone)]
struct DisplayEvent {
    label: String,
    kind: EventKind,
    created: Instant,
}

#[derive(Clone, Copy, PartialEq)]
enum EventKind {
    Key { has_modifiers: bool },
    Mouse,
    Scroll,
}

impl DisplayEvent {
    fn opacity(&self, now: Instant, display_secs: f64, fade_secs: f64) -> f32 {
        let lifetime = display_secs + fade_secs;
        let age = now.duration_since(self.created).as_secs_f64();
        if age < display_secs {
            1.0
        } else if age < lifetime {
            let fade_progress = (age - display_secs) / fade_secs;
            (1.0 - fade_progress as f32).max(0.0)
        } else {
            0.0
        }
    }

    fn is_expired(&self, now: Instant, display_secs: f64, fade_secs: f64) -> bool {
        now.duration_since(self.created).as_secs_f64() > display_secs + fade_secs
    }
}

// ── Overlay App ──────────────────────────────────────────────────────────

pub struct OverlayApp {
    rx: Receiver<InputEvent>,
    config: SharedConfig,
    events: VecDeque<DisplayEvent>,
    last_mouse_btn: Option<(MouseButton, Instant)>,
}

impl OverlayApp {
    pub fn new(rx: Receiver<InputEvent>, config: SharedConfig) -> Self {
        Self {
            rx,
            config,
            events: VecDeque::with_capacity(16),
            last_mouse_btn: None,
        }
    }

    fn push_event(&mut self, event: InputEvent, cfg: &AppConfig) {
        // Filter by config toggles.
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

        let max = cfg.max_visible_events;
        while self.events.len() >= max {
            self.events.pop_front();
        }

        self.events.push_back(DisplayEvent {
            label,
            kind,
            created: Instant::now(),
        });
    }

    fn prune_expired(&mut self, cfg: &AppConfig) {
        let now = Instant::now();
        let d = cfg.display_duration_secs as f64;
        let f = cfg.fade_duration_secs as f64;
        while self.events.front().is_some_and(|e| e.is_expired(now, d, f)) {
            self.events.pop_front();
        }
    }

    fn mouse_highlight(&self, cfg: &AppConfig) -> MouseHighlight {
        if let Some((btn, t)) = self.last_mouse_btn {
            let age = Instant::now().duration_since(t).as_secs_f64();
            if age < (cfg.display_duration_secs + cfg.fade_duration_secs) as f64 {
                return MouseHighlight::from_button(btn);
            }
        }
        MouseHighlight::None
    }

    fn mouse_alpha(&self, cfg: &AppConfig) -> f32 {
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
        0.4 // dim when idle
    }
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let cfg = self.config.lock().unwrap().clone();

        // Drain incoming events.
        while let Ok(event) = self.rx.try_recv() {
            self.push_event(event, &cfg);
        }
        self.prune_expired(&cfg);

        let palette = Palette::from_config(&cfg);

        // Fully transparent visuals — no panel background at all.
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::TRANSPARENT;
        visuals.window_fill = egui::Color32::TRANSPARENT;
        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::proportional(cfg.font_size),
        );
        ctx.set_style(style);

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let now = Instant::now();
                let d = cfg.display_duration_secs as f64;
                let f = cfg.fade_duration_secs as f64;
                let rounding = cfg.pill_rounding;
                let opacity = cfg.overlay_opacity;

                ui.horizontal(|ui| {
                    // ── Mouse icon (left side) ──
                    if cfg.show_mouse_icon {
                        let hl = self.mouse_highlight(&cfg);
                        let mouse_a = self.mouse_alpha(&cfg) * opacity;
                        draw_mouse_icon(ui, hl, mouse_a, &palette);
                        ui.add_space(12.0);
                    }

                    // ── Event pills ──
                    ui.vertical(|ui| {
                        if self.events.is_empty() {
                            // Nothing to show when idle — fully transparent.
                        } else {
                            let events: Vec<_> = self.events.iter().rev().cloned().collect();
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
                        }
                    });
                });
            });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

// ── Pill Rendering ──────────────────────────────────────────────────────

fn draw_event_pill(
    ui: &mut egui::Ui,
    event: &DisplayEvent,
    alpha: f32,
    palette: &Palette,
    rounding: f32,
    font_size: f32,
) {
    match event.kind {
        EventKind::Key { has_modifiers } => {
            if has_modifiers {
                draw_combo_pill(ui, &event.label, alpha, palette, rounding, font_size);
            } else {
                draw_single_pill(
                    ui,
                    &event.label,
                    apply_alpha(palette.key_bg, alpha),
                    apply_alpha(palette.key_fg, alpha),
                    font_size + 2.0,
                    rounding,
                );
            }
        }
        EventKind::Mouse => {
            draw_single_pill(
                ui,
                &event.label,
                apply_alpha(palette.mouse_bg, alpha),
                apply_alpha(palette.mouse_fg, alpha),
                font_size,
                rounding,
            );
        }
        EventKind::Scroll => {
            draw_single_pill(
                ui,
                &event.label,
                apply_alpha(palette.scroll_bg, alpha),
                apply_alpha(palette.scroll_fg, alpha),
                font_size - 2.0,
                rounding,
            );
        }
    }
}

fn draw_single_pill(
    ui: &mut egui::Ui,
    text: &str,
    bg: egui::Color32,
    fg: egui::Color32,
    font_size: f32,
    rounding: f32,
) {
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_string(), egui::FontId::proportional(font_size), fg);

    let pill_size = egui::vec2(galley.size().x + 24.0, galley.size().y + 12.0);
    let (rect, _) = ui.allocate_exact_size(pill_size, egui::Sense::hover());

    ui.painter().rect_filled(rect, rounding, bg);
    ui.painter()
        .galley(rect.min + egui::vec2(12.0, 6.0), galley, fg);
}

fn draw_combo_pill(
    ui: &mut egui::Ui,
    combo: &str,
    alpha: f32,
    palette: &Palette,
    rounding: f32,
    font_size: f32,
) {
    let parts: Vec<&str> = combo.split(" + ").collect();
    let font = egui::FontId::proportional(font_size);
    let plus_font = egui::FontId::proportional(font_size - 4.0);

    let mut total_width: f32 = 0.0;
    let mut galleys = Vec::new();

    for (i, part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;
        let fg = if is_last {
            apply_alpha(palette.key_fg, alpha)
        } else {
            apply_alpha(palette.mod_fg, alpha)
        };

        let galley = ui
            .painter()
            .layout_no_wrap(part.to_string(), font.clone(), fg);

        let seg_width = galley.size().x + 16.0;
        total_width += seg_width;
        galleys.push((galley, is_last));

        if !is_last {
            total_width += 20.0;
        }
    }

    let row_height = font_size + 14.0;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(total_width, row_height), egui::Sense::hover());

    let mut x = rect.min.x;
    let y = rect.min.y;

    for (galley, is_last) in &galleys {
        let seg_width = galley.size().x + 16.0;
        let seg_rect =
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(seg_width, row_height));

        let bg = if *is_last {
            apply_alpha(palette.key_bg, alpha)
        } else {
            apply_alpha(palette.mod_bg, alpha)
        };

        ui.painter().rect_filled(seg_rect, rounding - 2.0, bg);

        let text_y = y + (row_height - galley.size().y) / 2.0;
        ui.painter().galley(
            egui::pos2(x + 8.0, text_y),
            galley.clone(),
            egui::Color32::TRANSPARENT,
        );

        x += seg_width;

        if !is_last {
            let plus_galley = ui.painter().layout_no_wrap(
                "+".to_string(),
                plus_font.clone(),
                apply_alpha(palette.title_fg, alpha),
            );
            let plus_y = y + (row_height - plus_galley.size().y) / 2.0;
            ui.painter().galley(
                egui::pos2(x + 6.0, plus_y),
                plus_galley,
                egui::Color32::TRANSPARENT,
            );
            x += 20.0;
        }
    }
}
