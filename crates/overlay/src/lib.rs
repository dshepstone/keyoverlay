mod mouse_icon;
mod overlay_window;
mod settings_window;
mod theme;

use std::sync::mpsc::Receiver;

use anyhow::Result;
use eframe::egui;
use keyoverlay_core::SharedConfig;
use keyoverlay_input::InputEvent;

pub use overlay_window::OverlayApp;
pub use settings_window::SettingsApp;

/// Launch the transparent overlay popup.
pub fn run_overlay(rx: Receiver<InputEvent>, config: SharedConfig) -> Result<()> {
    let cfg = config.lock().unwrap().clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([cfg.overlay_width, cfg.overlay_height])
            .with_decorations(false)
            .with_always_on_top()
            .with_resizable(false)
            .with_transparent(true)
            .with_mouse_passthrough(true),
        ..Default::default()
    };

    eframe::run_native(
        "KeyOverlay",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Box::new(OverlayApp::new(rx, config))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}

/// Launch the settings UI window.
pub fn run_settings(config: SharedConfig) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 560.0])
            .with_resizable(true)
            .with_min_inner_size([420.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "KeyOverlay Settings",
        options,
        Box::new(move |_cc| Box::new(SettingsApp::new(config))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
