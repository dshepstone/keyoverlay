use eframe::egui;

use crate::theme::{apply_alpha, Palette};

pub fn draw_event_pill(
    ui: &mut egui::Ui,
    label: &str,
    alpha: f32,
    bg: egui::Color32,
    fg: egui::Color32,
    rounding: f32,
    font_size: f32,
) {
    draw_single_pill(
        ui,
        label,
        apply_alpha(bg, alpha),
        apply_alpha(fg, alpha),
        font_size,
        rounding,
    );
}

pub fn draw_chord_pill(
    ui: &mut egui::Ui,
    chord: &str,
    alpha: f32,
    palette: &Palette,
    rounding: f32,
    font_size: f32,
) {
    let parts: Vec<&str> = chord.split(" + ").collect();
    let font = egui::FontId::proportional(font_size);
    let plus_font = egui::FontId::proportional(font_size - 6.0);

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

        let seg_width = galley.size().x + 24.0;
        total_width += seg_width;
        galleys.push((galley, is_last));

        if !is_last {
            total_width += 26.0;
        }
    }

    let row_height = font_size + 20.0;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(total_width, row_height), egui::Sense::hover());

    let mut x = rect.min.x;
    let y = rect.min.y;

    for (galley, is_last) in &galleys {
        let seg_width = galley.size().x + 24.0;
        let seg_rect =
            egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(seg_width, row_height));

        let bg = if *is_last {
            apply_alpha(palette.key_bg, alpha)
        } else {
            apply_alpha(palette.mod_bg, alpha)
        };

        ui.painter().rect_filled(seg_rect, rounding, bg);

        let text_y = y + (row_height - galley.size().y) / 2.0;
        ui.painter().galley(
            egui::pos2(x + 12.0, text_y),
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
                egui::pos2(x + 8.0, plus_y),
                plus_galley,
                egui::Color32::TRANSPARENT,
            );
            x += 26.0;
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

    let pill_size = egui::vec2(galley.size().x + 32.0, galley.size().y + 16.0);
    let (rect, _) = ui.allocate_exact_size(pill_size, egui::Sense::hover());

    ui.painter().rect_filled(rect, rounding, bg);
    ui.painter()
        .galley(rect.min + egui::vec2(16.0, 8.0), galley, fg);
}
