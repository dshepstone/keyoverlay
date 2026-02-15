use std::sync::mpsc;

use anyhow::Result;
use eframe::egui::IconData;
use image::io::Reader as ImageReader;
use keyoverlay_core::shared_config;
use keyoverlay_input::spawn_input_listener;

fn main() -> Result<()> {
    println!("KeyOverlay – starting...");

    let config = shared_config();
    let (tx, rx) = mpsc::channel();

    // Spawn the global input listener (keyboard + mouse via rdev).
    spawn_input_listener(tx);

    let app_icon = load_app_icon();

    // Run the combined settings + overlay UI on the main thread.
    keyoverlay_overlay::run(rx, config, app_icon)
}

fn load_app_icon() -> Option<IconData> {
    let image = ImageReader::open(concat!(env!("CARGO_MANIFEST_DIR"), "/icon.png"))
        .ok()?
        .decode()
        .ok()?
        .into_rgba8();
    let (width, height) = image.dimensions();

    Some(IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}
