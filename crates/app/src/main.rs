use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use keyoverlay_input::{InputEvent, Key, KeyEvent, Modifiers, MouseButton, MouseEvent};

fn spawn_sample_input(tx: mpsc::Sender<InputEvent>) {
    thread::spawn(move || {
        let samples = [
            InputEvent::Key(KeyEvent::new(Key::A, Modifiers::empty())),
            InputEvent::Key(KeyEvent::new(Key::V, Modifiers::CTRL)),
            InputEvent::Mouse(MouseEvent::new(MouseButton::Left, true)),
            InputEvent::Mouse(MouseEvent::new(MouseButton::Right, true)),
            InputEvent::Key(KeyEvent::new(Key::ArrowLeft, Modifiers::SHIFT)),
            InputEvent::Key(KeyEvent::new(
                Key::Enter,
                Modifiers::CTRL | Modifiers::SHIFT,
            )),
            InputEvent::Mouse(MouseEvent::new(MouseButton::Middle, true)),
            InputEvent::Key(KeyEvent::new(Key::Space, Modifiers::ALT | Modifiers::WIN)),
        ];

        let mut index = 0usize;
        loop {
            let event = samples[index % samples.len()];
            if tx.send(event).is_err() {
                break;
            }
            index += 1;
            thread::sleep(Duration::from_millis(550));
        }
    });
}

fn main() -> Result<()> {
    println!("KeyOverlay – Phase 4 UI shell");

    let (tx, rx) = mpsc::channel();
    spawn_sample_input(tx);

    keyoverlay_overlay::run(rx)
}
