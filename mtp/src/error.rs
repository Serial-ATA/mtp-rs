use core::error::Error;
use core::fmt::Display;

/// A specialized `Result` type for MTP operations.
pub type Result<T> = core::result::Result<T, MtpError>;

#[derive(Debug)]
pub enum MtpError {
	Usb(nusb::Error),
}

impl Display for MtpError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Usb(error) => write!(f, "{error}"),
		}
	}
}

impl Error for MtpError {}

impl From<nusb::Error> for MtpError {
	fn from(error: nusb::Error) -> Self {
		Self::Usb(error)
	}
}
