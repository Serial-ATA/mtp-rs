//! Errors that can occur during MTP operations

use crate::object::types::DateTimeError;
use crate::object::types::properties::ObjectPropertyCode;

use alloc::boxed::Box;
use core::fmt::{Debug, Display};

// Shorthand for return Err(MtpError::Foo)
//
// Usage:
// - err!(Variant)          -> return Err(MtpError::Variant)
// - err!(Variant(Message)) -> return Err(MtpError::Variant(Message))
macro_rules! err {
    ($variant:ident) => {
        return Err(crate::error::MtpError::$variant)
    };
    ($variant:ident, $reason:literal) => {
        return Err(crate::error::MtpError::$variant($reason))
    };
}

pub(crate) use err;

/// Errors that can occur during MTP operations
#[derive(Debug)]
#[non_exhaustive]
pub enum MtpError {
    /// Attempting to deserialize a [`PtpString`] containing a null byte
    ///
    /// [`PtpString`]: crate::object::types::PtpString
    StringContainsNull,
    /// Attempting to parse a malformed [`DateTime`]
    ///
    /// [`DateTime`]: crate::object::types::DateTime
    BadDateTime(DateTimeError),
    /// Attempt to modify a property that the device does not allow modifying
    CannotModify(ObjectPropertyCode),
    /// General serialization/deserialization errors
    Serialization(deku::DekuError),
    Generic(Box<dyn core::error::Error>),
}

impl Display for MtpError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MtpError::StringContainsNull => write!(f, "String contains null bytes"),
            MtpError::BadDateTime(err) => write!(f, "{err}"),
            MtpError::CannotModify(code) => write!(
                f,
                "Device does not support modifying the `{code:?}` property"
            ),
            MtpError::Serialization(error) => write!(f, "Serialization error: {}", error),
            MtpError::Generic(error) => write!(f, "{error}"),
        }
    }
}

impl core::error::Error for MtpError {}

impl From<DateTimeError> for MtpError {
    fn from(error: DateTimeError) -> Self {
        MtpError::BadDateTime(error)
    }
}

impl From<deku::DekuError> for MtpError {
    fn from(error: deku::DekuError) -> Self {
        MtpError::Serialization(error)
    }
}
