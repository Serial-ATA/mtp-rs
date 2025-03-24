use core::error::Error;
use core::fmt::Display;

#[derive(Debug)]
pub enum UsbError {
	NoApplicableInterface,
	Native(nusb::Error),
	Transfer(nusb::transfer::TransferError),
}

impl Display for UsbError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::NoApplicableInterface => write!(f, "No applicable interface found"),
			Self::Native(error) => write!(f, "{error}"),
			Self::Transfer(error) => write!(f, "{error}"),
		}
	}
}

impl Error for UsbError {}

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
