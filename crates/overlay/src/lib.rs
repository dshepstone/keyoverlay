use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::Result;
use eframe::egui;
use keyoverlay_input::InputEvent;

/// How long an event stays visible before starting to fade.
const EVENT_DISPLAY_SECS: f64 = 2.0;
/// How long the fade-out animation lasts.
const EVENT_FADE_SECS: f64 = 0.5;
/// Total lifetime = display + fade.
const EVENT_LIFETIME_SECS: f64 = EVENT_DISPLAY_SECS + EVENT_FADE_SECS;
/// Maximum events to keep in the display queue.
const MAX_EVENTS: usize = 8;
/// Width of the overlay popup.
const OVERLAY_WIDTH: f32 = 360.0;
/// Height of the overlay popup.
const OVERLAY_HEIGHT: f32 = 280.0;

// ── Color Palette ────────────────────────────────────────────────────────

const BG_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(25, 25, 35, 230);
const KEY_BG_COLOR: egui::Color32 = egui::Color32::from_rgb(55, 55, 75);
const KEY_TEXT_COLOR: egui::Color32 = egui::Color32::from_rgb(240, 240, 250);
const MOD_BG_COLOR: egui::Color32 = egui::Color32::from_rgb(80, 120, 200);
const MOD_TEXT_COLOR: egui::Color32 = egui::Color32::WHITE;
const MOUSE_BG_COLOR: egui::Color32 = egui::Color32::from_rgb(200, 80, 100);
const MOUSE_TEXT_COLOR: egui::Color32 = egui::Color32::WHITE;
const SCROLL_BG_COLOR: egui::Color32 = egui::Color32::from_rgb(80, 170, 120);
const SCROLL_TEXT_COLOR: egui::Color32 = egui::Color32::WHITE;
const SEPARATOR_COLOR: egui::Color32 = egui::Color32::from_rgb(60, 60, 80);
const TITLE_COLOR: egui::Color32 = egui::Color32::from_rgb(140, 140, 170);

// ── Display Event ────────────────────────────────────────────────────────

#[derive(Clone)]
struct DisplayEvent {
    label: String,
    kind: EventKind,
    created: Instant,
}

#[derive(Clone, Copy, PartialEq)]
enum EventKind {
    /// Keystroke (possibly with modifiers).
    Key { has_modifiers: bool },
    /// Mouse click.
    Mouse,
    /// Scroll wheel.
    Scroll,
}

impl DisplayEvent {
    fn opacity(&self, now: Instant) -> f32 {
        let age = now.duration_since(self.created).as_secs_f64();
        if age < EVENT_DISPLAY_SECS {
            1.0
        } else if age < EVENT_LIFETIME_SECS {
            let fade_progress = (age - EVENT_DISPLAY_SECS) / EVENT_FADE_SECS;
            (1.0 - fade_progress as f32).max(0.0)
        } else {
            0.0
        }
    }

    fn is_expired(&self, now: Instant) -> bool {
        now.duration_since(self.created).as_secs_f64() > EVENT_LIFETIME_SECS
    }
}

// ── Overlay App ──────────────────────────────────────────────────────────

struct OverlayApp {
    rx: Receiver<InputEvent>,
    events: VecDeque<DisplayEvent>,
}

impl OverlayApp {
    fn new(rx: Receiver<InputEvent>) -> Self {
        Self {
            rx,
            events: VecDeque::with_capacity(MAX_EVENTS),
        }
    }

    fn push_event(&mut self, event: InputEvent) {
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

        if self.events.len() >= MAX_EVENTS {
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
        while self.events.front().is_some_and(|e| e.is_expired(now)) {
            self.events.pop_front();
        }
    }
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain incoming events.
        while let Ok(event) = self.rx.try_recv() {
            self.push_event(event);
        }

        // Remove expired events.
        self.prune_expired();

        // Configure dark visuals.
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG_COLOR;
        visuals.window_rounding = egui::Rounding::same(12.0);
        ctx.set_visuals(visuals);

        // Set base font sizes.
        let mut style = (*ctx.style()).clone();
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(18.0));
        ctx.set_style(style);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .fill(BG_COLOR)
                    .inner_margin(egui::Margin::same(16.0))
                    .rounding(egui::Rounding::same(12.0)),
            )
            .show(ctx, |ui| {
                // ── Title bar ──
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("KeyOverlay")
                            .color(TITLE_COLOR)
                            .size(13.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("LIVE")
                                .color(egui::Color32::from_rgb(100, 220, 100))
                                .size(10.0)
                                .strong(),
                        );
                        // Pulsing dot indicator.
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        let pulse = ((ctx.input(|i| i.time) * 2.0).sin() * 0.5 + 0.5) as f32;
                        let dot_color = egui::Color32::from_rgba_unmultiplied(
                            100,
                            220,
                            100,
                            (150.0 + 105.0 * pulse) as u8,
                        );
                        ui.painter().circle_filled(rect.center(), 3.5, dot_color);
                    });
                });

                // Thin separator.
                ui.add_space(4.0);
                let sep_rect = ui.available_rect_before_wrap();
                let sep_rect =
                    egui::Rect::from_min_size(sep_rect.min, egui::vec2(sep_rect.width(), 1.0));
                ui.painter().rect_filled(sep_rect, 0.0, SEPARATOR_COLOR);
                ui.add_space(8.0);

                // ── Events list ──
                let now = Instant::now();

                if self.events.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label(
                            egui::RichText::new("Press any key or click...")
                                .color(egui::Color32::from_rgb(100, 100, 130))
                                .size(15.0)
                                .italics(),
                        );
                    });
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            // Show events newest-first (bottom to top visual, but we iterate
                            // in reverse and render top-down for natural scroll).
                            let events: Vec<_> = self.events.iter().rev().cloned().collect();
                            for event in &events {
                                let alpha = event.opacity(now);
                                if alpha <= 0.0 {
                                    continue;
                                }

                                ui.add_space(3.0);
                                draw_event_pill(ui, event, alpha);
                                ui.add_space(3.0);
                            }
                        });
                }
            });

        // Always request repaint for smooth animations.
        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

