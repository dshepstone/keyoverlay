use std::sync::mpsc;

use anyhow::Result;
use keyoverlay_input::spawn_input_listener;

fn main() -> Result<()> {
    println!("KeyOverlay – starting input listener and overlay...");

    let (tx, rx) = mpsc::channel();

    // Spawn the real global input listener (keyboard + mouse via rdev).
    spawn_input_listener(tx);

    // Launch the overlay UI (blocks until window is closed).
    keyoverlay_overlay::run(rx)
}
