use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use eframe::egui;
use keyoverlay_input::InputEvent;

const MAX_EVENTS: usize = 40;

#[derive(Debug, Clone)]
struct OverlaySettings {
    show_keyboard: bool,
    show_mouse: bool,
    x: f32,
    y: f32,
    popup_ttl_secs: f32,
    popup_scale: f32,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            show_keyboard: true,
            show_mouse: true,
            x: 40.0,
            y: 40.0,
            popup_ttl_secs: 1.8,
            popup_scale: 1.2,
        }
    }
}

#[derive(Debug, Clone)]
struct PopupEvent {
    label: String,
    created_at: Instant,
}

struct OverlayApp {
    rx: Receiver<InputEvent>,
    history: VecDeque<String>,
    popups: VecDeque<PopupEvent>,
    settings: OverlaySettings,
}

impl OverlayApp {
    fn new(rx: Receiver<InputEvent>) -> Self {
        Self {
            rx,
            history: VecDeque::with_capacity(MAX_EVENTS),
            popups: VecDeque::with_capacity(MAX_EVENTS),
            settings: OverlaySettings::default(),
        }
    }

    fn accepts_event(&self, event: &InputEvent) -> bool {
        (event.is_keyboard() && self.settings.show_keyboard)
            || (event.is_mouse() && self.settings.show_mouse)
    }

    fn push_event(&mut self, event: InputEvent) {
        if !self.accepts_event(&event) {
            return;
        }

        let label = event.display_string();
        if self.history.len() == MAX_EVENTS {
            self.history.pop_front();
        }
        if self.popups.len() == MAX_EVENTS {
            self.popups.pop_front();
        }

        self.history.push_back(label.clone());
        self.popups.push_back(PopupEvent {
            label,
            created_at: Instant::now(),
        });
    }

    fn cleanup_expired_popups(&mut self) {
        let ttl = Duration::from_secs_f32(self.settings.popup_ttl_secs.max(0.2));
        let now = Instant::now();
        while let Some(front) = self.popups.front() {
            if now.duration_since(front.created_at) > ttl {
                self.popups.pop_front();
            } else {
                break;
            }
        }
    }
}

impl eframe::App for OverlayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut received_any = false;
        while let Ok(event) = self.rx.try_recv() {
            received_any = true;
            self.push_event(event);
        }

        self.cleanup_expired_popups();

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("KeyOverlay Control Menu");
                ui.separator();
                ui.label("Phase 4 scaffold: controls + popup placement");
            });
        });

        egui::SidePanel::left("controls").show(ctx, |ui| {
            ui.label("Capture Filters");
            ui.checkbox(&mut self.settings.show_keyboard, "Show keyboard events");
            ui.checkbox(&mut self.settings.show_mouse, "Show mouse button events");

            ui.separator();
            ui.label("On-screen Popup Placement");
            ui.add(egui::Slider::new(&mut self.settings.x, 0.0..=1800.0).text("X"));
            ui.add(egui::Slider::new(&mut self.settings.y, 0.0..=1000.0).text("Y"));

            ui.separator();
            ui.label("Popup Behavior");
            ui.add(
                egui::Slider::new(&mut self.settings.popup_ttl_secs, 0.2..=6.0)
                    .text("Visible seconds"),
            );
            ui.add(egui::Slider::new(&mut self.settings.popup_scale, 0.8..=2.2).text("Scale"));

            if ui.button("Clear history").clicked() {
                self.history.clear();
                self.popups.clear();
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Event History");
            ui.separator();

            if self.history.is_empty() {
                ui.label("Waiting for input events...");
            } else {
                for event in self.history.iter().rev() {
                    ui.label(event);
                }
            }
        });

        egui::Area::new("presentation_popups".into())
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(self.settings.x, self.settings.y))
            .show(ctx, |ui| {
                let text_size = 28.0 * self.settings.popup_scale;
                for popup in self.popups.iter().rev().take(3) {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&popup.label)
                                .size(text_size)
                                .strong()
                                .monospace(),
                        );
                    });
                    ui.add_space(6.0);
                }
            });

        if received_any {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

pub fn run(rx: Receiver<InputEvent>) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([920.0, 540.0])
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "KeyOverlay",
        options,
        Box::new(|_cc| Box::new(OverlayApp::new(rx))),
    )
    .map_err(|err| anyhow!(err.to_string()))
}
