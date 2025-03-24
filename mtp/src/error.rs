use core::error::Error;
use core::fmt::Display;

/// A specialized `Result` type for MTP operations.
pub type Result<T> = core::result::Result<T, MtpError>;

#[derive(Debug)]
pub enum MtpError {
	Usb(crate::usb::error::UsbError),
	Io(std::io::Error),
	Timeout,
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
