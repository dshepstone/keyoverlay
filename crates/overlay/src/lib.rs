use std::collections::BTreeSet;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use eframe::egui;
use egui::{Color32, Rounding, Stroke, ViewportBuilder, ViewportClass, ViewportId};
use keyoverlay_input::{InputEvent, Key, MouseButton};

#[derive(Debug, Clone)]
struct OverlaySettings {
    show_keyboard: bool,
    show_mouse: bool,
    x: f32,
    y: f32,
    visible_seconds: f32,
    popup_scale: f32,
    click_through: bool,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            show_keyboard: true,
            show_mouse: true,
            x: 1420.0,
            y: 900.0,
            visible_seconds: 1.8,
            popup_scale: 1.0,
            click_through: true,
        }
    }
}

#[derive(Debug, Clone)]
struct LiveInputState {
    keys_down: BTreeSet<Key>,
    mouse_down: BTreeSet<MouseButton>,
    last_change: Instant,
    last_non_modifier: Option<Key>,
}

impl LiveInputState {
    fn new() -> Self {
        Self {
            keys_down: BTreeSet::new(),
            mouse_down: BTreeSet::new(),
            last_change: Instant::now(),
            last_non_modifier: None,
        }
    }

    fn update_from_event(&mut self, event: InputEvent, settings: &OverlaySettings) {
        match event {
            InputEvent::KeyDown { key, .. } if settings.show_keyboard => {
                self.keys_down.insert(key);
                if !is_modifier(key) {
                    self.last_non_modifier = Some(key);
                }
                self.last_change = Instant::now();
            }
            InputEvent::KeyUp { key } if settings.show_keyboard => {
                self.keys_down.remove(&key);
                if self.last_non_modifier == Some(key) {
                    self.last_non_modifier = None;
                }
                self.last_change = Instant::now();
            }
            InputEvent::MouseDown { button } if settings.show_mouse => {
                self.mouse_down.insert(button);
                self.last_change = Instant::now();
            }
            InputEvent::MouseUp { button } if settings.show_mouse => {
                self.mouse_down.remove(&button);
                self.last_change = Instant::now();
            }
            _ => {}
        }
    }

    fn has_pressed_input(&self) -> bool {
        !(self.keys_down.is_empty() && self.mouse_down.is_empty())
    }

    fn overlay_alpha(&self, visible_seconds: f32, now: Instant) -> f32 {
        if self.has_pressed_input() {
            return 1.0;
        }

        let visible_duration = Duration::from_secs_f32(visible_seconds.max(0.05));
        let elapsed = now.saturating_duration_since(self.last_change);
        if elapsed >= visible_duration {
            0.0
        } else {
            1.0 - elapsed.as_secs_f32() / visible_duration.as_secs_f32()
        }
    }
}

fn is_modifier(key: Key) -> bool {
    matches!(key, Key::Ctrl | Key::Shift | Key::Alt | Key::Win)
}

fn modifier_rank(key: Key) -> Option<usize> {
    match key {
        Key::Ctrl => Some(0),
        Key::Shift => Some(1),
        Key::Alt => Some(2),
        Key::Win => Some(3),
        _ => None,
    }
}

fn format_chord(state: &LiveInputState) -> Vec<String> {
    let mut labels = Vec::new();

    let mut modifiers: Vec<Key> = state
        .keys_down
        .iter()
        .copied()
        .filter(|k| is_modifier(*k))
        .collect();
    modifiers.sort_by_key(|k| modifier_rank(*k).unwrap_or(usize::MAX));
    labels.extend(modifiers.into_iter().map(|k| k.to_string()));

    let mut non_modifiers: Vec<Key> = state
        .keys_down
        .iter()
        .copied()
        .filter(|k| !is_modifier(*k))
        .collect();

    if let Some(last) = state.last_non_modifier {
        if let Some(pos) = non_modifiers.iter().position(|k| *k == last) {
            let promoted = non_modifiers.remove(pos);
            non_modifiers.insert(0, promoted);
        }
    }

    labels.extend(non_modifiers.into_iter().map(|k| k.to_string()));
    labels.extend(state.mouse_down.iter().map(|button| button.to_string()));

    labels
}

struct OverlayApp {
    rx: Receiver<InputEvent>,
    live_state: LiveInputState,
    settings: OverlaySettings,
}

impl OverlayApp {
    fn new(rx: Receiver<InputEvent>) -> Self {
        Self {
            rx,
            live_state: LiveInputState::new(),
            settings: OverlaySettings::default(),
        }
    }

