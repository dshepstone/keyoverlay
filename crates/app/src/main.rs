use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use keyoverlay_input::{InputEvent, Key, Modifiers, MouseButton};

fn spawn_sample_input(tx: mpsc::Sender<InputEvent>) {
    thread::spawn(move || {
        let samples = [
            InputEvent::key_down(Key::Ctrl, Modifiers::CTRL),
            InputEvent::key_down(Key::V, Modifiers::CTRL),
            InputEvent::key_up(Key::V),
            InputEvent::key_up(Key::Ctrl),
            InputEvent::mouse_down(MouseButton::Left),
            InputEvent::mouse_up(MouseButton::Left),
            InputEvent::key_down(Key::Shift, Modifiers::SHIFT),
            InputEvent::key_down(Key::ArrowLeft, Modifiers::SHIFT),
            InputEvent::key_up(Key::ArrowLeft),
            InputEvent::key_up(Key::Shift),
        ];

        let mut index = 0usize;
        loop {
            let event = samples[index % samples.len()];
            if tx.send(event).is_err() {
                break;
            }
            index += 1;
            thread::sleep(Duration::from_millis(240));
        }
    });
}

fn main() -> Result<()> {
    println!("KeyOverlay – live overlay + controls scaffold");

    let (tx, rx) = mpsc::channel();
    spawn_sample_input(tx);

    keyoverlay_overlay::run(rx)
}
