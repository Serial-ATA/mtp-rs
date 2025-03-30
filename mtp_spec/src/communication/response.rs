pub mod errors;
mod impls;

pub use impls::*;

use crate::communication::TransactionId;
use crate::communication::operation::DynOperation;

use core::error::Error;
use core::fmt::{self, Debug, Display};

/// The response code for a successful operation
pub const CODE_OK: u16 = 0x2001;

/// All error response codes
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u16)]
pub enum ErrorCode {
    /// This response code is not used.
    Undefined = 0x2000,
    /// This operation did not complete, and the reason for the failure is not known.
    GeneralError = 0x2002,
    /// The session handle for this operation is not a currently open session.
    SessionNotOpen = 0x2003,
    /// The [`TransactionId`] of this operation does not identify a valid transaction.
    InvalidTransactionID = 0x2004,
    /// An Operation has been called, but the responder does not support it.
    ///
    /// The initiator should only invoke operations contained in the responder’s DeviceInfo dataset,
    /// so this response should not normally be returned.
    OperationNotSupported = 0x2005,
    /// A parameter of an operation contains a non-zero value, but is not supported.
    ///
    /// This response is different from [`InvalidParameter`](Self::InvalidParameter).
    ParameterNotSupported = 0x2006,
    /// A transfer did not complete successfully, the data transferred is to be discarded.
    ///
    /// This response shall not be sent if the transfer was cancelled by the Initiator.
    IncompleteTransfer = 0x2007,
    /// One or more [`StorageId`]s sent as parameters of an operation do not refer to
    /// actual StorageIDs on the device.
    ///
    /// [`StorageId`]: crate::device::storage::id::StorageId
    InvalidStorageID = 0x2008,
    /// One or more [`ObjectHandle`]s sent as parameters of an operation do not refer
    /// to actual Objects on the device.
    ///
    /// The list of valid [`ObjectHandle`]s should be requested again, along with any appropriate
    /// [`ObjectInfo`] datasets.
    ///
    /// [`ObjectHandle`]: crate::object::types::ObjectHandle
    /// [`ObjectInfo`]: crate::object::info::ObjectInfo
    InvalidObjectHandle = 0x2009,
    /// A [`DevicePropCode`] sent in an operation is not supported by the device.
    ///
    /// The initiator should only attempt to work with
    /// Device Properties identified in the [DevicePropertiesSupported] field of the [`DeviceInfo`]
    /// Dataset, so this response should not normally be returned.
    ///
    /// [`DevicePropCode`]: crate::device::properties::code::DevicePropCode
    /// [`DeviceInfo`]: crate::device::info::DeviceInfo
    DevicePropNotSupported = 0x200A,
    /// The device does not support an [`ObjectFormatCode`] supplied in the given context.
    ///
    /// [`ObjectFormatCode`]: crate::object::types::ObjectFormatCode
    InvalidObjectFormatCode = 0x200B,
    /// A store identified in this operation is full
    StoreFull = 0x200C,
    /// An object referred to by the operation is write-protected.
    ObjectWriteProtected = 0x200D,
    /// A store referred to by the operation is read-only.
    StoreReadOnly = 0x200E,
    /// Access to data required by the operation is denied.
    ///
    /// This shall not be used when the device is busy, but to indicate that if the current state of
    /// the device does not change access will continue to be denied.
    AccessDenied = 0x200F,
    /// A data object exists with the specified [`ObjectHandle`], but a thumbnail cannot be provided
    /// for that object.
    ///
    /// [`ObjectHandle`]: crate::object::types::ObjectHandle
    NoThumbnailPresent = 0x2010,
    /// The device failed a device-specific self test.
    SelfTestFailed = 0x2011,
    /// Only a subset of the objects indicated for deletion were actually deleted.
    ///
    /// This could be caused by some of those objects being write-protected or on read-only stores.
    PartialDeletion = 0x2012,
    /// The store indicated (or the store that contains the indicated object) is not physically available.
    ///
    /// This can be caused by media ejection. This response shall not be used to indicate that the store is busy.
    StoreNotAvailable = 0x2013,
    /// The responder does not support specifying [`ObjectFormatCode`]s for this operation
    ///
    /// The operation should be attempted again without specifying by format.
    ///
    /// When this response is sent, it shall indicate that any future attempts to call the same operation
    /// specifying by format will also result in this response.
    ///
    /// [`ObjectFormatCode`]: crate::object::types::ObjectFormatCode
    SpecificationByFormatUnsupported = 0x2014,
    /// A [`SendObject`] operation has been called without the initiator having previously sent a corresponding [`SendObjectInfo`] successfully.
    ///
    /// The initiator must successfully complete a [`SendObjectInfo`] operation before attempting another [`SendObject`] operation.
    ///
    /// [`SendObject`]: crate::communication::operation::SendObject
    /// [`SendObjectInfo`]: crate::communication::operation::SendObjectInfo
    NoValidObjectInfo = 0x2015,
    /// A datacode used in this operation does not have the correct format.
    ///
    /// This response shall be used when the most-significant bits of a datacode does not have the
    /// format required for that type of code, and not when the data appears to have the correct type
    /// but is invalid for other reasons.
    InvalidCodeFormat = 0x2016,
    /// The indicated data code has the correct format, but is in a vendor extension
    /// range not recognized by the device.
    ///
    /// This response will typically not occur, because the Initiator can identify the supported
    /// vendor extensions by examination of the [`DeviceInfo`] dataset.
    ///
    /// [`DeviceInfo`]: crate::device::info::DeviceInfo
    UnknownVendorCode = 0x2017,
    /// An operation attempted to terminate a capture session, but that the
    /// capture session has already terminated.
    ///
    /// This response is only used for the [`TerminateOpenCapture`] operation, which is only used to
    /// terminate open-ended captures.
    ///
    /// [`TerminateOpenCapture`]: crate::communication::operation::TerminateOpenCapture
    CaptureAlreadyTerminated = 0x2018,
    /// The device is not currently able to process a request because it, or the specified store, is busy.
    ///
    /// This response implies that the operation may be successful at a later time, but is not possible
    /// right now. This response shall not be used to indicate that a store is physically unavailable.
    DeviceBusy = 0x2019,
    /// An indicated object is not of type [`Association`], but is required to be in the current context, and therefore is not a
    /// valid ParentObject.
    ///
    /// This response is not intended to be used for specified [`ObjectHandle`]s that do not refer to
    /// valid objects, but only for [`ObjectHandle`]s which refer to actual objects which are not of
    /// type [`Association`].
    ///
    /// [`Association`]: crate::object::types::ObjectFormatCode::Association
    /// [`ObjectHandle`]: crate::object::types::ObjectHandle
    InvalidParentObject = 0x201A,
    /// An attempt is made to set a [DeviceProperty], but the [`DevicePropDesc`] dataset sent is not the correct size or format.
    InvalidDevicePropFormat = 0x201B,
    /// An attempt is made to set a [DeviceProperty] to a particular value, but that value is not allowed by the device.
    InvalidDevicePropValue = 0x201C,
    /// A parameter of the operation is not a valid value.
    ///
    /// This response is different from [`ParameterNotSupported`](Self::ParameterNotSupported), which
    /// indicates that no value was expected in this parameter.
    InvalidParameter = 0x201D,
    /// A response to an [`OpenSession`] operation.
    ///
    /// If multiple sessions are supported by the device, this response indicates that a session with
    /// the specified [`SessionId`] is already open.
    ///
    /// If multiple sessions are not supported by the device, this response indicates that a session
    /// is open and must be closed before another session can be opened.
    ///
    /// [`OpenSession`]: crate::communication::operation::OpenSession
    /// [`SessionId`]: crate::communication::SessionId
    SessionAlreadyOpen = 0x201E,
    /// This response indicates that the operation was interrupted due to manual cancellation by
    /// the initiator.
    TransactionCancelled = 0x201F,
    /// A response to a [`SendObjectInfo`] operation to indicate that the responder does not support the specification of destination.
    ///
    /// This response implies that any future attempts to specify the object destination will also
    /// fail with the same response.
    SpecificationOfDestinationUnsupported = 0x2020,
    /// The device does not support the sent [`ObjectPropertyCode`] in this context.
    ///
    /// [`ObjectPropertyCode`]: crate::object::types::properties::ObjectPropertyCode
    InvalidObjectPropCode = 0x2021,
    /// An object property sent to the device is in an unsupported size or type.
    InvalidObjectPropFormat = 0x2022,
    /// An object property sent to the device is the correct type, but contains a value which is not supported.
    ///
    /// The supported values shall be identified by the [ObjectPropDesc] dataset.
    ///
    /// [`ObjectPropDesc`]: crate::object::types::ObjectPropDesc
    InvalidObjectPropValue = 0x2023,
    /// A sent Object Reference is invalid.
    ///
    /// Either the reference contains an object handle not present on the device, or the reference
    /// attempting to be set is unsupported in context.
    InvalidObjectReference = 0x2024,
    /// The dataset sent in the data phase of this operation is invalid.
    InvalidDataset = 0x2025,
    /// The responder does not support the specification of groups by the initiator.
    ///
    /// This response implies that the initiator should not attempt to specify the group code in any
    /// future operations, as they will also fail with the same response.
    SpecificationByGroupUnsupported = 0x2026,
    /// The responder does not support the specification of depth by the initiator.
    ///
    /// This response implies that the initiator should not attempt to specify depth in any future
    /// call of the operation which resulted in this response, as they will also fail with the same response.
    SpecificationByDepthUnsupported = 0x2027,
    /// The object desired to be sent cannot be stored in the filesystem of the device.
    ///
    /// This does not necessarily mean there is insufficient space on the storage. For example, a FAT32
    /// system can only support a 4GB object. A 6GB object would receive `ObjectTooLarge`.
    ObjectTooLarge = 0x2028,
    /// An [`ObjectPropertyCode`] sent in an operation is not supported by the device.
    ///
    /// The initiator should only attempt to work with Object Properties identified as supported by
    /// the responder, so this response should not normally be returned.
    ///
    /// [`ObjectPropertyCode`]: crate::object::types::properties::ObjectPropertyCode
    ObjectPropNotSupported = 0x2029,
    /// An Object Property group code sent in an operation is not supported by the device.
    ///
    /// The initiator should only attempt to work with Object Property group codes identified as
    /// supported by the responder, so this response should not normally be returned.
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

/// The result of a successful or failed operation
///
/// See [`SuccessResponse`] and [`ErrorResponse`]
pub type Response<O> =
    Result<SuccessResponse<<O as DynOperation>::Response>, <O as DynOperation>::Error>;

/// The result of a successful operation
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuccessResponse<T>
where
    T: Clone + Debug + Eq + PartialEq,
{
    /// The data returned by the responder
    pub data: T,
    /// The transaction ID of the operation
    pub transaction_id: TransactionId,
}

/// The result of a failed operation
///
/// This includes the transaction ID of the operation that failed
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ErrorResponse {
    /// The error code returned by the responder
    pub code: ErrorCode,
    /// The transaction ID of the operation that failed
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
