use crate::communication::response::ErrorCode;
use crate::communication::SessionId;

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
		pub struct $name:ident {
			code: $code:literal,
			parameters: ($($param:ident: $ty:ty),* $(,)?),
			valid_reponse_codes: [$($error:expr),* $(,)?] $(,)?
		}
	) => {
		const _: () = {
			if counter([$(replace_expr!($param ())),*]) > MAX_PARAMETERS {
				panic!("Too many parameters");
			}
		};

		$(#[$meta])*
		pub struct $name {
			parameters: [$crate::communication::Parameter; counter([$(replace_expr!($param ())),*])],
		}

		impl $name {
			const OPCODE: u16 = $code;
			const VALID_ERRORS: &[$crate::communication::response::ErrorCode] = &[$($error),*];

			paste::paste! {
				pub fn new($($param: $ty),*) -> Self {
					Self {
						parameters: [$(Into::<$crate::communication::Parameter>::into($param)),*],
					}
				}
			}
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
		valid_reponse_codes: [ErrorCode::ParameterNotSupported]
	}
}

define_operation! {
	/// Create a new session for between the initiator and responder.
	///
	/// Unless specified otherwise, all operations must be performed within the context of a session.
	///
	/// In the event that an active session already exists, a response of [`ResponseCode::SessionAlreadyOpen`]
	/// will be returned.
	pub struct OpenSession {
		code: 0x1002,
		parameters: (session_id: SessionId),
		valid_reponse_codes: [
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
		valid_reponse_codes: [
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
		valid_reponse_codes: [
			ErrorCode::OperationNotSupported,
			ErrorCode::SessionNotOpen,
			ErrorCode::InvalidTransactionID,
			ErrorCode::ParameterNotSupported,
		]
	}
}
