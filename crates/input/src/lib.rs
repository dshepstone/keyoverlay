use std::fmt;
use std::sync::{mpsc, Arc, OnceLock};
use std::thread;
use std::time::Instant;

use bitflags::bitflags;

fn repaint_debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("OVERLAY_REPAINT_DEBUG").is_ok_and(|v| v == "1"))
}

// ── Key Enum ────────────────────────────────────────────────────────────

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Key {
    // Letters
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

    // Digits
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

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Navigation
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    PageUp,
    PageDown,

    // Editing
    Enter,
    Escape,
    Tab,
    Space,
    Backspace,
    Delete,
    Insert,

    // Punctuation / symbols
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Backquote,
    Comma,
    Period,
    Slash,

    // Modifiers (when pressed alone)
    Shift,
    Ctrl,
    Alt,
    Win,

    // Misc
    CapsLock,
    PrintScreen,
    ScrollLock,
    Pause,
    NumLock,
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
            Key::F1 => "F1",
            Key::F2 => "F2",
            Key::F3 => "F3",
            Key::F4 => "F4",
            Key::F5 => "F5",
            Key::F6 => "F6",
            Key::F7 => "F7",
            Key::F8 => "F8",
            Key::F9 => "F9",
            Key::F10 => "F10",
            Key::F11 => "F11",
            Key::F12 => "F12",
            Key::ArrowLeft => "\u{2190}",  // ←
            Key::ArrowRight => "\u{2192}", // →
            Key::ArrowUp => "\u{2191}",    // ↑
            Key::ArrowDown => "\u{2193}",  // ↓
            Key::Home => "Home",
            Key::End => "End",
            Key::PageUp => "PgUp",
            Key::PageDown => "PgDn",
            Key::Enter => "\u{23CE}", // ⏎
            Key::Escape => "Esc",
            Key::Tab => "\u{21E5}", // ⇥
            Key::Space => "Space",
            Key::Backspace => "\u{232B}", // ⌫
            Key::Delete => "Del",
            Key::Insert => "Ins",
            Key::Minus => "-",
            Key::Equal => "=",
            Key::BracketLeft => "[",
            Key::BracketRight => "]",
            Key::Backslash => "\\",
            Key::Semicolon => ";",
            Key::Quote => "'",
            Key::Backquote => "`",
            Key::Comma => ",",
            Key::Period => ".",
            Key::Slash => "/",
            Key::Shift => "Shift",
            Key::Ctrl => "Ctrl",
            Key::Alt => "Alt",
            Key::Win => {
                #[cfg(target_os = "windows")]
                {
                    "Win"
                }
                #[cfg(target_os = "macos")]
                {
                    "Cmd"
                }
                #[cfg(not(any(target_os = "windows", target_os = "macos")))]
                {
                    "Super"
                }
            }
            Key::CapsLock => "CapsLk",
            Key::PrintScreen => "PrtSc",
            Key::ScrollLock => "ScrLk",
            Key::Pause => "Pause",
            Key::NumLock => "NumLk",
        };
        write!(f, "{label}")
    }
}

// ── Modifiers ────────────────────────────────────────────────────────────

bitflags! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
    pub struct Modifiers: u8 {
        const SHIFT = 0b0001;
        const CTRL  = 0b0010;
        const ALT   = 0b0100;
        const WIN   = 0b1000;
    }
}

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return Ok(());
        }

        let mut first = true;
        #[cfg(target_os = "windows")]
        const WIN_LABEL: &str = "Win";
        #[cfg(target_os = "macos")]
        const WIN_LABEL: &str = "Cmd";
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        const WIN_LABEL: &str = "Super";

        let parts = [
            (Modifiers::CTRL, "Ctrl"),
            (Modifiers::SHIFT, "Shift"),
            (Modifiers::ALT, "Alt"),
            (Modifiers::WIN, WIN_LABEL),
        ];

        for (flag, label) in parts {
            if self.contains(flag) {
                if !first {
                    write!(f, " + ")?;
                }
                first = false;
                write!(f, "{label}")?;
            }
        }

        Ok(())
    }
}

