mod error_impls;
mod impls;

pub use error_impls::*;
pub use impls::*;

use crate::communication::{SessionId, TransactionId};

use crate::communication::operation::Operation;
use core::error::Error;
use core::fmt::{self, Debug, Display};

pub const CODE_OK: u16 = 0x2001;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u16)]
pub enum ErrorCode {
	/// This response code is not used.
	Undefined = 0x2000,
	/// This operation did not complete, and the reason for the failure is not known.
	GeneralError = 0x2002,
	/// Indicates that the session handle identified by the operation dataset for this operation is
	/// not a currently open session.
	SessionNotOpen = 0x2003,
	/// Indicates that the [TransactionID] of this operation does not identify a valid transaction.
	InvalidTransactionID = 0x2004,
	/// Indicates that an Operation has been called with what appears to be a valid
	/// code, but the responder does not support the operation identified by that code. The
	/// initiator should only invoke operations contained in the responder’s DeviceInfo dataset,
	/// so this response should not normally be returned.
	OperationNotSupported = 0x2005,
	/// Indicates that a parameter of an operation contains a non-zero value, but is not supported.
	/// This response is different from [`InvalidParameter`](Self::InvalidParameter).
	ParameterNotSupported = 0x2006,
	/// This response shall be sent when a transfer did not complete successfully, and indicates
	/// that data transferred is to be discarded. This response shall not be sent if the transfer was
	/// cancelled by the Initiator.
	IncompleteTransfer = 0x2007,
	/// Indicates that one or more [StorageId]s sent as parameters of an operation do not refer to
	/// actual StorageIDs on the device.
	///
	/// [StorageId]: crate::device::storage::id::StorageId
	InvalidStorageID = 0x2008,
	/// Indicates that one or more [`ObjectHandle`]s sent as parameters of an operation do not refer
	/// to actual Objects on the device. The list of valid [`ObjectHandle`]s should be requested
	/// again, along with any appropriate ObjectInfo datasets.
	///
	/// [ObjectHandle]: crate::object::types::object_handle::ObjectHandle
	InvalidObjectHandle = 0x2009,
	/// Indicates that a [DevicePropCode] sent as a parameter of an operation appears to be a valid
	/// code, but is not supported by the device. The initiator should only attempt to work with
	/// Device Properties identified in the [DevicePropertiesSupported] field of the [DeviceInfo]
	/// Dataset, so this response should not normally be returned.
	DevicePropNotSupported = 0x200A,
	/// Indicates that the device does not support an [`ObjectFormatCode`] supplied in the given context.
	///
	/// [ObjectFormatCode]: crate::object::types::format_code::ObjectFormatCode
	InvalidObjectFormatCode = 0x200B,
	/// Indicates that a store identified in this operation is full, and this is preventing the
	/// successful completion of that operation.
	StoreFull = 0x200C,
	/// Indicates that an object referred to by the operation is write-protected.
	ObjectWriteProtected = 0x200D,
	/// Indicates that a store referred to by the operation is read-only.
	StoreReadOnly = 0x200E,
	/// This response shall be sent when access to data required by the operation is denied. This
	/// shall not be used when the device is busy, but to indicate that if the current state of the
	/// device does not change access will continue to be denied.
	AccessDenied = 0x200F,
	/// Indicates that a data object exists with the specified [`ObjectHandle`], but a thumbnail
	/// cannot be provided for that object.
	///
	/// [ObjectHandle]: crate::object::types::object_handle::ObjectHandle
	NoThumbnailPresent = 0x2010,
	/// This shall be sent when the device fails a device-specific self test.
	SelfTestFailed = 0x2011,
	/// Indicates that only a subset of the objects indicated for deletion were actually deleted.
	/// This could be caused by some of those objects being write-protected or on read-only stores.
	PartialDeletion = 0x2012,
	/// Indicates that the store indicated (or the store that contains the indicated object) is not
	/// physically available. This can be caused by media ejection. This response shall not be
	/// used to indicate that the store is busy.
	StoreNotAvailable = 0x2013,
	/// This response shall be sent when an operation attempts to specify an action only on
	/// objects which have a particular format code, but the responder does not support that
	/// capability. The operation should be attempted again without specifying by format. When
	/// this response is sent, it shall indicate that any future attempts to call the same operation
	/// specifying by format will also result in this response.
	SpecificationByFormatUnsupported = 0x2014,
	/// This shall be sent when a [SendObject] operation has been called without the initiator
	/// having previously sent a corresponding SendObjectInfo successfully. The initiator must
	/// successfully complete a [SendObjectInfo] operation before attempting another [SendObject] operation.
	NoValidObjectInfo = 0x2015,
	/// Indicates that a datacode used in this operation does not have the correct format, and is
	/// therefore known to be invalid. This response shall be used when the most-significant bits
	/// of a datacode does not have the format required for that type of code, and not when the
	/// data appears to have the correct type but is invalid for other reasons.
	InvalidCodeFormat = 0x2016,
	/// Indicates that the indicated data code has the correct format, but is in a vendor extension
	/// range not recognized by the device. This response will typically not occur, because the
	/// Initiator can identify the supported vendor extensions by examination of the DeviceInfo
	/// dataset.
	UnknownVendorCode = 0x2017,
	/// This shall be sent when an operation attempts to terminate a capture session, but that the
	/// capture session has already terminated. This response is only used for the
	/// TerminateOpenCapture operation, which is only used to terminate open-ended captures.
	CaptureAlreadyTerminated = 0x2018,
	/// This response shall be sent when the device is not currently able to process a request
	/// because it, or the specified store, is busy. This response implies that the operation may be
	/// successful at a later time, but is not possible right now. This response shall not be used to
	/// indicate that a store is physically unavailable.
	DeviceBusy = 0x2019,
	/// This response shall be sent when an indicated object is not of type Association, but is
	/// required to be of type Association in the context in which it is used, and therefore is not a
	/// valid ParentObject. This response is not intended to be used for specified [`ObjectHandle`]s
	/// that do not refer to valid objects, but only for [`ObjectHandle`]s which refer to actual objects
	/// which are not of type Association.
	///
	/// [ObjectHandle]: crate::object::types::object_handle::ObjectHandle
	InvalidParentObject = 0x201A,
	/// This response shall be sent when an attempt is made to set a DeviceProperty, but the
	/// DevicePropDesc dataset sent is not the correct size or format.
	InvalidDevicePropFormat = 0x201B,
	/// This response shall be sent when an attempt is made to set a DeviceProperty to a
	/// particular value, but that value is not allowed by the device.
	InvalidDevicePropValue = 0x201C,
	/// This response indicates that a parameter of the operation is not a valid value. This
	/// response is different from [`ParameterNotSupported`](Self::ParameterNotSupported), which indicates that no value was
	/// expected in this parameter.
	InvalidParameter = 0x201D,
	/// This response may be sent in resonse to an [OpenSession] operation. If multiple sessions
	/// are supported by the device, this response indicates that a session with the specified
	/// [SessionId] is already open. If multiple sessions are not supported by the device, this
	/// response indicates that a session is open and must be closed before another session can be
	/// opened.
	SessionAlreadyOpen = 0x201E,
	/// This response indicates that the operation was interrupted due to manual cancellation by
	/// the initiator.
	TransactionCancelled = 0x201F,
	/// This response may be sent as a response to a [SendObjectInfo] operation to indicate that
	/// the responder does not support the specification of destination. This response implies that
	/// any future attempts to specify the object destination will also fail with the same response.
	SpecificationOfDestinationUnsupported = 0x2020,
	/// Indicates that the device does not support the sent Object Property Code in this context.
	InvalidObjectPropCode = 0x2021,
	/// Indicates that an object property sent to the device is in an unsupported size or type.
	InvalidObjectPropFormat = 0x2022,
	/// Indicates that an object property sent to the device is the correct type, but contains a value
	/// which is not supported. The supported values shall be identified by the [ObjectPropDesc]
	/// dataset.
	InvalidObjectPropValue = 0x2023,
	/// Indicates that a sent Object Reference is invalid. Either the reference contains an object
	/// handle not present on the device, or the reference attempting to be set is unsupported in
	/// context.
	InvalidObjectReference = 0x2024,
	/// Indicates that the dataset sent in the data phase of this operation is invalid.
	InvalidDataset = 0x2025,
	/// May be used as the response to indicate that the responder does not support the
	/// specification of groups by the initiator. This response implies that the initiator should not
	/// attempt to specify the group code in any future operations, as they will also fail with the
	/// same response.
	SpecificationByGroupUnsupported = 0x2026,
	/// May be used as the response to indicate that the responder does not support the
	/// specification of depth by the initiator. This response implies that the initiator should not
	/// attempt to specify depth in any future call of the operation which resulted in this
	/// response, as they will also fail with the same response.
	SpecificationByDepthUnsupported = 0x2027,
	/// Indicates that the object desired to be sent cannot be stored in the filesystem of the
	/// device. This should not be used when there is insufficient space on the storage. For
	/// example, a FAT32 system can only support a 4GB object. A 6GB object would receive
	/// `ObjectTooLarge`.
	ObjectTooLarge = 0x2028,
	/// Indicates that an [ObjectPropCode] sent as a parameter of an operation appears to be a
	/// valid code, but is not supported by the device. The initiator should only attempt to work
	/// with Object Properties identified as supported by the responder, so this response should
	/// not normally be returned.
	ObjectPropNotSupported = 0x2029,
	/// Indicates that an Object Property group code sent as a parameter of an operation appears
	/// to be a valid code, but is not supported by the device. The initiator should only attempt to
	/// work with Object Property group codes identified as supported by the responder, so this
	/// response should not normally be returned.
	ObjectPropGroupNotSupported = 0x202A,

