use crate::communication::operation::Operation;
use crate::communication::response::ErrorCode;
use crate::communication::{response, SessionId, TransactionId};
use crate::device::info::DeviceInfo;
use crate::device::storage::id::StorageId;
use crate::object::types::Array;

const fn counter<const N: usize>(_: [(); N]) -> usize {
	N
}

macro_rules! replace_expr {
	($_t:tt $sub:expr) => {
		$sub
	};
}

const MAX_PARAMETERS: usize = 5;

macro_rules! define_operation {
	(
		$(#[$meta:meta])*
		$([[session_id($session_id:ident)]])?
		pub struct $name:ident {
			code: $code:literal,
			parameters: ($($param:ident: $ty:ty),* $(,)?),
			response: $response:ty,
			valid_error_codes: [$($error:expr),* $(,)?] $(,)?
		}
	) => {
		define_operation!(
			$(#[$meta])* $name, code: $code, $(session_id: $session_id)?, [$($param: $ty),*], $response, [$($error),*]
		);

		impl $name {
			const OPCODE: u16 = $code;
			const VALID_ERRORS: &[$crate::communication::response::ErrorCode] = &[$($error),*];
		}

		define_operation!(
			@CONSTRUCTOR $name, $code, $(SESSION_ID: $session_id,)? ($($param: $ty),*), [$($error),*]
		);
	};

	(
		@CONSTRUCTOR
		$name:ident,
		$code:literal,
		SESSION_ID: false,
		($($param:ident: $ty:ty),* $(,)?),
		[$($error:expr),* $(,)?]
	) => {
		impl $name {
			pub fn new(transaction_id: $crate::communication::TransactionId, $($param: $ty),*) -> Self {
				Self {
					parameters: [$(Into::<$crate::communication::Parameter>::into($param)),*],
					session_id: None,
					transaction_id,
				}
			}
		}
	};

	(
		@CONSTRUCTOR
		$name:ident,
		$code:literal,
		($($param:ident: $ty:ty),* $(,)?),
		[$($error:expr),* $(,)?]
	) => {
		impl $name {
			pub fn new(transaction_id: $crate::communication::TransactionId, session_id: SessionId, $($param: $ty),*) -> Self {
				Self {
					parameters: [$(Into::<$crate::communication::Parameter>::into($param)),*],
					session_id: Some(session_id),
					transaction_id,
				}
			}
		}
	};

	(
		$(#[$meta:meta])* $name:ident, code: $code:literal, $(session_id: $session_id:ident)?, [$($param:ident: $ty:ty),* $(,)?], $response:ty, [$($error:expr),* $(,)?]
	) => {
		const _: () = {
			if counter([$(replace_expr!($param ())),*]) > MAX_PARAMETERS {
				panic!("Too many parameters");
			}
		};

		$(#[$meta])*
		pub struct $name {
			parameters: [$crate::communication::Parameter; counter([$(replace_expr!($param ())),*])],
			session_id: Option<$crate::communication::SessionId>,
			transaction_id: $crate::communication::TransactionId,
		}

		impl<'a> From<&'a $name> for $crate::communication::operation::SerializedOperation<'a> {
			fn from(value: &'a $name) -> $crate::communication::operation::SerializedOperation {
				Self {
					code: <$name>::OPCODE,
					session_id: value.session_id.unwrap_or(SessionId::NONE),
					transaction_id: value.transaction_id,
					parameters: value.parameters.as_slice(),
				}
			}
		}

		impl $crate::communication::operation::Operation for $name {
			type Response = $response;
		}
	}
}

define_operation! {
	/// Get the [`DeviceInfo`] of the device.
	///
	/// This operation is commonly the first operation called by an initiator upon
	/// connecting to a responder for the first time.
	pub struct GetDeviceInfo {
		code: 0x1001,
		parameters: (),
		response: response::GetDeviceInfoResponse,
		valid_error_codes: [ErrorCode::ParameterNotSupported]
	}
}

define_operation! {
	/// Create a new session for between the initiator and responder.
	///
	/// Unless specified otherwise, all operations must be performed within the context of a session.
	///
	/// In the event that an active session already exists, a response of [`ResponseCode::SessionAlreadyOpen`]
	/// will be returned.
	[[session_id(false)]]
	pub struct OpenSession {
		code: 0x1002,
		parameters: (session_id: SessionId),
		response: response::EmptyResponse,
		valid_error_codes: [
			ErrorCode::ParameterNotSupported,
			ErrorCode::InvalidParameter,
			ErrorCode::SessionAlreadyOpen,
			ErrorCode::DeviceBusy,
		]
	}
}

define_operation! {
	/// Close the current session.
	///
	/// All stateful information associated with the session will be discarded.
	///
	/// If no session is currently open, a response of [`ResponseCode::SessionNotOpen`] will be returned.
	pub struct CloseSession {
		code: 0x1003,
		parameters: (),
		response: response::EmptyResponse,
		valid_error_codes: [
			ErrorCode::SessionNotOpen,
			ErrorCode::InvalidTransactionID,
			ErrorCode::ParameterNotSupported,
		]
	}
}

define_operation! {
	/// Get the storage IDs of all storages on the device.
	pub struct GetStorageIDs {
		code: 0x1004,
		parameters: (),
		response: response::GetStorageIDsResponse,
		valid_error_codes: [
			ErrorCode::OperationNotSupported,
			ErrorCode::SessionNotOpen,
			ErrorCode::InvalidTransactionID,
			ErrorCode::ParameterNotSupported,
		]
	}
}
