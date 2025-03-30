//! Device-to-Device communication primitives
//!
//! This module contains the primitives used for encoding and decoding MTP operations, responses,
//! and events.
//!
//! Some important terminology:
//!
//! * `Initiator`: Your host device, the one initiating operations through this library
//! * `Responder`: The device that is connected to the initiator, responding to operations

use core::fmt::Display;

use deku::{DekuRead, DekuWrite};

pub mod event;
pub mod operation;
pub mod response;

/// An encoded operation parameter
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
#[repr(transparent)]
pub struct Parameter(u32);

impl From<Parameter> for u32 {
    fn from(value: Parameter) -> Self {
        value.0
    }
}

impl Parameter {
    pub(crate) fn new(value: u32) -> Self {
        Self(value)
    }
}

/// Private parameter constructor, to disallow raw `u32` values
struct ParameterPriv(Parameter);

impl ParameterPriv {
    fn new<T: Into<Parameter>>(value: T) -> Self {
        Self(value.into())
    }

    fn new_raw(value: u32) -> Self {
        Self(Parameter::new(value))
    }
}

/// A session identifier in which an operation exists.
///
/// Operations can exist outside of an active session, in which case
/// [`SessionId::NONE`] is valid.
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
#[repr(transparent)]
pub struct SessionId(u32);

impl SessionId {
    /// A session identifier that represents no active session.
    pub const NONE: Self = SessionId(0);

    /// A session identifier representing all current sessions.
    pub const ALL: Self = SessionId(0xFFFFFF);

    /// Create a new session identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use mtp_spec::communication::SessionId;
    ///
    /// let session_id = SessionId::new(0x1234);
    /// assert_eq!(session_id.value(), 0x1234);
    /// ```
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Get the value of the session identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use mtp_spec::communication::SessionId;
    ///
    /// let session_id = SessionId::new(0x1234);
    /// assert_eq!(session_id.value(), 0x1234);
    /// ```
    pub fn value(self) -> u32 {
        self.0
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:06x}", self.0)
    }
}

impl From<SessionId> for Parameter {
    fn from(value: SessionId) -> Self {
        Parameter::new(value.0)
    }
}

/// An identifier for a transaction initiated by an operation.
///
/// Operations can exist outside of an active session, in which case
/// [`TransactionId::NONE`] is valid.
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
#[repr(transparent)]
pub struct TransactionId(u32);

impl TransactionId {
    /// A transaction identifier for operations with no active session.
    pub const NONE: Self = TransactionId(0);

    /// Create a new transaction identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use mtp_spec::communication::TransactionId;
    ///
    /// let transaction_id = TransactionId::new(0x1234);
    /// assert_eq!(transaction_id.value(), 0x1234);
    /// ```
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Get the value of the transaction identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use mtp_spec::communication::TransactionId;
    ///
    /// let transaction_id = TransactionId::new(0x1234);
    /// assert_eq!(transaction_id.value(), 0x1234);
    /// ```
    pub fn value(self) -> u32 {
        self.0
    }

    /// Get the next transaction identifier.
    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl Display for TransactionId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:06x}", self.0)
    }
}

impl From<TransactionId> for Parameter {
    fn from(value: TransactionId) -> Self {
        Parameter::new(value.0)
    }
}
