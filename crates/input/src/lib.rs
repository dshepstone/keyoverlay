use std::fmt;
use std::time::Instant;

use bitflags::bitflags;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Enter,
    Escape,
    Tab,
    Space,
    Backspace,
    Shift,
    Ctrl,
    Alt,
    Win,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Key::A => "A",
            Key::B => "B",
            Key::C => "C",
            Key::D => "D",
            Key::E => "E",
            Key::F => "F",
            Key::G => "G",
            Key::H => "H",
            Key::I => "I",
            Key::J => "J",
            Key::K => "K",
            Key::L => "L",
            Key::M => "M",
            Key::N => "N",
            Key::O => "O",
            Key::P => "P",
            Key::Q => "Q",
            Key::R => "R",
            Key::S => "S",
            Key::T => "T",
            Key::U => "U",
            Key::V => "V",
            Key::W => "W",
            Key::X => "X",
            Key::Y => "Y",
            Key::Z => "Z",
            Key::Digit0 => "0",
            Key::Digit1 => "1",
            Key::Digit2 => "2",
            Key::Digit3 => "3",
            Key::Digit4 => "4",
            Key::Digit5 => "5",
            Key::Digit6 => "6",
            Key::Digit7 => "7",
            Key::Digit8 => "8",
            Key::Digit9 => "9",
            Key::ArrowLeft => "ArrowLeft",
            Key::ArrowRight => "ArrowRight",
            Key::ArrowUp => "ArrowUp",
            Key::ArrowDown => "ArrowDown",
            Key::Enter => "Enter",
            Key::Escape => "Escape",
            Key::Tab => "Tab",
            Key::Space => "Space",
            Key::Backspace => "Backspace",
            Key::Shift => "Shift",
            Key::Ctrl => "Ctrl",
            Key::Alt => "Alt",
            Key::Win => "Win",
        };

        write!(f, "{label}")
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
    pub struct Modifiers: u8 {
        const SHIFT = 0b0001;
        const CTRL = 0b0010;
        const ALT = 0b0100;
        const WIN = 0b1000;
    }
}

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return Ok(());
        }

        let mut first = true;
        let parts = [
            (Modifiers::CTRL, "Ctrl"),
            (Modifiers::SHIFT, "Shift"),
            (Modifiers::ALT, "Alt"),
            (Modifiers::WIN, "Win"),
        ];

        for (flag, label) in parts {
            if self.contains(flag) {
                if !first {
                    write!(f, "+")?;
                }
                first = false;
                write!(f, "{label}")?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
    pub timestamp: Instant,
}

impl KeyEvent {
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key,
            modifiers,
            timestamp: Instant::now(),
        }
    }

    pub fn with_timestamp(key: Key, modifiers: Modifiers, timestamp: Instant) -> Self {
        Self {
            key,
            modifiers,
            timestamp,
        }
    }

    pub fn display_string(&self) -> String {
        if self.modifiers.is_empty() {
            format!("{}", self.key)
        } else {
            format!("{}+{}", self.modifiers, self.key)
        }
    }
}

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::vk_to_key;

#[cfg(not(windows))]
pub fn vk_to_key(_: u32) -> Option<Key> {
    None
}
