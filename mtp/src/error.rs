//! Error types for MTP communication

use core::fmt::Display;
pub use mtp_spec::error::*;
use std::sync::Arc;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

/// A specialized `Result` type for MTP operations.
pub type Result<T> = core::result::Result<T, Error>;

/// Errors that can occur during MTP operations
#[derive(Clone, Debug)]
pub enum Error {
    /// Errors during USB transport
    #[cfg(feature = "usb")]
    Usb(Arc<crate::usb::error::UsbError>),
    /// Any I/O errors
    Io(Arc<std::io::Error>),
    /// Channel errors while attempting to receive events
    EventStream(BroadcastStreamRecvError),
    /// Any low-level protocol errors from [`mtp_spec`]
    Core(mtp_spec::error::MtpError),
    /// Any other errors
    Generic(Arc<dyn std::error::Error + Send + Sync>),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usb(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::EventStream(error) => write!(f, "{error}"),
            Self::Core(error) => write!(f, "{error}"),
            Self::Generic(error) => write!(f, "{error}"),
        }
    }
}

impl core::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(Arc::new(error))
    }
}

#[cfg(feature = "usb")]
impl From<crate::usb::error::UsbError> for Error {
    fn from(error: crate::usb::error::UsbError) -> Self {
        Self::Usb(Arc::new(error))
    }
}

impl From<BroadcastStreamRecvError> for Error {
    fn from(error: BroadcastStreamRecvError) -> Self {
        Self::EventStream(error)
    }
}

impl From<mtp_spec::error::MtpError> for Error {
    fn from(error: mtp_spec::error::MtpError) -> Self {
        Self::Core(error)
    }
}

impl From<deku::DekuError> for Error {
    fn from(error: deku::DekuError) -> Self {
        Self::Core(error.into())
    }
}