// ── Pill Rendering ──────────────────────────────────────────────────────

fn draw_event_pill(ui: &mut egui::Ui, event: &DisplayEvent, alpha: f32) {
    let apply_alpha = |c: egui::Color32, a: f32| -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (c.a() as f32 * a) as u8)
    };

    match event.kind {
        EventKind::Key { has_modifiers } => {
            if has_modifiers {
                draw_combo_pill(ui, &event.label, alpha);
            } else {
                draw_single_pill(
                    ui,
                    &event.label,
                    apply_alpha(KEY_BG_COLOR, alpha),
                    apply_alpha(KEY_TEXT_COLOR, alpha),
                    20.0,
                );
            }
        }
        EventKind::Mouse => {
            draw_single_pill(
                ui,
                &event.label,
                apply_alpha(MOUSE_BG_COLOR, alpha),
                apply_alpha(MOUSE_TEXT_COLOR, alpha),
                18.0,
            );
        }
        EventKind::Scroll => {
            draw_single_pill(
                ui,
                &event.label,
                apply_alpha(SCROLL_BG_COLOR, alpha),
                apply_alpha(SCROLL_TEXT_COLOR, alpha),
                16.0,
            );
        }
    }
}

/// Draw a single rounded pill with text.
fn draw_single_pill(
    ui: &mut egui::Ui,
    text: &str,
    bg: egui::Color32,
    fg: egui::Color32,
    font_size: f32,
) {
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_string(), egui::FontId::proportional(font_size), fg);

    let pill_size = egui::vec2(galley.size().x + 24.0, galley.size().y + 12.0);

    let (rect, _) = ui.allocate_exact_size(pill_size, egui::Sense::hover());

    ui.painter().rect_filled(rect, 8.0, bg);
    ui.painter()
        .galley(rect.min + egui::vec2(12.0, 6.0), galley, fg);
}

/// Draw a combo pill for modifier+key combos (e.g. "Ctrl + V").
/// Each segment gets its own colored badge joined with a "+" separator.
fn draw_combo_pill(ui: &mut egui::Ui, combo: &str, alpha: f32) {
    let apply_alpha = |c: egui::Color32, a: f32| -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (c.a() as f32 * a) as u8)
    };

    let parts: Vec<&str> = combo.split(" + ").collect();
    let font = egui::FontId::proportional(18.0);
    let plus_font = egui::FontId::proportional(14.0);

    // Measure total width.
    let mut total_width: f32 = 0.0;
    let mut galleys = Vec::new();

    for (i, part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;
        let fg = if is_last {
            apply_alpha(KEY_TEXT_COLOR, alpha)
        } else {
            apply_alpha(MOD_TEXT_COLOR, alpha)
        };

        let galley = ui
            .painter()
            .layout_no_wrap(part.to_string(), font.clone(), fg);

        let seg_width = galley.size().x + 16.0; // padding
        total_width += seg_width;
        galleys.push((galley, is_last));

        if !is_last {
            // Space for "+" separator
            total_width += 20.0;
        }
    }

    let row_height = 32.0;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(total_width, row_height), egui::Sense::hover());

    let mut x = rect.min.x;
    let y = rect.min.y;

    for (galley, is_last) in &galleys {
        let seg_width = galley.size().x + 16.0;
        let seg_rect =
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(seg_width, row_height));

        let bg = if *is_last {
            apply_alpha(KEY_BG_COLOR, alpha)
        } else {
            apply_alpha(MOD_BG_COLOR, alpha)
        };

        ui.painter().rect_filled(seg_rect, 6.0, bg);

        let text_y = y + (row_height - galley.size().y) / 2.0;
        ui.painter().galley(
            egui::pos2(x + 8.0, text_y),
            galley.clone(),
            egui::Color32::TRANSPARENT, // color already baked into galley
        );

        x += seg_width;

        if !is_last {
            // Draw "+" between segments.
            let plus_galley = ui.painter().layout_no_wrap(
                "+".to_string(),
                plus_font.clone(),
                apply_alpha(TITLE_COLOR, alpha),
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

// ── Public entry point ──────────────────────────────────────────────────

pub fn run(rx: Receiver<InputEvent>) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([OVERLAY_WIDTH, OVERLAY_HEIGHT])
            .with_decorations(false)
            .with_always_on_top()
            .with_resizable(false)
            .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "KeyOverlay",
        options,
        Box::new(|cc| {
            // Enable transparent background.
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Box::new(OverlayApp::new(rx))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