    fn draw_control_window(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.heading("KeyOverlay Control Menu");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Capture Filters");
            ui.checkbox(&mut self.settings.show_keyboard, "Show keyboard events");
            ui.checkbox(&mut self.settings.show_mouse, "Show mouse button events");
            ui.checkbox(
                &mut self.settings.click_through,
                "Click-through overlay (passes mouse clicks through)",
            );

            ui.separator();
            ui.label("On-screen popup placement");
            ui.add(egui::Slider::new(&mut self.settings.x, 0.0..=3840.0).text("X"));
            ui.add(egui::Slider::new(&mut self.settings.y, 0.0..=2160.0).text("Y"));
            if ui
                .button("Reset overlay position (bottom-right default)")
                .clicked()
            {
                self.settings.x = 1420.0;
                self.settings.y = 900.0;
            }

            ui.separator();
            ui.label("Popup behavior");
            ui.add(
                egui::Slider::new(&mut self.settings.visible_seconds, 0.1..=6.0)
                    .text("Visible seconds"),
            );
            ui.add(egui::Slider::new(&mut self.settings.popup_scale, 0.7..=2.5).text("Scale"));

            ui.separator();
            let pressed_labels = format_chord(&self.live_state);
            if pressed_labels.is_empty() {
                ui.label("Pressed state preview: <none>");
            } else {
                ui.label(format!(
                    "Pressed state preview: {}",
                    pressed_labels.join(" + ")
                ));
            }
        });
    }

    fn draw_overlay_viewport(&self, ctx: &egui::Context) {
        let now = Instant::now();
        let alpha = self
            .live_state
            .overlay_alpha(self.settings.visible_seconds, now);
        let visible = alpha > 0.01;
        let labels = format_chord(&self.live_state);

        let builder = ViewportBuilder::default()
            .with_position(egui::pos2(self.settings.x, self.settings.y))
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_mouse_passthrough(self.settings.click_through)
            .with_visible(visible)
            .with_inner_size([460.0, 140.0]);

        let overlay_viewport_id = ViewportId::from_hash_of("keyoverlay-overlay-viewport");

        ctx.show_viewport_immediate(
            overlay_viewport_id,
            builder,
            move |ctx, class| match class {
                ViewportClass::Embedded => {
                    egui::Window::new("Overlay")
                        .title_bar(false)
                        .resizable(false)
                        .collapsible(false)
                        .show(ctx, |ui| draw_keycaps(ui, &labels, alpha));
                }
                _ => {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
                        .show(ctx, |ui| draw_keycaps(ui, &labels, alpha));
                }
            },
        );
    }
}

fn draw_keycaps(ui: &mut egui::Ui, labels: &[String], alpha: f32) {
    if labels.is_empty() || alpha <= 0.01 {
        return;
    }

    let bg = Color32::from_rgba_premultiplied(23, 26, 31, (220.0 * alpha) as u8);
    let border = Color32::from_rgba_premultiplied(200, 208, 219, (160.0 * alpha) as u8);
    let text = Color32::from_rgba_premultiplied(242, 245, 250, (255.0 * alpha) as u8);

    ui.horizontal_wrapped(|ui| {
        for label in labels {
            let frame = egui::Frame::none()
                .fill(bg)
                .rounding(Rounding::same(10.0))
                .stroke(Stroke::new(1.0, border))
                .outer_margin(egui::Margin::symmetric(4.0, 4.0))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0));

            frame.show(ui, |ui| {
                ui.label(
                    egui::RichText::new(label)
                        .size(24.0 * alpha.max(0.25))
                        .color(text)
                        .strong(),
                );
            });
        }
    });
}

impl eframe::App for OverlayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(event) = self.rx.try_recv() {
            self.live_state.update_from_event(event, &self.settings);
        }

        self.draw_control_window(ctx);
        self.draw_overlay_viewport(ctx);

        ctx.request_repaint_after(Duration::from_millis(16));
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::TRANSPARENT.to_normalized_gamma_f32()
    }
}

pub fn run(rx: Receiver<InputEvent>) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([520.0, 420.0]),
        ..Default::default()
    };

    eframe::run_native(
        "KeyOverlay Controls",
        options,
        Box::new(|_cc| Box::new(OverlayApp::new(rx))),
    )
    .map_err(|err| anyhow!(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_chord_orders_modifiers_first_and_promotes_last_non_modifier() {
        let mut state = LiveInputState::new();
        state.keys_down.insert(Key::V);
        state.keys_down.insert(Key::Ctrl);
        state.keys_down.insert(Key::Shift);
        state.keys_down.insert(Key::A);
        state.mouse_down.insert(MouseButton::Left);
        state.last_non_modifier = Some(Key::V);

        let labels = format_chord(&state);
        assert_eq!(labels, vec!["Ctrl", "Shift", "V", "A", "MouseLeft"]);
    }
}
