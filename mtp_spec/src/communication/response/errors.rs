//! MTP error responses

use crate::communication::SessionId;
use crate::communication::response::impls::define_response;
use crate::device::storage::StorageId;
use crate::object::properties::ObjectPropertyCode;
use crate::object::{ObjectFormatCode, ObjectHandle};

use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek};
use deku::reader::Reader;
use deku::{DekuError, DekuReader};

macro_rules! define_error_responses {
	(
		$(
			$(#[$meta:meta])*
			[[error($error_msg:literal)]]
			pub struct $name:ident {
				code: $code:literal,
				$($param:ident: $ty:ty),* $(,)?
			}
		)*
	) => {
		$(
			define_response!(
				$(#[$meta])*
				pub struct $name[][] {
					$($param: $ty),*
				}
			);

			impl $name {
				/// The raw datacode for this error
				pub const CODE: u16 = $code;
			}

			impl core::fmt::Display for $name {
				fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
					write!(f, "{}", ErrorCode::$name)
				}
			}

			impl core::error::Error for $name {}
		)*

		/// All defined operation error codes
		#[derive(Clone, Debug)]
		pub enum OperationError {
			$(
			$(#[$meta])*
			$name($name),
			)*
		}

		impl<'a> DekuReader<'a, (Endian, u16)> for OperationError {
			fn from_reader_with_ctx<R: Read + Seek>(
				reader: &mut Reader<R>,
				(endian, code): (Endian, u16),
			) -> Result<Self, DekuError>
			where
				Self: Sized,
			{
				match code {
					$(
					$code => Ok(OperationError::$name($name::from_reader_with_ctx(reader, endian)?)),
					)*
					code => Err(DekuError::Parse(format!("The responder provided an unknown response code: {code:#04X}").into())),
				}
			}
		}

		impl core::fmt::Display for OperationError {
			fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
				match self {
					$(
					Self::$name(v) => v.fmt(f),
					)*
				}
			}
		}

		impl core::error::Error for OperationError {}

		impl OperationError {
			/// Get the raw error code for this error
			pub fn code(&self) -> ErrorCode {
				match self {
					$(
					Self::$name(_) => ErrorCode::$name,
					)*
				}
			}
		}

		/// All error response codes
		#[allow(missing_docs)]
		#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
		#[repr(u16)]
		pub enum ErrorCode {
			$(
			$name = $code,
			)*
		}

		impl TryFrom<u16> for ErrorCode {
			type Error = ();

			fn try_from(value: u16) -> Result<Self, Self::Error> {
				match value {
					$(
					$code => Ok(ErrorCode::$name),
					)*
					_ => Err(()),
				}
			}
		}

		impl core::fmt::Display for ErrorCode {
			fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
				match self {
					$(
					Self::$name => write!(
						f,
						"Error (code = {:#X}): {}",
						$code, $error_msg
					),
					)*
				}
			}
		}

		impl core::error::Error for ErrorCode {}
	}
}

