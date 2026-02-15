use eframe::egui;

use crate::theme::{apply_alpha, Palette};
use keyoverlay_input::MouseButton;

/// Which mouse button is currently highlighted (just clicked).
#[derive(Clone, Copy, PartialEq)]
pub enum MouseHighlight {
    None,
    Left,
    Right,
    Middle,
}

impl MouseHighlight {
    pub fn from_button(btn: MouseButton) -> Self {
        match btn {
            MouseButton::Left => Self::Left,
            MouseButton::Right => Self::Right,
            MouseButton::Middle => Self::Middle,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScrollArrowDirection {
    Up,
    Down,
}

/// Draw a stylized mouse icon with optional button highlight and scroll arrow overlay.
pub fn draw_mouse_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    highlight: MouseHighlight,
    alpha: f32,
    scroll_arrow: Option<(ScrollArrowDirection, f32)>,
    palette: &Palette,
) {
    let body_color = apply_alpha(palette.mouse_body, alpha);
    let outline_color = apply_alpha(palette.mouse_outline, alpha);
    let hl_color = apply_alpha(palette.mouse_highlight, alpha);

    // ── Mouse body (rounded rect) ──
    let icon_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(48.0, 68.0));
    let body =
        egui::Rect::from_min_size(icon_rect.min + egui::vec2(4.0, 4.0), egui::vec2(40.0, 60.0));

    painter.rect_filled(body, 16.0, body_color);
    painter.rect_stroke(body, 16.0, egui::Stroke::new(1.5, outline_color));

    // ── Divider lines (two vertical lines splitting top into 3 zones) ──
    let mid_x = body.center().x;
    let top_y = body.min.y;
    let div_y = body.min.y + 26.0; // button zone height

    // Left divider
    let left_div_x = mid_x - 6.0;
    painter.line_segment(
        [
            egui::pos2(left_div_x, top_y + 6.0),
            egui::pos2(left_div_x, div_y),
        ],
        egui::Stroke::new(1.0, outline_color),
    );

    // Right divider
    let right_div_x = mid_x + 6.0;
    painter.line_segment(
        [
            egui::pos2(right_div_x, top_y + 6.0),
            egui::pos2(right_div_x, div_y),
        ],
        egui::Stroke::new(1.0, outline_color),
    );

    // Horizontal divider below buttons
    painter.line_segment(
        [
            egui::pos2(body.min.x + 4.0, div_y),
            egui::pos2(body.max.x - 4.0, div_y),
        ],
        egui::Stroke::new(1.0, outline_color),
    );

    // ── Button highlights ──
    let btn_top = top_y + 2.0;
    let btn_bot = div_y;

    // Left button zone
    if highlight == MouseHighlight::Left {
        let left_rect = egui::Rect::from_min_max(
            egui::pos2(body.min.x + 2.0, btn_top),
            egui::pos2(left_div_x, btn_bot),
        );
        painter.rect_filled(
            left_rect,
            egui::Rounding {
                nw: 14.0,
                ne: 0.0,
                sw: 0.0,
                se: 0.0,
            },
            hl_color,
        );
    }

    // Middle button zone
    if highlight == MouseHighlight::Middle {
        let mid_rect = egui::Rect::from_min_max(
            egui::pos2(left_div_x, btn_top + 4.0),
            egui::pos2(right_div_x, btn_bot),
        );
        painter.rect_filled(mid_rect, 2.0, hl_color);
    }

    // Right button zone
    if highlight == MouseHighlight::Right {
        let right_rect = egui::Rect::from_min_max(
            egui::pos2(right_div_x, btn_top),
            egui::pos2(body.max.x - 2.0, btn_bot),
        );
        painter.rect_filled(
            right_rect,
            egui::Rounding {
                nw: 0.0,
                ne: 14.0,
                sw: 0.0,
                se: 0.0,
            },
            hl_color,
        );
    }

    // ── Scroll wheel indicator (small oval in center) ──
    let wheel_center = egui::pos2(mid_x, top_y + 14.0);
    painter.circle_stroke(wheel_center, 3.5, egui::Stroke::new(1.0, outline_color));

    if let Some((dir, arrow_alpha)) = scroll_arrow {
        let arrow_color = apply_alpha(palette.mouse_highlight, arrow_alpha.clamp(0.0, 1.0));

        // Keep arrow fully inside wheel column.
        let wheel_top = top_y + 6.0;
        let wheel_bottom = div_y - 2.0;
        let center_x = mid_x;
        let head_half_width = 3.2;
        let head_height = 4.0;
        let stroke_outer = egui::Stroke::new(2.2, arrow_color);
        let stroke_inner = egui::Stroke::new(1.2, arrow_color);

        let (tail_y, head_tip_y, head_base_y) = match dir {
            ScrollArrowDirection::Up => (
                wheel_bottom - 3.0,
                wheel_top + 2.0,
                wheel_top + 2.0 + head_height,
            ),
            ScrollArrowDirection::Down => (
                wheel_top + 3.0,
                wheel_bottom - 2.0,
                wheel_bottom - 2.0 - head_height,
            ),
        };

        // Shaft (double-stroke to create outlined/thicker visual)
        painter.line_segment(
            [
                egui::pos2(center_x, tail_y),
                egui::pos2(center_x, head_base_y),
            ],
            stroke_outer,
        );
        painter.line_segment(
            [
                egui::pos2(center_x, tail_y),
                egui::pos2(center_x, head_base_y),
            ],
            stroke_inner,
        );

        // Chevron head
        painter.line_segment(
            [
                egui::pos2(center_x - head_half_width, head_base_y),
                egui::pos2(center_x, head_tip_y),
            ],
            stroke_outer,
        );
        painter.line_segment(
            [
                egui::pos2(center_x + head_half_width, head_base_y),
                egui::pos2(center_x, head_tip_y),
            ],
            stroke_outer,
        );

        // Filled inner triangle for stronger arrowhead readability.
        painter.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(center_x - (head_half_width - 0.8), head_base_y),
                egui::pos2(center_x + (head_half_width - 0.8), head_base_y),
                egui::pos2(center_x, head_tip_y),
            ],
            arrow_color,
            egui::Stroke::NONE,
        ));
    }
}