	/// **NOT PART OF THE SPEC**
	///
	/// This indicates that the responder sent an invalid error code in response to an operation.
	UnknownResponse(u16),
}

impl From<u16> for ErrorCode {
	fn from(code: u16) -> Self {
		match code {
			0x2000 => ErrorCode::Undefined,
			0x2002 => ErrorCode::GeneralError,
			0x2003 => ErrorCode::SessionNotOpen,
			0x2004 => ErrorCode::InvalidTransactionID,
			0x2005 => ErrorCode::OperationNotSupported,
			0x2006 => ErrorCode::ParameterNotSupported,
			0x2007 => ErrorCode::IncompleteTransfer,
			0x2008 => ErrorCode::InvalidStorageID,
			0x2009 => ErrorCode::InvalidObjectHandle,
			0x200A => ErrorCode::DevicePropNotSupported,
			0x200B => ErrorCode::InvalidObjectFormatCode,
			0x200C => ErrorCode::StoreFull,
			0x200D => ErrorCode::ObjectWriteProtected,
			0x200E => ErrorCode::StoreReadOnly,
			0x200F => ErrorCode::AccessDenied,
			0x2010 => ErrorCode::NoThumbnailPresent,
			0x2011 => ErrorCode::SelfTestFailed,
			0x2012 => ErrorCode::PartialDeletion,
			0x2013 => ErrorCode::StoreNotAvailable,
			0x2014 => ErrorCode::SpecificationByFormatUnsupported,
			0x2015 => ErrorCode::NoValidObjectInfo,
			0x2016 => ErrorCode::InvalidCodeFormat,
			0x2017 => ErrorCode::UnknownVendorCode,
			0x2018 => ErrorCode::CaptureAlreadyTerminated,
			0x2019 => ErrorCode::DeviceBusy,
			0x201A => ErrorCode::InvalidParentObject,
			0x201B => ErrorCode::InvalidDevicePropFormat,
			0x201C => ErrorCode::InvalidDevicePropValue,
			0x201D => ErrorCode::InvalidParameter,
			0x201E => ErrorCode::SessionAlreadyOpen,
			0x201F => ErrorCode::TransactionCancelled,
			0x2020 => ErrorCode::SpecificationOfDestinationUnsupported,
			0x2021 => ErrorCode::InvalidObjectPropCode,
			0x2022 => ErrorCode::InvalidObjectPropFormat,
			0x2023 => ErrorCode::InvalidObjectPropValue,
			0x2024 => ErrorCode::InvalidObjectReference,
			0x2025 => ErrorCode::InvalidDataset,
			0x2026 => ErrorCode::SpecificationByGroupUnsupported,
			0x2027 => ErrorCode::SpecificationByDepthUnsupported,
			0x2028 => ErrorCode::ObjectTooLarge,
			0x2029 => ErrorCode::ObjectPropNotSupported,
			0x202A => ErrorCode::ObjectPropGroupNotSupported,
			unknown => ErrorCode::UnknownResponse(unknown),
		}
	}
}

