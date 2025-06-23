// use embedded_graphics::{pixelcolor::Rgb565, prelude::DrawTarget};
use thiserror::Error;

pub type Result<T> = core::result::Result<T, MyError>;

#[derive(Error, Debug)]
pub enum MyError {
    // #[error("data store disconnected")]
    // Disconnect(#[from] io::Error),
    // #[error("the data for key `{0}` is not available")]
    // Redaction(String),
    // #[error("invalid header (expected {expected:?}, found {found:?})")]
    // InvalidHeader { expected: String, found: String },
    // #[error("unknown data store error")]
    // Unknown,
    #[error("EspError")]
    EspLCDError(#[from] esp_idf_hal::sys::EspError),
    // #[error("Draw target error")]
    // DrawError(#[from] DrawTarget<Color = Rgb565>::Error),
    // #[error("SlintError")]
    // SlintError(#[from] slint::PlatformError),
}
