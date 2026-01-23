use std::error::Error;
use std::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum OverlayError {
    NotImplemented,
}

impl fmt::Display for OverlayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OverlayError::NotImplemented => write!(f, "overlay rendering is not implemented yet"),
        }
    }
}

impl Error for OverlayError {}

pub fn initialize() -> Result<(), OverlayError> {
    // TODO: Initialize transparent overlay window + renderer.
    Err(OverlayError::NotImplemented)
}