// ── Mouse Button ─────────────────────────────────────────────────────────

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl fmt::Display for MouseButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MouseButton::Left => write!(f, "Left Click"),
            MouseButton::Right => write!(f, "Right Click"),
            MouseButton::Middle => write!(f, "Middle Click"),
        }
    }
}

// ── Scroll Direction ─────────────────────────────────────────────────────

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ScrollDirection {
    Up,
    Down,
}

impl fmt::Display for ScrollDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScrollDirection::Up => write!(f, "Scroll \u{2191}"),
            ScrollDirection::Down => write!(f, "Scroll \u{2193}"),
        }
    }
}

// ── Input Event (unified keyboard + mouse) ───────────────────────────────

#[derive(Debug, Clone)]
pub enum InputEvent {
    Key(KeyEvent),
    MouseClick(MouseClickEvent),
    Scroll(ScrollEvent),
}

impl InputEvent {
    pub fn timestamp(&self) -> Instant {
        match self {
            InputEvent::Key(e) => e.timestamp,
            InputEvent::MouseClick(e) => e.timestamp,
            InputEvent::Scroll(e) => e.timestamp,
        }
    }

    pub fn display_string(&self) -> String {
        match self {
            InputEvent::Key(e) => e.display_string(),
            InputEvent::MouseClick(e) => format!("{}", e.button),
            InputEvent::Scroll(e) => format!("{}", e.direction),
        }
    }
}

// ── Key Event ────────────────────────────────────────────────────────────

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
    pub is_down: bool,
    pub timestamp: Instant,
}

impl KeyEvent {
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key,
            modifiers,
            is_down: true,
            timestamp: Instant::now(),
        }
    }

    pub fn new_released(key: Key, modifiers: Modifiers) -> Self {
        Self {
            key,
            modifiers,
            is_down: false,
            timestamp: Instant::now(),
        }
    }

    pub fn with_timestamp(
        key: Key,
        modifiers: Modifiers,
        is_down: bool,
        timestamp: Instant,
    ) -> Self {
        Self {
            key,
            modifiers,
            is_down,
            timestamp,
        }
    }

    pub fn display_string(&self) -> String {
        if self.modifiers.is_empty() {
            format!("{}", self.key)
        } else {
            format!("{} + {}", self.modifiers, self.key)
        }
    }
}

// ── Mouse Click Event ────────────────────────────────────────────────────

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct MouseClickEvent {
    pub button: MouseButton,
    pub is_down: bool,
    pub timestamp: Instant,
}

impl MouseClickEvent {
    pub fn new(button: MouseButton) -> Self {
        Self {
            button,
            is_down: true,
            timestamp: Instant::now(),
        }
    }

    pub fn new_released(button: MouseButton) -> Self {
        Self {
            button,
            is_down: false,
            timestamp: Instant::now(),
        }
    }
}

// ── Scroll Event ─────────────────────────────────────────────────────────

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ScrollEvent {
    pub direction: ScrollDirection,
    pub timestamp: Instant,
}

impl ScrollEvent {
    pub fn new(direction: ScrollDirection) -> Self {
        Self {
            direction,
            timestamp: Instant::now(),
        }
    }
}

// ── rdev-based input capture ─────────────────────────────────────────────

