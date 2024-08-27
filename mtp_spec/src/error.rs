use core::fmt::{Debug, Display};

pub type Result<T> = core::result::Result<T, MtpError>;

// Shorthand for return Err(MtpError::new(MtpErrorKind::Foo))
//
// Usage:
// - err!(Variant)          -> return Err(MtpError::new(MtpErrorKind::Variant))
// - err!(Variant(Message)) -> return Err(MtpError::new(MtpErrorKind::Variant(Message)))
macro_rules! err {
	($variant:ident) => {
		return Err(crate::error::MtpError::new(
			crate::error::MtpErrorKind::$variant,
		))
	};
	($variant:ident($reason:literal)) => {
		return Err(crate::error::MtpError::new(
			crate::error::MtpErrorKind::$variant($reason),
		))
	};
}

pub(crate) use err;

#[derive(Debug)]
#[non_exhaustive]
pub enum MtpErrorKind {
	StringContainsNull,
}

impl Display for MtpErrorKind {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			MtpErrorKind::StringContainsNull => write!(f, "String contains null bytes"),
		}
	}
}

pub struct MtpError {
	kind: MtpErrorKind,
}

impl MtpError {
	pub fn new(kind: MtpErrorKind) -> MtpError {
		MtpError { kind }
	}
}

impl Debug for MtpError {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "{:?}", self.kind)
	}
}

impl Display for MtpError {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "{}", self.kind)
	}
}

impl std::error::Error for MtpError {}
