use std::sync::mpsc;

use anyhow::Result;
use keyoverlay_core::shared_config;
use keyoverlay_input::spawn_input_listener;

fn main() -> Result<()> {
    println!("KeyOverlay – starting...");

    let config = shared_config();
    let (tx, rx) = mpsc::channel();

    // Spawn the global input listener (keyboard + mouse via rdev).
    spawn_input_listener(tx);

    // Run the combined settings + overlay UI on the main thread.
    keyoverlay_overlay::run(rx, config)
}
