use esp_idf_svc::errors::EspIOError;
use esp_idf_sys::{self as _, EspError};

use std::error;
use std::fmt;
use std::str::Utf8Error;

pub type Result<T> = std::result::Result<T, WmError>;

#[derive(Debug, Clone)]
pub enum WmError {
    InvalidArgument,
    EspError(EspError),
    EspIOError(EspIOError),
    Utf8Error(Utf8Error),
    InternalError,
    GlyphNotFound(char),
}

impl error::Error for WmError {}

impl fmt::Display for WmError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WmError::InvalidArgument => write!(f, "Invalid Argument"),
            WmError::EspError(error) => error.fmt(f),
            WmError::EspIOError(error) => error.fmt(f),
            WmError::Utf8Error(error) => error.fmt(f),
            WmError::InternalError => write!(f, "Internal Error"),
            WmError::GlyphNotFound(ch) => write!(f, "GlyphNotFound '{}'", ch),
        }
    }
}

impl From<EspError> for WmError {
    fn from(value: EspError) -> Self {
        WmError::EspError(value)
    }
}

impl From<Utf8Error> for WmError {
    fn from(value: Utf8Error) -> Self {
        WmError::Utf8Error(value)
    }
}

impl From<EspIOError> for WmError {
    fn from(value: EspIOError) -> Self {
        WmError::EspIOError(value)
    }
}

impl From<serde_json::Error> for WmError {
    fn from(_: serde_json::Error) -> Self {
        WmError::InternalError
    }
}

impl From<std::io::Error> for WmError {
    fn from(_: std::io::Error) -> Self {
        WmError::InternalError
    }
}

impl From<u8g2_fonts::Error<WmError>> for WmError {
    fn from(value: u8g2_fonts::Error<WmError>) -> Self {
        match value {
            u8g2_fonts::Error::BackgroundColorNotSupported => WmError::InternalError,
            u8g2_fonts::Error::GlyphNotFound(ch) => WmError::GlyphNotFound(ch),
            u8g2_fonts::Error::DisplayError(value) => value,
        }
    }
}
