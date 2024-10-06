use alloc::vec::Vec;

use deku::{DekuRead, DekuWrite};

pub mod event;
pub mod operation;
pub mod response;

#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
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
}

impl From<TransactionId> for Parameter {
	fn from(value: TransactionId) -> Self {
		Parameter::new(value.0)
	}
}
