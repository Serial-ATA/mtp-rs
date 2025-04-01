//! Error types for MTP communication

use core::fmt::Display;

pub use mtp_spec::error::*;

/// A specialized `Result` type for MTP operations.
pub type Result<T> = core::result::Result<T, Error>;

/// Errors that can occur during MTP operations
#[derive(Debug)]
pub enum Error {
    /// Errors during USB transport
    #[cfg(feature = "usb")]
    Usb(crate::usb::error::UsbError),
    /// Any I/O errors
    Io(std::io::Error),
    /// Any low-level protocol errors from [`mtp_spec`]
    Core(mtp_spec::error::MtpError),
    /// Any other errors
    Generic(Box<dyn std::error::Error>),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usb(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::Core(error) => write!(f, "{error}"),
            Self::Generic(error) => write!(f, "{error}"),
        }
    }
}

impl core::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(feature = "usb")]
impl From<crate::usb::error::UsbError> for Error {
    fn from(error: crate::usb::error::UsbError) -> Self {
        Self::Usb(error)
    }
}

impl From<mtp_spec::error::MtpError> for Error {
    fn from(error: mtp_spec::error::MtpError) -> Self {
        Self::Core(error)
    }
}
