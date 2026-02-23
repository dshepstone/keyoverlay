use std::sync::{mpsc, Arc};

use anyhow::Result;
use eframe::egui::IconData;
use keyoverlay_core::shared_config;
use keyoverlay_input::spawn_input_listener_with_wakeup;

fn main() -> Result<()> {
    println!("KeyOverlay – starting...");

    let config = shared_config();
    let (tx, rx) = mpsc::channel();

    // Spawn the global input listener (keyboard + mouse via rdev).
    let wake_repaint = Arc::new(|| keyoverlay_overlay::request_external_repaint());
    spawn_input_listener_with_wakeup(tx, Some(wake_repaint));

    let app_icon = load_app_icon();

    // Run the combined settings + overlay UI on the main thread.
    keyoverlay_overlay::run(rx, config, app_icon)
}

fn load_app_icon() -> Option<IconData> {
    let bytes = include_bytes!("../icon.png");
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = image.dimensions();

    Some(IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}