/// Convert an rdev key to our Key type.
fn rdev_key_to_key(rkey: rdev::Key) -> Option<Key> {
    use rdev::Key as RK;
    let key = match rkey {
        RK::KeyA => Key::A,
        RK::KeyB => Key::B,
        RK::KeyC => Key::C,
        RK::KeyD => Key::D,
        RK::KeyE => Key::E,
        RK::KeyF => Key::F,
        RK::KeyG => Key::G,
        RK::KeyH => Key::H,
        RK::KeyI => Key::I,
        RK::KeyJ => Key::J,
        RK::KeyK => Key::K,
        RK::KeyL => Key::L,
        RK::KeyM => Key::M,
        RK::KeyN => Key::N,
        RK::KeyO => Key::O,
        RK::KeyP => Key::P,
        RK::KeyQ => Key::Q,
        RK::KeyR => Key::R,
        RK::KeyS => Key::S,
        RK::KeyT => Key::T,
        RK::KeyU => Key::U,
        RK::KeyV => Key::V,
        RK::KeyW => Key::W,
        RK::KeyX => Key::X,
        RK::KeyY => Key::Y,
        RK::KeyZ => Key::Z,
        RK::Num0 => Key::Digit0,
        RK::Num1 => Key::Digit1,
        RK::Num2 => Key::Digit2,
        RK::Num3 => Key::Digit3,
        RK::Num4 => Key::Digit4,
        RK::Num5 => Key::Digit5,
        RK::Num6 => Key::Digit6,
        RK::Num7 => Key::Digit7,
        RK::Num8 => Key::Digit8,
        RK::Num9 => Key::Digit9,
        RK::F1 => Key::F1,
        RK::F2 => Key::F2,
        RK::F3 => Key::F3,
        RK::F4 => Key::F4,
        RK::F5 => Key::F5,
        RK::F6 => Key::F6,
        RK::F7 => Key::F7,
        RK::F8 => Key::F8,
        RK::F9 => Key::F9,
        RK::F10 => Key::F10,
        RK::F11 => Key::F11,
        RK::F12 => Key::F12,
        RK::LeftArrow => Key::ArrowLeft,
        RK::RightArrow => Key::ArrowRight,
        RK::UpArrow => Key::ArrowUp,
        RK::DownArrow => Key::ArrowDown,
        RK::Home => Key::Home,
        RK::End => Key::End,
        RK::PageUp => Key::PageUp,
        RK::PageDown => Key::PageDown,
        RK::Return => Key::Enter,
        RK::Escape => Key::Escape,
        RK::Tab => Key::Tab,
        RK::Space => Key::Space,
        RK::Backspace => Key::Backspace,
        RK::Delete => Key::Delete,
        RK::Insert => Key::Insert,
        RK::Minus => Key::Minus,
        RK::Equal => Key::Equal,
        RK::LeftBracket => Key::BracketLeft,
        RK::RightBracket => Key::BracketRight,
        RK::BackSlash => Key::Backslash,
        RK::SemiColon => Key::Semicolon,
        RK::Quote => Key::Quote,
        RK::BackQuote => Key::Backquote,
        RK::Comma => Key::Comma,
        RK::Dot => Key::Period,
        RK::Slash => Key::Slash,
        RK::ShiftLeft | RK::ShiftRight => Key::Shift,
        RK::ControlLeft | RK::ControlRight => Key::Ctrl,
        RK::Alt | RK::AltGr => Key::Alt,
        RK::MetaLeft | RK::MetaRight => Key::Win,
        RK::CapsLock => Key::CapsLock,
        RK::PrintScreen => Key::PrintScreen,
        RK::ScrollLock => Key::ScrollLock,
        RK::Pause => Key::Pause,
        RK::NumLock => Key::NumLock,
        _ => return None,
    };
    Some(key)
}

/// Returns true if this key is a modifier key.
fn is_modifier_key(key: Key) -> bool {
    matches!(key, Key::Shift | Key::Ctrl | Key::Alt | Key::Win)
}

fn modifier_flag(key: Key) -> Option<Modifiers> {
    match key {
        Key::Shift => Some(Modifiers::SHIFT),
        Key::Ctrl => Some(Modifiers::CTRL),
        Key::Alt => Some(Modifiers::ALT),
        Key::Win => Some(Modifiers::WIN),
        _ => None,
    }
}

/// Track currently held modifier state.
#[derive(Default)]
struct ModifierState {
    shift: bool,
    ctrl: bool,
    alt: bool,
    win: bool,
}

impl ModifierState {
    fn as_modifiers(&self) -> Modifiers {
        let mut m = Modifiers::empty();
        if self.shift {
            m |= Modifiers::SHIFT;
        }
        if self.ctrl {
            m |= Modifiers::CTRL;
        }
        if self.alt {
            m |= Modifiers::ALT;
        }
        if self.win {
            m |= Modifiers::WIN;
        }
        m
    }

    fn press(&mut self, key: Key) {
        match key {
            Key::Shift => self.shift = true,
            Key::Ctrl => self.ctrl = true,
            Key::Alt => self.alt = true,
            Key::Win => self.win = true,
            _ => {}
        }
    }

