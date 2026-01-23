use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use keyoverlay_input::{KeyboardHook, KeyEvent, Modifiers};

fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel();
    let hook = KeyboardHook::start(tx)?;

    let running = Arc::new(AtomicBool::new(true));
    let running_flag = Arc::clone(&running);

    ctrlc::set_handler(move || {
        running_flag.store(false, Ordering::SeqCst);
    })?;

    while running.load(Ordering::SeqCst) {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(event) => print_event(&event),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    drop(hook);
    Ok(())
}

fn print_event(event: &KeyEvent) {
    println!(
        "[{}] KeyDown: {} (mods: {})",
        format_timestamp(event.timestamp),
        event.key_name,
        format_modifiers(&event.modifiers)
    );
}

fn format_modifiers(modifiers: &Modifiers) -> String {
    let mut parts = Vec::new();
    if modifiers.shift {
        parts.push("Shift");
    }
    if modifiers.ctrl {
        parts.push("Ctrl");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    if modifiers.win {
        parts.push("Win");
    }

    if parts.is_empty() {
        "None".to_string()
    } else {
        parts.join("+")
    }
}

fn format_timestamp(timestamp: SystemTime) -> String {
    match timestamp.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let seconds = duration.as_secs() % 86_400;
            let hours = seconds / 3_600;
            let minutes = (seconds % 3_600) / 60;
            let seconds = seconds % 60;
            format!("{hours:02}:{minutes:02}:{seconds:02}")
        }
        Err(_) => "00:00:00".to_string(),
    }
}
