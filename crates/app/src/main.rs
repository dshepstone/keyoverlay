use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use keyoverlay_input::{Key, KeyEvent, Modifiers};

fn spawn_sample_input(tx: mpsc::Sender<KeyEvent>) {
    thread::spawn(move || {
        let samples = [
            KeyEvent::new(Key::A, Modifiers::empty()),
            KeyEvent::new(Key::V, Modifiers::CTRL),
            KeyEvent::new(Key::ArrowLeft, Modifiers::SHIFT),
            KeyEvent::new(Key::Enter, Modifiers::CTRL | Modifiers::SHIFT),
            KeyEvent::new(Key::Space, Modifiers::ALT | Modifiers::WIN),
        ];

        let mut index = 0usize;
        loop {
            let event = samples[index % samples.len()];
            if tx.send(event).is_err() {
                break;
            }
            index += 1;
            thread::sleep(Duration::from_millis(750));
        }
    });
}

fn main() -> Result<()> {
    println!("KeyOverlay – Phase 3 overlay shell");

    let (tx, rx) = mpsc::channel();
    spawn_sample_input(tx);

    keyoverlay_overlay::run(rx)
}
