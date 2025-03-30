//! Error types for MTP communication

use core::error::Error;
use core::fmt::Display;

/// A specialized `Result` type for MTP operations.
pub type Result<T> = core::result::Result<T, MtpError>;

/// Errors that can occur during MTP operations
#[derive(Debug)]
pub enum MtpError {
	/// Errors during USB transport
	Usb(crate::usb::error::UsbError),
	/// Any I/O errors
	Io(std::io::Error),
	/// A USB operation timed out
	Timeout,
	/// Any low-level protocol errors from [`mtp_spec`]
	Core(mtp_spec::error::MtpError),
}

impl Display for MtpError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Usb(error) => write!(f, "{error}"),
			Self::Io(error) => write!(f, "{error}"),
			Self::Timeout => write!(f, "Operation timed out"),
			Self::Core(error) => write!(f, "{error}"),
		}
	}
}

impl Error for MtpError {}

impl From<std::io::Error> for MtpError {
	fn from(error: std::io::Error) -> Self {
		Self::Io(error)
	}
}

impl From<tokio::time::error::Elapsed> for MtpError {
	fn from(_error: tokio::time::error::Elapsed) -> Self {
		Self::Timeout
	}
}

impl From<crate::usb::error::UsbError> for MtpError {
	fn from(error: crate::usb::error::UsbError) -> Self {
		Self::Usb(error)
	}
}

impl From<mtp_spec::error::MtpError> for MtpError {
	fn from(error: mtp_spec::error::MtpError) -> Self {
		Self::Core(error)
	}
}
