use std::time::Instant;

use eframe::egui;

use crate::theme::{apply_alpha, Palette};

// ── Display Event ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct DisplayEvent {
    pub label: String,
    pub kind: EventKind,
    pub created: Instant,
}

#[derive(Clone, Copy, PartialEq)]
pub enum EventKind {
    Key { has_modifiers: bool },
    Mouse,
    Scroll,
}

impl DisplayEvent {
    pub fn opacity(&self, now: Instant, display_secs: f64, fade_secs: f64) -> f32 {
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

    pub fn is_expired(&self, now: Instant, display_secs: f64, fade_secs: f64) -> bool {
        now.duration_since(self.created).as_secs_f64() > display_secs + fade_secs
    }
}

// ── Pill Rendering ──────────────────────────────────────────────────────

pub fn draw_event_pill(
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
