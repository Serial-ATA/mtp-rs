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

macro_rules! session_id {
	(false) => {
		None
	};
	() => {
		Some(value.session_id)
	};
}

macro_rules! define_operation {
	(
		$(#[$meta:meta])*
		$([[session_id($session_id:ident)]])?
		pub struct $name:ident {
			code: $code:literal,
			parameters: ($($param:ident: $ty:ty),* $(,)?),
			parameter_count: $parameter_count:literal,
			valid_reponse_codes: [$($error:expr),* $(,)?] $(,)?
		}
	) => {
		const _: () = {
			if counter([$(replace_expr!($param ())),*]) > MAX_PARAMETERS {
				panic!("Too many parameters");
			}

			if counter([$(replace_expr!($param ())),*]) != $parameter_count {
				panic!("Parameter count mismatch");
			}
		};

		$(#[$meta])*
		pub struct $name {
			parameters: [$crate::communication::Parameter; counter([$(replace_expr!($param ())),*])],
			session_id: Option<$crate::communication::SessionId>,
			transaction_id: $crate::communication::TransactionId,
		}

		impl $name {
			const OPCODE: u16 = $code;
			const VALID_ERRORS: &[$crate::communication::response::ErrorCode] = &[$($error),*];

			paste::paste! {
				pub fn new(transaction_id: $crate::communication::TransactionId, $($param: $ty),*) -> Self {
					Self {
						parameters: [$(Into::<$crate::communication::Parameter>::into($param)),*],
						session_id: session_id!($($session_id)?),
						transaction_id,
					}
				}
			}
		}

		impl From<$name> for $crate::communication::operation::Operation {
			fn from(value: $name) -> $crate::communication::operation::Operation {
				let mut parameters = [None::<$crate::communication::Parameter>; 5];
				::seq_macro::seq!(i in 0..$parameter_count {
					parameters[i] = Some(value.parameters[i]);
				});

				Self {
					code: <$name>::OPCODE,
					session_id: session_id!($($session_id)?),
					transaction_id: value.transaction_id,
					parameters,
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
		parameter_count: 0,
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
	[[session_id(false)]]
	pub struct OpenSession {
		code: 0x1002,
		parameters: (session_id: SessionId),
		parameter_count: 1,
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
		parameter_count: 0,
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
		parameter_count: 0,
		valid_reponse_codes: [
			ErrorCode::OperationNotSupported,
			ErrorCode::SessionNotOpen,
			ErrorCode::InvalidTransactionID,
			ErrorCode::ParameterNotSupported,
		]
	}
}
