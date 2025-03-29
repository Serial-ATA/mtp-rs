use crate::communication::SessionId;
use crate::communication::response::impls::{MAX_PARAMETERS, define_response, replace_expr};
use crate::object::types::{ObjectFormatCode, ObjectHandle};

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
			pub struct $name[][] {
				$(data: $data,)?
				$(parameters: ($($param: $ty),*),)?
			}
		);

		impl $name {
			/// The raw datacode for this error
			pub const CODE: u16 = $code;
		}

		impl core::fmt::Display for $name {
			fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
				write!(
					f,
					"Error (code = {}): {}",
					$code, $error_msg
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
	pub struct InvalidTransactionId {
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
	/// Indicates that the specified object exists, but a thumbnail cannot be provided.
	[[error("The object does not have a thumbnail")]]
	pub struct NoThumbnailPresent {
		code: 0x2010,
		parameters: (handle: ObjectHandle),
	}
}

define_error_response! {
	/// Indicates that the device failed a device-specific self test.
	[[error("The device failed a self test")]]
	pub struct SelfTestFailed {
		code: 0x2011,
	}
}

define_error_response! {
	/// Indicates that only a subset of the requested objects were deleted.
	///
	/// This could be caused by some of those objects being write-protected or on read-only stores.
	[[error("Some objects could not be deleted")]]
	pub struct PartialDeletion {
		code: 0x2012,
	}
}

define_error_response! {
	/// Indicates that the requested store is not physically available.
	///
	/// This can be caused by media ejection.
	[[error("The store is not available")]]
	pub struct StoreNotAvailable {
		code: 0x2013,
	}
}

define_error_response! {
	/// Indicates that the responder does not support operations with [`ObjectFormatCode`]s.
	///
	/// The operation should be attempted again without specifying by format. When
	/// this response is sent, it shall indicate that any future attempts to call the same operation
	/// specifying by format will also result in this response.
	[[error("The device does not support operations by object format")]]
	pub struct SpecificationByFormatUnsupported {
		code: 0x2014,
	}
}

define_error_response! {
	/// Indicates that a [`SendObject`] operation was received without a corresponding [`SendObjectInfo`]
	/// operation.
	///
	/// The initiator must successfully complete a SendObjectInfo operation before attempting another
	/// SendObject operation.
	[[error("No `SendObjectInfo` operation has been received")]]
	pub struct NoValidObjectInfo {
		code: 0x2015,
	}
}

define_error_response! {
	/// Indicates that a specified datacode is malformed
	[[error("Malformed datacode")]]
	pub struct InvalidCodeFormat {
		code: 0x2016,
	}
}

define_error_response! {
	/// Indicates that a vendor-specific datacode was provided, and is unrecognized by the device.
	[[error("Unrecognized vendor-specific datacode")]]
	pub struct UnknownVendorCode {
		code: 0x2017,
	}
}

define_error_response! {
	/// Indicates that a capture session is already terminated.
	[[error("The capture session is already terminated")]]
	pub struct CaptureAlreadyTerminated {
		code: 0x2018,
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
	/// Indicates that the specified object is not of type [`ObjectFormatCode::Association`].
	///
	/// This indicates that the object exists, but is expected to be of type Association in this context.
	[[error("The object is not an association")]]
	pub struct InvalidParentObject {
		code: 0x201A,
	}
}

define_error_response! {
	/// Malformed [`DevicePropFormat`]
	[[error("The object is not an association")]]
	pub struct InvalidDevicePropFormat {
		code: 0x201B,
	}
}

define_error_response! {
	/// Malformed [`PropertyValue`]
	[[error("The object is not an association")]]
	pub struct InvalidDevicePropValue {
		code: 0x201C,
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

define_error_response! {
	/// The initiator manually cancelled the transaction.
	[[error("The transaction was cancelled")]]
	pub struct TransactionCancelled {
		code: 0x201F,
	}
}

define_error_response! {
	/// The responder does not support specifying object destinations.
	///
	/// This response implies that any future attempts to specify the object destination will also
	/// fail with the same response.
	[[error("Specification of object destinations is not supported")]]
	pub struct SpecificationOfDestinationUnsupported {
		code: 0x2020,
	}
}

define_error_response! {
	/// The device does not support the sent Object Property Code in this context
	[[error("The device does not support the sent Object Property Code in this context")]]
	pub struct InvalidObjectPropCode {
		code: 0xA801,
	}
}

define_error_response! {
	/// An object property sent to the device is in an unsupported size or type
	[[error("An object property sent to the device is in an unsupported size or type")]]
	pub struct InvalidObjectPropFormat {
		code: 0xA802,
	}
}

define_error_response! {
	/// An object is of the correct type, but contains a value which is not supported
	[[error("An object is of the correct type, but contains a value which is not supported")]]
	pub struct InvalidObjectPropValue {
		code: 0xA803,
	}
}

define_error_response! {
	/// An object reference is invalid, either by it not being present or by it not being supported
	/// by the device.
	[[error("An object reference is invalid")]]
	pub struct InvalidObjectReference {
		code: 0xA804,
	}
}

define_error_response! {
	/// The provided object property group is not supported by the device
	[[error("The provided object property group is not supported by the device")]]
	pub struct GroupNotSupported {
		code: 0xA805,
	}
}

define_error_response! {
	/// The dataset sent in the data phase of this operation is invalid
	[[error("The dataset sent in the data phase of this operation is invalid")]]
	pub struct InvalidDataset {
		code: 0xA806,
	}
}

define_error_response! {
	/// The responder does not support the specification of groups by the initiator
	///
	/// This response implies that the initiator should not attempt to specify the group code in any
	/// future operations, as they will also fail with the same response.
	[[error("The responder does not support the specification of groups by the initiator")]]
	pub struct SpecificationByGroupUnsupported {
		code: 0xA807,
	}
}

define_error_response! {
	/// The responder does not support the specification of depth by the initiator
	///
	/// This response implies that the initiator should not attempt to specify depth in any future
	/// call of the operation which resulted in this response, as they will also fail with the same response.
	[[error("The responder does not support the specification of depth by the initiator")]]
	pub struct SpecificationByDepthUnsupported {
		code: 0xA808,
	}
}

define_error_response! {
	/// The object desired to be sent cannot be stored in the filesystem of the device
	///
	/// This implies that the object is too large for the filesystem, ***not*** that the device does not
	/// have adequate space to store the object.
	///
	/// For example, a FAT32 system can only support a 4GB object. A 6GB object would receive `ObjectTooLarge`.
	[[error("The responder does not support the specification of depth by the initiator")]]
	pub struct ObjectTooLarge {
		code: 0xA809,
	}
}

define_error_response! {
	/// The provided object property is not supported by the device
	[[error("The provided object property is not supported by the device")]]
	pub struct ObjectPropNotSupported {
		code: 0xA80A,
	}
}
