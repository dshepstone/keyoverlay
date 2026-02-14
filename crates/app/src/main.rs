use std::sync::mpsc;
use std::thread;

use anyhow::Result;
use keyoverlay_core::shared_config;
use keyoverlay_input::spawn_input_listener;

fn main() -> Result<()> {
    println!("KeyOverlay – starting...");

    let config = shared_config();

    let (tx, rx) = mpsc::channel();

    // Spawn the real global input listener (keyboard + mouse via rdev).
    spawn_input_listener(tx);

    // Spawn the overlay in a separate thread.
    let overlay_config = config.clone();
    thread::spawn(move || {
        if let Err(e) = keyoverlay_overlay::run_overlay(rx, overlay_config) {
            eprintln!("Overlay error: {e}");
        }
    });

    // Run the settings window on the main thread (eframe requirement).
    // When the user closes settings, the app exits.
    keyoverlay_overlay::run_settings(config)
}
