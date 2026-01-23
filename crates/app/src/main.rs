use keyoverlay_input::{Key, KeyEvent, Modifiers};

fn main() {
    println!("KeyOverlay – Phase 2 input normalization demo");

    let samples = [
        KeyEvent::new(Key::A, Modifiers::empty()),
        KeyEvent::new(Key::V, Modifiers::CTRL),
        KeyEvent::new(Key::ArrowLeft, Modifiers::SHIFT),
        KeyEvent::new(Key::Enter, Modifiers::CTRL | Modifiers::SHIFT),
        KeyEvent::new(Key::Space, Modifiers::ALT | Modifiers::WIN),
    ];

    for event in samples {
        println!("{}", event.display_string());
    }
}
