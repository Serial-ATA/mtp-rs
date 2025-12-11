//! Error types for MTP communication

use core::fmt::Display;
use std::sync::Arc;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

pub use mtp_spec::error::*;

/// A specialized `Result` type for MTP operations.
pub type Result<T, TransportError> = core::result::Result<T, Error<TransportError>>;

/// Errors that can occur during MTP operations
#[derive(Clone, Debug)]
pub enum Error<Transport> {
    /// Any I/O errors
    Io(Arc<std::io::Error>),
    /// Channel errors while attempting to receive events
    EventStream(BroadcastStreamRecvError),
    /// Any protocol errors from [`mtp_spec`]
    Core(MtpError<Transport>),
    /// Any other errors
    Generic(Arc<dyn std::error::Error + Send + Sync>),
}

impl<Transport> Display for Error<Transport>
where
    Transport: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::EventStream(error) => write!(f, "{error}"),
            Self::Core(error) => write!(f, "{error}"),
            Self::Generic(error) => write!(f, "{error}"),
        }
    }
}

impl<Transport> core::error::Error for Error<Transport> where Transport: core::error::Error {}

impl<Transport> From<std::io::Error> for Error<Transport> {
    fn from(error: std::io::Error) -> Self {
        Self::Io(Arc::new(error))
    }
}

impl<Transport> From<BroadcastStreamRecvError> for Error<Transport> {
    fn from(error: BroadcastStreamRecvError) -> Self {
        Self::EventStream(error)
    }
}

impl<Transport> From<MtpError<Transport>> for Error<Transport> {
    fn from(error: MtpError<Transport>) -> Self {
        Self::Core(error)
    }
}

impl<Transport> From<deku::DekuError> for Error<Transport> {
    fn from(error: deku::DekuError) -> Self {
        Self::Core(MtpError::Serialization(error.into()))
    }
}