    fn release(&mut self, key: Key) {
        match key {
            Key::Shift => self.shift = false,
            Key::Ctrl => self.ctrl = false,
            Key::Alt => self.alt = false,
            Key::Win => self.win = false,
            _ => {}
        }
    }
}

/// Spawn a background thread that listens for real keyboard and mouse input
/// and forwards events over the provided channel.
pub fn spawn_input_listener(tx: mpsc::Sender<InputEvent>) {
    spawn_input_listener_with_wakeup(tx, None);
}

/// Same as `spawn_input_listener`, but allows providing a repaint wake callback
/// that is invoked after every enqueued input event.
pub fn spawn_input_listener_with_wakeup(
    tx: mpsc::Sender<InputEvent>,
    wake_repaint: Option<Arc<dyn Fn() + Send + Sync>>,
) {
    thread::spawn(move || {
        use rdev::{listen, Event, EventType};

        // `listen` takes an `FnMut` callback that only ever runs on this
        // thread, so plain mutable captures are enough — no locking in the
        // hook path.
        let mut state = ModifierState::default();

        // Forward an event to the UI and wake the repaint loop. Kept minimal:
        // this runs inside the OS input hook, so it must never block.
        let forward = move |event: InputEvent| {
            if tx.send(event).is_ok() {
                if let Some(wake) = &wake_repaint {
                    if repaint_debug_enabled() {
                        eprintln!(
                            "[overlay-repaint] enqueue input event t={:?} -> request_repaint",
                            Instant::now()
                        );
                    }
                    wake();
                }
            }
        };

        let callback = move |event: Event| {
            match event.event_type {
                EventType::KeyPress(rkey) => {
                    if let Some(key) = rdev_key_to_key(rkey) {
                        if is_modifier_key(key) {
                            state.press(key);
                            // Report the other held modifiers, not the one
                            // this event is about.
                            let mut clean_mods = state.as_modifiers();
                            if let Some(flag) = modifier_flag(key) {
                                clean_mods.remove(flag);
                            }
                            forward(InputEvent::Key(KeyEvent::new(key, clean_mods)));
                        } else {
                            let modifiers = state.as_modifiers();
                            forward(InputEvent::Key(KeyEvent::new(key, modifiers)));
                        }
                    }
                }
                EventType::KeyRelease(rkey) => {
                    if let Some(key) = rdev_key_to_key(rkey) {
                        let mut modifiers = state.as_modifiers();
                        if let Some(flag) = modifier_flag(key) {
                            modifiers.remove(flag);
                        }
                        forward(InputEvent::Key(KeyEvent::new_released(key, modifiers)));

                        if is_modifier_key(key) {
                            state.release(key);
                        }
                    }
                }
                EventType::ButtonPress(btn) => {
                    let button = match btn {
                        rdev::Button::Left => Some(MouseButton::Left),
                        rdev::Button::Right => Some(MouseButton::Right),
                        rdev::Button::Middle => Some(MouseButton::Middle),
                        _ => None,
                    };
                    if let Some(b) = button {
                        forward(InputEvent::MouseClick(MouseClickEvent::new(b)));
                    }
                }
                EventType::ButtonRelease(btn) => {
                    let button = match btn {
                        rdev::Button::Left => Some(MouseButton::Left),
                        rdev::Button::Right => Some(MouseButton::Right),
                        rdev::Button::Middle => Some(MouseButton::Middle),
                        _ => None,
                    };
                    if let Some(b) = button {
                        forward(InputEvent::MouseClick(MouseClickEvent::new_released(b)));
                    }
                }
                EventType::Wheel { delta_y, .. } => {
                    let direction = if delta_y > 0 {
                        ScrollDirection::Up
                    } else if delta_y < 0 {
                        ScrollDirection::Down
                    } else {
                        return;
                    };
                    forward(InputEvent::Scroll(ScrollEvent::new(direction)));
                }
                _ => {}
            }
        };

        if let Err(e) = listen(callback) {
            eprintln!("Input listener error: {:?}", e);
        }
    });
}

// ── Platform-specific VK mapping (kept for Windows compatibility) ────────

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::vk_to_key;

#[cfg(not(windows))]
pub fn vk_to_key(_: u32) -> Option<Key> {
    None
}
