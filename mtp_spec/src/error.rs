//! MTP protocol error types

use crate::communication::response::errors::OperationError;
use crate::object::{DateTimeError, NulError};

use core::fmt::{Display, Formatter};

/// Errors while serializing/deserializing operation data
#[derive(Clone, Debug)]
pub enum SerializationError {
    /// Attempting to send data to a responder, when the data direction is [`DataDirection::ResponderToInitiator`]
    ///
    /// [`DataDirection::ResponderToInitiator`]: crate::communication::operation::DataDirection::ResponderToInitiator
    WrongDataDirection,
    /// Attempted to provide data for an operation whose data direction is `None`
    UnexpectedDataProvided,
    /// Attempting to send an operation whose data direction is [`DataDirection::InitiatorToResponder`], but providing no data
    ///
    /// [`DataDirection::InitiatorToResponder`]: crate::communication::operation::DataDirection::InitiatorToResponder
    NoDataProvided,
    /// Attempting to deserialize a [`PtpString`] containing a null byte
    ///
    /// [`PtpString`]: crate::object::PtpString
    StringContainsNull,
    /// Attempting to parse a malformed [`DateTime`]
    ///
    /// [`DateTime`]: crate::object::DateTime
    BadDateTime(DateTimeError),
    /// General serialization/deserialization errors
    General(deku::DekuError),
}

impl Display for SerializationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            SerializationError::WrongDataDirection => write!(
                f,
                "Attempted to send data with an operation whose data direction is responder -> \
                 initiator"
            ),
            SerializationError::UnexpectedDataProvided => write!(
                f,
                "Data provided for operation when data direction is `None`"
            ),
            SerializationError::NoDataProvided => {
                write!(f, "Expected data for operation, but none was provided")
            },
            SerializationError::StringContainsNull => write!(f, "{NulError}"),
            SerializationError::BadDateTime(err) => write!(f, "{err}"),
            SerializationError::General(error) => write!(f, "{error}"),
        }
    }
}

impl core::error::Error for SerializationError {}

impl From<NulError> for SerializationError {
    fn from(_: NulError) -> Self {
        SerializationError::StringContainsNull
    }
}

impl From<DateTimeError> for SerializationError {
    fn from(error: DateTimeError) -> Self {
        SerializationError::BadDateTime(error)
    }
}

impl From<deku::DekuError> for SerializationError {
    fn from(error: deku::DekuError) -> Self {
        SerializationError::General(error)
    }
}

/// Errors that can occur during serialization and transport
#[derive(Clone, Debug)]
pub enum MtpError<E> {
    /// An error while serializing/deserializing operation/response data
    Serialization(SerializationError),
    /// An error from the protocol layer (e.g. the responder doesn't support an operation)
    Protocol(OperationError),
    /// An error from the transport layer (e.g. failed to decode a USB packet)
    Transport(E),
    /// Generic error for unsupported operations performed by high-level utilities
    UnsupportedOperation,
}

impl<E> Display for MtpError<E>
where
    E: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            MtpError::Serialization(error) => write!(f, "{error}"),
            MtpError::Protocol(e) => write!(f, "{e}"),
            MtpError::Transport(e) => write!(f, "{e}"),
            MtpError::UnsupportedOperation => {
                write!(f, "Operation is not supported by the responder")
            },
        }
    }
}

impl<E> core::error::Error for MtpError<E> where E: core::error::Error {}

impl<E> From<SerializationError> for MtpError<E> {
    fn from(error: SerializationError) -> Self {
        MtpError::Serialization(error)
    }
}

impl<E> From<NulError> for MtpError<E> {
    fn from(error: NulError) -> Self {
        MtpError::Serialization(error.into())
    }
}

impl<E> From<DateTimeError> for MtpError<E> {
    fn from(error: DateTimeError) -> Self {
        MtpError::Serialization(error.into())
    }
}

impl<E> From<deku::DekuError> for MtpError<E> {
    fn from(error: deku::DekuError) -> Self {
        MtpError::Serialization(error.into())
    }
}
