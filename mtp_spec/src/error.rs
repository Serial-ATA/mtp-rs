//! Errors that can occur during MTP operations

use core::fmt::{Debug, Display};

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

/// The kind of error that occurred
#[derive(Debug)]
#[non_exhaustive]
pub enum MtpErrorKind {
    /// Attempting to deserialize a [`PtpString`] containing a null byte
    ///
    /// [`PtpString`]: crate::object::types::PtpString
    StringContainsNull,
    /// Attempting to parse a malformed [`DateTime`]
    ///
    /// [`DateTime`]: crate::object::types::DateTime
    BadDateTime(&'static str),
    /// General serialization/deserialization errors
    Serialization(deku::DekuError),
    Generic(Box<dyn core::error::Error>),
    #[cfg(feature = "fs")]
    Io(std::io::Error),
}

impl Display for MtpErrorKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MtpErrorKind::StringContainsNull => write!(f, "String contains null bytes"),
            MtpErrorKind::BadDateTime(reason) => write!(f, "Bad DateTime string: {}", reason),
            MtpErrorKind::Serialization(error) => write!(f, "Serialization error: {}", error),
            MtpErrorKind::Generic(error) => write!(f, "{error}"),
            #[cfg(feature = "fs")]
            MtpErrorKind::Io(error) => write!(f, "{error}"),
        }
    }
}

/// Errors that can occur during MTP operations
pub struct MtpError {
    kind: MtpErrorKind,
}

impl MtpError {
    /// Create a new `MtpError`
    pub fn new(kind: MtpErrorKind) -> MtpError {
        MtpError { kind }
    }

    pub fn kind(&self) -> &MtpErrorKind {
        &self.kind
    }
}

impl Debug for MtpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.kind)
    }
}

impl Display for MtpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl core::error::Error for MtpError {}

impl From<deku::DekuError> for MtpError {
    fn from(error: deku::DekuError) -> Self {
        MtpError::new(MtpErrorKind::Serialization(error))
    }
}

#[cfg(feature = "fs")]
impl From<std::io::Error> for MtpError {
    fn from(error: std::io::Error) -> Self {
        MtpError::new(MtpErrorKind::Io(error))
    }
}
