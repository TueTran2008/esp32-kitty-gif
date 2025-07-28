use thiserror::Error;
use esp_idf_svc::sys::EspError;
use slint::platform::SetPlatformError;
use slint::PlatformError;

pub type Result<T> = core::result::Result<T, MyError>;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("EspError occurred: {0:?}")]
    Esp(#[from] EspError),
    #[error("{0:?}")]
    SetPlatformError(SetPlatformError),
    #[error("{0:?}")]
    PlatformError(PlatformError),
}

impl From<SetPlatformError> for MyError {
    fn from(err: SetPlatformError) -> Self {
        MyError::SetPlatformError(err)
    }
}

impl From<PlatformError> for MyError {
    fn from(err: PlatformError) -> Self {
        MyError::PlatformError(err)
    }
}