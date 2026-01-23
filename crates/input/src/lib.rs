use std::error::Error;
use std::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InputError {
    NotImplemented,
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputError::NotImplemented => write!(f, "input hooks are not implemented yet"),
        }
    }
}

impl Error for InputError {}

pub fn initialize() -> Result<(), InputError> {
    // TODO: Install low-level keyboard/mouse hooks for Windows.
    Err(InputError::NotImplemented)
}