impl Display for ErrorCode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Undefined => write!(f, "An undefined error occurred"),
			Self::GeneralError => write!(f, "The operation failed for an unknown reason"),
			Self::SessionNotOpen => write!(f, "The session is not open"),
			Self::InvalidTransactionID => write!(f, "The transaction ID is invalid"),
			Self::OperationNotSupported => write!(f, "This operation is not supported"),
			Self::ParameterNotSupported => {
				write!(f, "One of the provided parameters is not supported")
			},
			Self::IncompleteTransfer => write!(f, "The transfer did not complete successfully"),
			Self::InvalidStorageID => write!(f, "One or more storage IDs are invalid"),
			Self::InvalidObjectHandle => write!(f, "One or more object handles are invalid"),
			Self::DevicePropNotSupported => {
				write!(f, "The provided device property is not supported")
			},
			Self::InvalidObjectFormatCode => {
				write!(f, "The provided object format code is not supported")
			},
			Self::StoreFull => write!(f, "The storage is full"),
			Self::ObjectWriteProtected => write!(f, "Attempted to write to write-protected object"),
			Self::StoreReadOnly => write!(f, "The storage is read-only"),
			Self::AccessDenied => write!(f, "Access to data is denied"),
			Self::NoThumbnailPresent => write!(f, "No thumbnail is present for the object"),
			Self::SelfTestFailed => write!(f, "The device failed a self test"),
			Self::PartialDeletion => write!(
				f,
				"Only a subset of objects were deleted, possibly due to write-protection"
			),
			Self::StoreNotAvailable => write!(f, "The store is not available"),
			Self::SpecificationByFormatUnsupported => {
				write!(f, "The operation does not support specifying by format")
			},
			Self::NoValidObjectInfo => write!(f, "No valid object info was provided"),
			Self::InvalidCodeFormat => write!(f, "A provided data code has an invalid format"),
			Self::UnknownVendorCode => write!(f, "The vendor code is not recognized by the device"),
			Self::CaptureAlreadyTerminated => {
				write!(f, "The capture session has already been terminated")
			},
			Self::DeviceBusy => write!(f, "The device is busy"),
			Self::InvalidParentObject => write!(f, "The parent object is invalid"),
			Self::InvalidDevicePropFormat => write!(f, "The device property format is invalid"),
			Self::InvalidDevicePropValue => write!(f, "The device property value is invalid"),
			Self::InvalidParameter => {
				write!(f, "One of the provided parameters has an invalid value")
			},
			Self::SessionAlreadyOpen => write!(f, "A session is already open"),
			Self::TransactionCancelled => {
				write!(f, "The transaction was cancelled by the initiator")
			},
			Self::SpecificationOfDestinationUnsupported => {
				write!(f, "The destination specification is not supported")
			},
			Self::InvalidObjectPropCode => {
				write!(f, "The object property code is invalid in this context")
			},
			Self::InvalidObjectPropFormat => write!(f, "The object property format is invalid"),
			Self::InvalidObjectPropValue => write!(f, "The object property value is invalid"),
			Self::InvalidObjectReference => write!(f, "The object reference is invalid"),
			Self::InvalidDataset => write!(f, "The dataset is invalid"),
			Self::SpecificationByGroupUnsupported => {
				write!(f, "Group specification is not supported")
			},
			Self::SpecificationByDepthUnsupported => {
				write!(f, "Depth specification is not supported")
			},
			Self::ObjectTooLarge => write!(f, "The object is too large to be stored"),
			Self::ObjectPropNotSupported => write!(f, "The object property is not supported"),
			Self::ObjectPropGroupNotSupported => {
				write!(f, "The object property group is not supported")
			},

			// **NOT PART OF THE SPEC**
			Self::UnknownResponse(code) => write!(
				f,
				"The responder provided an unknown response code: 0x{:04X}",
				code
			),
		}
	}
}

pub type Response<O: Operation> =
	Result<SuccessResponse<<O as Operation>::Response>, <O as Operation>::Error>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuccessResponse<T>
where
	T: Clone + Debug + Eq + PartialEq,
{
	pub data: T,
	pub transaction_id: TransactionId,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ErrorResponse {
	pub code: ErrorCode,
	pub transaction_id: TransactionId,
}

impl Display for ErrorResponse {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"Error response: code = {}, transaction_id = {}",
			self.code, self.transaction_id
		)
	}
}

impl Error for ErrorResponse {}

pub trait ResponseFlags: sealed::Sealed {
	/// Hint to the decoder whether to expect data with this response.
	const EXPECTS_DATA: bool = true;
}

mod sealed {
	pub trait Sealed {}

	impl Sealed for super::impls::Empty {}
}
