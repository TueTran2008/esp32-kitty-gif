// use embedded_graphics::{pixelcolor::Rgb565, prelude::DrawTarget};
use thiserror::Error;
// use esp_idf_svc::ota::EspOta;
use esp_idf_svc::sys::EspError;
// use embedded_svc::http::HttpError;
// use esp_idf_svc::ota::OtaUpdateError;


pub type Result<T> = core::result::Result<T, MyError>;
/// Macro to quickly create EspError from an ESP_ERR_ constant.

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
    Esp(#[from] EspError),

    // #[error("Failed to get running OTA slot")]
    // GetSlotFailed,

    // #[error("No firmware found in running slot")]
    // FirmwareMissing,

    // #[error("Failed to obtain OTA instance")]
    // OtaInit,

    // #[error("Failed to get running OTA slot")]
    // GetRunningSlot,

    // #[error("No firmware found in slot")]
    // MissingFirmware,

    // #[error("Failed to send update request")]
    // SendRequest,

    // #[error("Failed to initiate update")]
    // InitiateUpdate,

    // #[error("Failed to download update")]
    // DownloadUpdate,

    // #[error("Failed to complete update")]
    // CompleteUpdate,

    // #[error("Failed to abort update")]
    // AbortUpdate,
    // #[error("ESP 32 idf error")]
    // EspError,
    // #[error("HTTP request error")]
    // Http,

    // // #[error("ESP OTA error")]
    // // EspOta(#[from] esp_idf_svc::ota::OtaUpdateError),
    // #[error("HTTP error")]
    // HttpError,

    // #[error("I/O error")]
    // IoError,

    // #[error("OTA write failed")]
    // OtaWriteFailed,

    // #[error("Failed to read firmware info")]
    // FirmwareInfoReadFailed,

    // #[error("Failed to parse firmware info")]
    // FirmwareInfoParseFailed,

    // #[error("Firmware info not found")]
    // FirmwareInfoNotFound,
    
    // #[error("Firmware copy failed")]
    // CopyFailed,
}
