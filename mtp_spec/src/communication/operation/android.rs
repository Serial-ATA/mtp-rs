//! Android-specific operation implementations

use crate::communication::operation::DataDirection;
use crate::communication::operation::impls::define_operations;
use crate::communication::response;
use crate::object::types::ObjectHandle;

define_operations! {
    OPCODE_ENUM: AndroidOperation;

    /// 64-bit variant of [`GetPartialObject`]
    ///
    /// [`GetPartialObject`]: crate::device::info::DeviceInfo
    pub struct GetPartialObject64 {
        code: 0x95C1,
        visible_parameters: (object: ObjectHandle, @RAW(true) offset_high: u32, @RAW(true) offset_low: u32, @RAW(true) len: u32),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::Empty,
        valid_error_codes: [ParameterNotSupported]
    }

    /// Write to a region of an object
    ///
    /// NOTE: An edit session must be started first, see [`BeginEditObject`].
    pub struct SendPartialObject {
        code: 0x95C2,
        visible_parameters: (object: ObjectHandle, @RAW(true) offset_high: u32, @RAW(true) offset_low: u32, @RAW(true) len: u32),
        data_direction: Some(DataDirection::InitiatorToResponder),
        response: response::android::SendPartialObject,
        valid_error_codes: [ParameterNotSupported]
    }

    /// Truncate an object to the given 64-bit size
    ///
    /// NOTE: An edit session must be started first, see [`BeginEditObject`].
    pub struct TruncateObject {
        code: 0x95C3,
        visible_parameters: (object: ObjectHandle, @RAW(true) size_high: u32, @RAW(true) size_low: u32),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [ParameterNotSupported]
    }

    /// Starts an edit session for the given `object`
    ///
    /// During the session, the [`SendPartialObject`] and [`TruncateObject`] operations can be used.
    /// To actually commit the changes, an [`EndEditObject`] operation must be sent.
    pub struct BeginEditObject {
        code: 0x95C4,
        visible_parameters: (object: ObjectHandle),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [ParameterNotSupported]
    }

    /// Ends an edit session for the given `object`
    pub struct EndEditObject {
        code: 0x95C5,
        visible_parameters: (object: ObjectHandle),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [ParameterNotSupported]
    }
}
