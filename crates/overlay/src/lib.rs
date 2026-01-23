use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use anyhow::Result;
use eframe::egui;
use keyoverlay_input::KeyEvent;

const MAX_EVENTS: usize = 20;

struct OverlayApp {
    rx: Receiver<KeyEvent>,
    events: VecDeque<String>,
}

impl OverlayApp {
    fn new(rx: Receiver<KeyEvent>) -> Self {
        Self {
            rx,
            events: VecDeque::with_capacity(MAX_EVENTS),
        }
    }

    fn push_event(&mut self, event: KeyEvent) {
        if self.events.len() == MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(event.display_string());
    }
}

impl eframe::App for OverlayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut received_any = false;
        while let Ok(event) = self.rx.try_recv() {
            received_any = true;
            self.push_event(event);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("KeyOverlay – overlay shell");
            ui.separator();

            if self.events.is_empty() {
                ui.label("Waiting for input events...");
            } else {
                for event in self.events.iter().rev() {
                    ui.label(event);
                }
            }
        });

        if received_any {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}

pub fn run(rx: Receiver<KeyEvent>) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 200.0])
            .with_decorations(false)
            .with_always_on_top(true)
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "KeyOverlay",
        options,
        Box::new(|_cc| Box::new(OverlayApp::new(rx))),
    )?;

    Ok(())
}
