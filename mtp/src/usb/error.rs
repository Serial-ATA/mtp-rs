//! Error types for USB transport

use core::fmt::Display;
use std::sync::Arc;

pub use mtp_spec::error::*;

/// A specialized `Result` type for MTP over USB operations.
pub type Result<T> = crate::error::Result<T, Arc<UsbError>>;

/// A specialized error type for MTP over USB operations.
pub type Error = crate::error::Error<Arc<UsbError>>;

impl From<Arc<UsbError>> for Error {
    fn from(error: Arc<UsbError>) -> Self {
        Self::Core(MtpError::Transport(error))
    }
}

/// Errors that can occur during USB transport
#[derive(Debug)]
pub enum UsbError {
    /// No applicable interfaces were found on the USB device
    NoApplicableInterface,
    /// OS I/O errors
    Native(nusb::Error),
    /// An error occurred during a USB transfer
    Transfer(nusb::transfer::TransferError),
    /// A USB operation timed out
    Timeout,
    /// The device replied with too much data (more than it claimed to have)
    TooMuchData,
}

impl Display for UsbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoApplicableInterface => write!(f, "No applicable interface found"),
            Self::Native(error) => write!(f, "{error}"),
            Self::Transfer(error) => write!(f, "{error}"),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::TooMuchData => write!(f, "Device replied with more data than claimed"),
        }
    }
}

impl core::error::Error for UsbError {}

impl From<nusb::Error> for UsbError {
    fn from(error: nusb::Error) -> Self {
        Self::Native(error)
    }
}

impl From<nusb::transfer::TransferError> for UsbError {
    fn from(error: nusb::transfer::TransferError) -> Self {
        Self::Transfer(error)
    }
}