define_error_responses! {
    /// This response code is not used.
    [[error("This response code is not used")]]
    pub struct Undefined {
        code: 0x2000,
    }

    /// This operation did not complete, and the reason for the failure is not known.
    [[error("This operation did not complete, and the reason for the failure is not known")]]
    pub struct GeneralError {
        code: 0x2002,
    }

    /// Indicates that the session handle identified by the operation dataset for this operation is
    /// not a currently open session.
    [[error("The session handle identified by the operation dataset for this operation is not a currently open session")]]
    pub struct SessionNotOpen {
        code: 0x2003,
        session_id: SessionId,
    }

    /// Indicates that the [`TransactionID`] of this operation does not identify a valid transaction.
    ///
    /// [`TransactionID`]: crate::communication::TransactionId
    [[error("The transaction ID of this operation does not identify a valid transaction")]]
    pub struct InvalidTransactionId {
        code: 0x2004,
    }

    /// Indicates that an Operation has been called with what appears to be a valid
    /// code, but the responder does not support the operation identified by that code. The
    /// initiator should only invoke operations contained in the responder’s [`DeviceInfo`] dataset,
    /// so this response should not normally be returned.
    ///
    /// [`DeviceInfo`]: crate::device::info::DeviceInfo
    [[error("The operation is not supported by the device")]]
    pub struct OperationNotSupported {
        code: 0x2005,
    }

    /// Indicates that a parameter of an operation contains a non-zero value, but is not supported.
    ///
    /// This response is different from [`InvalidParameter`], which indicates that the parameter is
    /// supported, but contains an invalid value.
    [[error("The parameter is not supported")]]
    pub struct ParameterNotSupported {
        code: 0x2006,
    }

    /// This response shall be sent when a transfer did not complete successfully, and indicates
    /// that data transferred is to be discarded. This response shall not be sent if the transfer was
    /// cancelled by the Initiator.
    [[error("The transfer did not complete successfully")]]
    pub struct IncompleteTransfer {
        code: 0x2007,
    }

    /// Indicates that one or more [`StorageId`]s sent as parameters of an operation do not refer to
    /// actual [`StorageId`]s on the device.
    [[error("The storage ID is not valid")]]
    pub struct InvalidStorageId {
        code: 0x2008,
    }

    /// Indicates that one or more [`ObjectHandle`]s sent in the operation do not refer
    /// to actual Objects on the device.
    ///
    /// The list of valid [`ObjectHandle`]s should be requested again, along with any appropriate
    /// [`ObjectInfo`] datasets.
    ///
    /// [`ObjectInfo`]: crate::object::info::ObjectInfo
    [[error("The object handle is not valid")]]
    pub struct InvalidObjectHandle {
        code: 0x2009,
    }

    /// Indicates that a [`DevicePropertyCode`] sent as a parameter of an operation is not supported by the device.
    ///
    /// The initiator should only attempt to work with device properties identified in the `device_properties_supported` field of the [`DeviceInfo`]
    /// Dataset, so this response should not normally be returned.
    ///
    /// [`DevicePropertyCode`]: crate::device::properties::DevicePropertyCode
    /// [`DeviceInfo`]: crate::device::info::DeviceInfo
    [[error("The device property is not supported")]]
    pub struct DevicePropNotSupported {
        code: 0x200A,
        device_prop_code: ObjectPropertyCode,
    }

    /// Indicates that the device does not support an [`ObjectFormatCode`] supplied in the given context.
    [[error("The object format code is not supported")]]
    pub struct InvalidObjectFormatCode {
        code: 0x200B,
        object_format_code: ObjectFormatCode,
    }

    /// Indicates that a store identified in this operation is full, and this is preventing the
    /// successful completion of that operation.
    [[error("The store is full")]]
    pub struct StoreFull {
        code: 0x200C,
        storage_id: StorageId,
    }

    /// Indicates that an object referred to by the operation is write-protected.
    [[error("The object is write-protected")]]
    pub struct ObjectWriteProtected {
        code: 0x200D,
        object_handle: ObjectHandle,
    }

    /// Indicates that a store referred to by the operation is read-only
    [[error("The store is read-only")]]
    pub struct StoreReadOnly {
        code: 0x200E,
        storage_id: StorageId,
    }

    /// Indicates that the device does not have permission to access the specified storage.
    [[error("The device does not have permission to access the storage")]]
    pub struct AccessDenied {
        code: 0x200F,
        storage_id: StorageId,
    }

    /// Indicates that the specified object exists, but a thumbnail cannot be provided.
    [[error("The object does not have a thumbnail")]]
    pub struct NoThumbnailPresent {
        code: 0x2010,
        handle: ObjectHandle,
    }

    /// Indicates that the device failed a device-specific self test.
    [[error("The device failed a self test")]]
    pub struct SelfTestFailed {
        code: 0x2011,
    }

    /// Indicates that only a subset of the requested objects were deleted.
    ///
    /// This could be caused by some of those objects being write-protected or on read-only stores.
    [[error("Some objects could not be deleted")]]
    pub struct PartialDeletion {
        code: 0x2012,
    }

    /// Indicates that the requested store is not physically available.
    ///
    /// This can be caused by media ejection.
    [[error("The store is not available")]]
    pub struct StoreNotAvailable {
        code: 0x2013,
    }

    /// Indicates that the responder does not support operations with [`ObjectFormatCode`]s.
    ///
    /// The operation should be attempted again without specifying by format. When
    /// this response is sent, it shall indicate that any future attempts to call the same operation
    /// specifying by format will also result in this response.
    [[error("The device does not support operations by object format")]]
    pub struct SpecificationByFormatUnsupported {
        code: 0x2014,
    }

    /// Indicates that a [`SendObject`] operation was received without a corresponding [`SendObjectInfo`]
    /// operation.
    ///
    /// The initiator must successfully complete a [`SendObjectInfo`] operation before attempting another
    /// [`SendObject`] operation.
    ///
    /// [`SendObject`]: crate::communication::operation::SendObject
    /// [`SendObjectInfo`]: crate::communication::operation::SendObjectInfo
    [[error("No `SendObjectInfo` operation has been received")]]
    pub struct NoValidObjectInfo {
        code: 0x2015,
    }

    /// Indicates that a specified datacode is malformed
    [[error("Malformed datacode")]]
    pub struct InvalidCodeFormat {
        code: 0x2016,
    }

    /// Indicates that a vendor-specific datacode was provided, and is unrecognized by the device.
    [[error("Unrecognized vendor-specific datacode")]]
    pub struct UnknownVendorCode {
        code: 0x2017,
    }

    /// Indicates that a capture session is already terminated.
    [[error("The capture session is already terminated")]]
    pub struct CaptureAlreadyTerminated {
        code: 0x2018,
    }

    /// This response shall be sent when the device is not currently able to process a request
    /// because it, or the specified store, is busy. This response implies that the operation may be
    /// successful at a later time, but is not possible right now. This response shall not be used to
    /// indicate that a store is physically unavailable.
    [[error("The device is busy")]]
    pub struct DeviceBusy {
        code: 0x2019,
    }

    /// Indicates that the specified object is not of type [`ObjectFormatCode::Association`].
    ///
    /// This indicates that the object exists, but is expected to be of type Association in this context.
    [[error("The object is not an association")]]
    pub struct InvalidParentObject {
        code: 0x201A,
    }

    /// Malformed [`DevicePropDesc`]
    ///
    /// [`DevicePropDesc`]: crate::device::properties::DevicePropDesc
    [[error("The device property description is malformed")]]
    pub struct InvalidDevicePropFormat {
        code: 0x201B,
    }

    /// The device does not allow setting the specified [`PropertyValue`]
    ///
    /// [`PropertyValue`]: crate::object::types::PropertyValue
    [[error("The device does not allow setting the specified property value")]]
    pub struct InvalidDevicePropValue {
        code: 0x201C,
    }

    /// A parameter of the operation is not a valid value
    ///
    /// This is different from [`ParameterNotSupported`], which indicates that no value was expected
    /// in this parameter.
    [[error("The parameter is not valid")]]
    pub struct InvalidParameter {
        code: 0x201D,
    }

    /// A session with the specified [`SessionId`] is already open.
    [[error("The session is already open")]]
    pub struct SessionAlreadyOpen {
        code: 0x201E,
        session_id: SessionId,
    }

    /// The initiator manually cancelled the transaction.
    [[error("The transaction was cancelled")]]
    pub struct TransactionCancelled {
        code: 0x201F,
    }

    /// The responder does not support specifying object destinations.
    ///
    /// This response implies that any future attempts to specify the object destination will also
    /// fail with the same response.
    [[error("Specification of object destinations is not supported")]]
    pub struct SpecificationOfDestinationUnsupported {
        code: 0x2020,
    }

    /// The device does not support the sent Object Property Code in this context
    [[error("The device does not support the sent Object Property Code in this context")]]
    pub struct InvalidObjectPropCode {
        code: 0xA801,
    }

    /// An object property sent to the device is in an unsupported size or type
    [[error("An object property sent to the device is in an unsupported size or type")]]
    pub struct InvalidObjectPropFormat {
        code: 0xA802,
    }

    /// An object is of the correct type, but contains a value which is not supported
    [[error("An object is of the correct type, but contains a value which is not supported")]]
    pub struct InvalidObjectPropValue {
        code: 0xA803,
    }

    /// An object reference is invalid, either by it not being present or by it not being supported
    /// by the device.
    [[error("An object reference is invalid")]]
    pub struct InvalidObjectReference {
        code: 0xA804,
    }

    /// The provided object property group is not supported by the device
    [[error("The provided object property group is not supported by the device")]]
    pub struct GroupNotSupported {
        code: 0xA805,
    }

    /// The dataset sent in the data phase of this operation is invalid
    [[error("The dataset sent in the data phase of this operation is invalid")]]
    pub struct InvalidDataset {
        code: 0xA806,
    }

    /// The responder does not support the specification of groups by the initiator
    ///
    /// This response implies that the initiator should not attempt to specify the group code in any
    /// future operations, as they will also fail with the same response.
    [[error("The responder does not support the specification of groups by the initiator")]]
    pub struct SpecificationByGroupUnsupported {
        code: 0xA807,
    }

    /// The responder does not support the specification of depth by the initiator
    ///
    /// This response implies that the initiator should not attempt to specify depth in any future
    /// call of the operation which resulted in this response, as they will also fail with the same response.
    [[error("The responder does not support the specification of depth by the initiator")]]
    pub struct SpecificationByDepthUnsupported {
        code: 0xA808,
    }

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

    /// The provided object property is not supported by the device
    [[error("The provided object property is not supported by the device")]]
    pub struct ObjectPropNotSupported {
        code: 0xA80A,
    }
}
