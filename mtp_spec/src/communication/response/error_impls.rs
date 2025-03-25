use crate::communication::response::impls::{define_response, replace_expr, MAX_PARAMETERS};
use crate::communication::SessionId;

macro_rules! define_error_response {
	(
		$(#[$meta:meta])*
		[[error($error_msg:literal)]]
		pub struct $name:ident {
			code: $code:literal,
			$(data: $data:ty,)?
			$(parameters: ($($param:ident: $ty:ty),* $(,)?),)?
		}
	) => {
		define_response!(
			$(#[$meta])*
			pub struct $name {
				$(data: $data,)?
				$(parameters: ($($param: $ty),*),)?
			}
		);

		impl $name {
			pub const CODE: u16 = $code;
		}

		impl core::fmt::Display for $name {
			fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
				write!(
					f,
					"Error (code = {}): {}",
					Self::CODE, $error_msg
				)
			}
		}

		impl core::error::Error for $name {}
	}
}

define_error_response! {
	/// This response code is not used.
	[[error("This response code is not used")]]
	pub struct Undefined {
		code: 0x2000,
	}
}

define_error_response! {
	/// This operation did not complete, and the reason for the failure is not known.
	[[error("This operation did not complete, and the reason for the failure is not known")]]
	pub struct GeneralError {
		code: 0x2002,
	}
}

define_error_response! {
	/// Indicates that the session handle identified by the operation dataset for this operation is
	/// not a currently open session.
	[[error("The session handle identified by the operation dataset for this operation is not a currently open session")]]
	pub struct SessionNotOpen {
		code: 0x2003,
		parameters: (session_id: SessionId),
	}
}

define_error_response! {
	/// Indicates that the [TransactionID] of this operation does not identify a valid transaction.
	[[error("The transaction ID of this operation does not identify a valid transaction")]]
	pub struct InvalidTransactionID {
		code: 0x2004,
	}
}

define_error_response! {
	/// Indicates that an Operation has been called with what appears to be a valid
	/// code, but the responder does not support the operation identified by that code. The
	/// initiator should only invoke operations contained in the responder’s DeviceInfo dataset,
	/// so this response should not normally be returned.
	[[error("The operation is not supported by the device")]]
	pub struct OperationNotSupported {
		code: 0x2005,
	}
}

define_error_response! {
	/// Indicates that a parameter of an operation contains a non-zero value, but is not supported.
	/// This response is different from [`InvalidParameter`](Self::InvalidParameter).
	[[error("The parameter is not supported")]]
	pub struct ParameterNotSupported {
		code: 0x2006,
	}
}

define_error_response! {
	/// This response shall be sent when a transfer did not complete successfully, and indicates
	/// that data transferred is to be discarded. This response shall not be sent if the transfer was
	/// cancelled by the Initiator.
	[[error("The transfer did not complete successfully")]]
	pub struct IncompleteTransfer {
		code: 0x2007,
	}
}

define_error_response! {
	/// Indicates that one or more [StorageId]s sent as parameters of an operation do not refer to
	/// actual StorageIDs on the device.
	[[error("The storage ID is not valid")]]
	pub struct InvalidStorageId {
		code: 0x2008,
		parameters: (storage_id: u32),
	}
}

define_error_response! {
	/// Indicates that one or more [`ObjectHandle`]s sent as parameters of an operation do not refer
	/// to actual Objects on the device. The list of valid [`ObjectHandle`]s should be requested
	/// again, along with any appropriate ObjectInfo datasets.
	[[error("The object handle is not valid")]]
	pub struct InvalidObjectHandle {
		code: 0x2009,
		parameters: (object_handle: u32),
	}
}

define_error_response! {
	/// Indicates that a [DevicePropCode] sent as a parameter of an operation appears to be a valid
	/// code, but is not supported by the device. The initiator should only attempt to work with
	/// Device Properties identified in the [DevicePropertiesSupported] field of the [DeviceInfo]
	/// Dataset, so this response should not normally be returned.
	[[error("The device property is not supported")]]
	pub struct DevicePropNotSupported {
		code: 0x200A,
		parameters: (device_prop_code: u32),
	}
}

define_error_response! {
	/// Indicates that the device does not support an [`ObjectFormatCode`] supplied in the given context.
	[[error("The object format code is not supported")]]
	pub struct InvalidObjectFormatCode {
		code: 0x200B,
		parameters: (object_format_code: u32),
	}
}

define_error_response! {
	/// Indicates that a store identified in this operation is full, and this is preventing the
	/// successful completion of that operation.
	[[error("The store is full")]]
	pub struct StoreFull {
		code: 0x200C,
		parameters: (storage_id: u32),
	}
}

define_error_response! {
	/// Indicates that an object referred to by the operation is write-protected.
	[[error("The object is write-protected")]]
	pub struct ObjectWriteProtected {
		code: 0x200D,
		parameters: (object_handle: u32),
	}
}

define_error_response! {
	/// Indicates that a store referred to by the operation is read-only
	[[error("The store is read-only")]]
	pub struct StoreReadOnly {
		code: 0x200E,
		parameters: (storage_id: u32),
	}
}

define_error_response! {
	/// Indicates that the device does not have permission to access the specified storage.
	[[error("The device does not have permission to access the storage")]]
	pub struct AccessDenied {
		code: 0x200F,
		parameters: (storage_id: u32),
	}
}

define_error_response! {
	/// This response shall be sent when the device is not currently able to process a request
	/// because it, or the specified store, is busy. This response implies that the operation may be
	/// successful at a later time, but is not possible right now. This response shall not be used to
	/// indicate that a store is physically unavailable.
	[[error("The device is busy")]]
	pub struct DeviceBusy {
		code: 0x2019,
	}
}

define_error_response! {
	[[error("The parameter is not valid")]]
	pub struct InvalidParameter {
		code: 0x201D,
	}
}

define_error_response! {
	/// A session with the specified [`SessionId`] is already open.
	[[error("The session is already open")]]
	pub struct SessionAlreadyOpen {
		code: 0x201E,
		parameters: (session_id: SessionId),
	}
}
