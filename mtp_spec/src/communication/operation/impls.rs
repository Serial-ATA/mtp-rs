use crate::communication::{Parameter, SessionId, TransactionId, response};
use crate::device::properties::DeviceProperty;
use crate::device::storage::id::StorageId;
use crate::device::storage::info::FilesystemType;
use crate::object::info::ProtectionStatus;
use crate::object::types::properties::{ObjectProperty, ObjectPropertyCode};
use crate::object::types::{ObjectFormatCode, ObjectHandle};

use deku::{DekuRead, DekuWrite};

/// The direction in which data is transferred in an operation
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DataDirection {
    /// The responder sends data to the initiator
    ///
    /// This is the most common direction
    ResponderToInitiator,
    /// The initiator sends data to the responder
    ///
    /// This is used for setter operations
    InitiatorToResponder,
}

macro_rules! replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

macro_rules! define_operations {
	(
		OPCODE_ENUM: $opcode_enum_name:ident;
		$($tt:tt)*
	) => {
		const fn __counter<const N: usize>(_: [(); N]) -> usize {
    		N
		}

		$crate::communication::operation::parse_operations!(@ON_STRUCT
			OPERATIONS_ENUM: [
				pub enum $opcode_enum_name {}
			]

			$($tt)*
		);
	};
}

macro_rules! parse_operations {
	// Base case, done parsing
	(@ON_STRUCT OPERATIONS_ENUM: [
		pub enum $opcode_enum_name:ident {
			$(
				$(#[$attr:meta])*
				$variant:ident = $code:literal
			),* $(,)?
		}
	]) => {
		/// All operation codes
		#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, deku::DekuRead, deku::DekuWrite)]
		#[repr(u16)]
		#[deku(
			id_type = "u16",
			id_endian = "little",
			ctx = "_endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Little"
		)]
		#[allow(missing_docs)]
		pub enum $opcode_enum_name {
			$(
				$(#[$attr])*
				$variant = $code,
			)*
		}

		impl TryFrom<u16> for $opcode_enum_name {
			type Error = ();

			fn try_from(value: u16) -> core::result::Result<Self, Self::Error> {
				match value {
					$(
						$code => Ok($opcode_enum_name::$variant),
					)*
					_ => Err(())
				}
			}
		}
	};

	// Normal, non-generic structs
	(@ON_STRUCT
		OPERATIONS_ENUM: [$($operations_enum:tt)*]

		$(#[$meta:meta])*
		$([[session_id($session_id:ident)]])?
		pub struct $name:ident {
			code: $code:literal,
			visible_parameters: (
				$(
					$(@DEFAULT($default:expr))?
					$(@RAW($bool:literal))?
					$param:ident: $ty:ty
				),* $(,)?
			),
			$(
				operation_parameters: (
					$(
						$operation_param_expr:expr
					),* $(,)?
				),
			)?
			data_direction: $data_direction:expr,
			response: $response:ty,
			valid_error_codes: [$($error:ident),* $(,)?] $(,)?
		}

		$($rest:tt)*
	) => {
		$(#[$meta])*
		pub struct $name {
			parameters: [$crate::communication::Parameter; __counter([$($crate::communication::operation::replace_expr!($param ())),*])],
			session_id: Option<$crate::communication::SessionId>,
			transaction_id: $crate::communication::TransactionId,
		}

		impl $name {
			$crate::communication::operation::parse_operations!(
				@CONSTRUCTOR $name, $code, $(SESSION_ID: $session_id,)?

				visible_parameters: ($($(@DEFAULT($default))? $(@RAW($bool))? $param: $ty),*),
				operation_parameters: ($($($operation_param_expr),*)?),
				[$($error),*]
			);
		}

		$crate::communication::operation::parse_operations!(@COMMON_OPERATIONS
		OPERATIONS_ENUM: [$($operations_enum)*]

		$name,
		GENERIC: [],
		NAME_WITH_GENERIC: [$name],
		WHERE_CLAUSE: [],

		$code, $data_direction, $response, visible_parameters: ($($param),*), valid_error_codes: [$($error),*] $($rest)*
		);
	};

	// Generic structs
	(@ON_STRUCT
		OPERATIONS_ENUM: [$($operations_enum:tt)*]

		$(#[$meta:meta])*
		$([[session_id($session_id:ident)]])?
		pub partial struct $name:ident<T>
			where T: [$($where_clause:tt)*]
		{
			code: $code:literal,
			visible_parameters: (
				$(
					$(@DEFAULT($default:expr))?
					$(@RAW($bool:literal))?
					$param:ident: $ty:ty
				),* $(,)?
			),
			$(
				operation_parameters: (
					$(
						$operation_param_expr:expr
					),* $(,)?
				),
			)?
			data_direction: $data_direction:expr,
			response: $response:ty,
			valid_error_codes: [$($error:ident),* $(,)?] $(,)?
		}

		$($rest:tt)*
	) => {
		$(#[$meta])*
		pub struct $name<T>
			where T: $($where_clause)*
		{
			parameters: [$crate::communication::Parameter; __counter([(), $($crate::communication::operation::replace_expr!($param ())),*])],
			session_id: Option<$crate::communication::SessionId>,
			transaction_id: $crate::communication::TransactionId,
			_phantom: core::marker::PhantomData<T>,
		}

		impl<T> $name<T>
			where T: $($where_clause)*
		{
			$crate::communication::operation::parse_operations!(
				@CONSTRUCTOR $name, $code, $(SESSION_ID: $session_id,)?

				visible_parameters: ($($(@DEFAULT($default))? $(@RAW($bool))? $param: $ty),*),
				operation_parameters: ($($($operation_param_expr),*)?),
				[$($error),*]
			);
		}

		$crate::communication::operation::parse_operations!(@COMMON_OPERATIONS
		OPERATIONS_ENUM: [$($operations_enum)*]

		$name,
		GENERIC: [T],
		NAME_WITH_GENERIC: [$name<T>],
		WHERE_CLAUSE: [where T: $($where_clause)*],

		$code, $data_direction, $response, visible_parameters: ($($param),*), valid_error_codes: [$($error),*] $($rest)*
		);
	};

	// Stuff to generate for all operations, regardless of partial status
	(
		@COMMON_OPERATIONS
		OPERATIONS_ENUM: [
			pub enum $opcode_enum_name:ident {
				$($variants:tt)*
			}
		]
		$name:ident,
		GENERIC: [$($generic:tt)*],
		NAME_WITH_GENERIC: [$($name_with_generic:tt)*],
		WHERE_CLAUSE: [$($where_clause:tt)*],
		$code:literal,
		$data_direction:expr,
		$response:ty,
		visible_parameters: ($($param:ident),*),
		valid_error_codes: [$($error:ident),*]
		$($rest:tt)*
	) => {
		impl<$($generic)*> $($name_with_generic)* $($where_clause)* {
			const OPCODE: u16 = $code;
		}

		impl<'a, $($generic)*> From<&'a $($name_with_generic)*> for $crate::communication::operation::SerializedOperation<'a> $($where_clause)* {
			fn from(value: &'a $($name_with_generic)*) -> $crate::communication::operation::SerializedOperation<'a> {
				Self {
					code: <$($name_with_generic)*>::OPCODE,
					session_id: value.session_id.unwrap_or($crate::communication::SessionId::NONE),
					transaction_id: value.transaction_id,
					parameters: value.parameters.as_slice(),
				}
			}
		}

		const _: () = {
			const MAX_PARAMETERS: usize = 5;

			if __counter([$($crate::communication::operation::replace_expr!($param ())),*]) > MAX_PARAMETERS {
				panic!("Too many parameters");
			}
		};

		paste::paste! {
			impl<$($generic)*> $crate::communication::operation::DynOperation for $($name_with_generic)* $($where_clause)* {
				const DATA_DIRECTION: Option<DataDirection> = $data_direction;

				type Response = $response;
			}
		}

		$crate::communication::operation::parse_operations!(
			@ON_STRUCT
			OPERATIONS_ENUM: [
				pub enum $opcode_enum_name {
					$($variants)*
					#[deku(id = $code)]
					$name = $code,
				}
			]
			$($rest)*
		);
	};

	// Constructor with no implicit session ID, and no hidden parameters
	(
		@CONSTRUCTOR
		$name:ident,
		$code:literal,
		SESSION_ID: false,
		visible_parameters: ($(
			$(@DEFAULT($default:expr))?
			$(@RAW($bool:literal))?
			$param:ident: $ty:ty),* $(,)?
		),
		operation_parameters: (),
		[$($error:expr),* $(,)?]
	) => {
		#[doc = "Create a new `"]
		#[doc = stringify!($name)]
		#[doc = "` operation"]
		pub fn new(transaction_id: $crate::communication::TransactionId, $($param: $ty),*) -> Self {
			Self {
				parameters: [$($crate::communication::operation::parse_operations!(@PARAM_CONVERT $(@RAW($bool))? $(@DEFAULT($default))? $param: $ty)),*],
				session_id: None,
				transaction_id,
			}
		}
	};

	// Constructor with implicit session ID and no hidden parameters
	(
		@CONSTRUCTOR
		$name:ident,
		$code:literal,
		visible_parameters:  ($(
			$(@DEFAULT($default:expr))?
			$(@RAW($bool:literal))?
			$param:ident: $ty:ty
		),* $(,)?),
		operation_parameters: (),
		[$($error:expr),* $(,)?]
	) => {
		#[doc = "Create a new `"]
		#[doc = stringify!($name)]
		#[doc = "` operation"]
		pub fn new(
			transaction_id: $crate::communication::TransactionId,
			session_id: $crate::communication::SessionId,
			$($param: $ty),*
		) -> Self {
			Self {
				parameters: [$($crate::communication::operation::parse_operations!(@PARAM_CONVERT $(@DEFAULT($default))? $(@RAW($bool))? $param: $ty)),*],
				session_id: Some(session_id),
				transaction_id,
			}
		}
	};

	// Constructor with implicit session ID and hidden parameters
	(
		@CONSTRUCTOR
		$name:ident,
		$code:literal,
		visible_parameters:  ($(
			$(@DEFAULT($_default:expr))?
			$(@RAW($_bool:literal))?
			$param:ident: $ty:ty
		),* $(,)?),
		operation_parameters: (
			$(
				$operation_param_expr:expr
			),*
		),
		[$($error:expr),* $(,)?]
	) => {
		#[doc = "Create a new `"]
		#[doc = stringify!($name)]
		#[doc = "` operation"]
		pub fn new(transaction_id: $crate::communication::TransactionId, session_id: SessionId, $($param: $ty),*) -> Self {
			Self {
				parameters: [$($crate::communication::ParameterPriv::new($operation_param_expr).0),*],
				session_id: Some(session_id),
				transaction_id,
				_phantom: core::marker::PhantomData
			}
		}
	};

	(
		@PARAM_CONVERT
		@DEFAULT($default:expr)
		$param:ident: $_ty:ty
	) => {
		$crate::communication::ParameterPriv::new($param.unwrap_or($default)).0
	};

	(
		@PARAM_CONVERT
		@RAW($_bool:literal)
		$param:ident: $_ty:ty
	) => {
		$crate::communication::ParameterPriv::new_raw($param).0
	};

	(
		@PARAM_CONVERT
		$param:ident: $_ty:ty
	) => {
		$crate::communication::ParameterPriv::new($param).0
	};
}

pub(super) use {define_operations, parse_operations, replace_expr};

define_operations! {
    OPCODE_ENUM: BaseOperation;

    /// Get the [`DeviceInfo`] of the device.
    ///
    /// This operation is commonly the first operation called by an initiator upon
    /// connecting to a responder for the first time.
    ///
    /// [`DeviceInfo`]: crate::device::info::DeviceInfo
    pub struct GetDeviceInfo {
        code: 0x1001,
        visible_parameters: (),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetDeviceInfo,
        valid_error_codes: [ParameterNotSupported]
    }

    /// Create a new session for between the initiator and responder.
    ///
    /// Unless specified otherwise, all operations must be performed within the context of a session.
    ///
    /// In the event that an active session already exists, a response of [`SessionAlreadyOpen`]
    /// will be returned.
    ///
    /// ## Parameters
    ///
    /// * `session_id` - An ID to assign to the session.
    ///
    /// [`SessionAlreadyOpen`]: crate::communication::response::errors::SessionAlreadyOpen
    [[session_id(false)]]
    pub struct OpenSession {
        code: 0x1002,
        visible_parameters: (session_id: SessionId),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            ParameterNotSupported,
            InvalidParameter,
            SessionAlreadyOpen,
            DeviceBusy,
        ]
    }

    /// Close the current session.
    ///
    /// All stateful information associated with the session will be discarded.
    ///
    /// If no session is currently open, a response of [`SessionNotOpen`] will be returned.
    ///
    /// [`SessionNotOpen`]: crate::communication::response::errors::SessionNotOpen
    pub struct CloseSession {
        code: 0x1003,
        visible_parameters: (),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            SessionNotOpen,
            InvalidTransactionId,
            ParameterNotSupported,
        ]
    }

    /// Get the storage IDs of all storages on the device.
    pub struct GetStorageIDs {
        code: 0x1004,
        visible_parameters: (),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetStorageIDs,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            ParameterNotSupported,
        ]
    }

    /// Get the [`StorageInfo`] of the given [`StorageId`].
    ///
    /// ## Parameters
    ///
    /// * `storage` - The storage to query.
    ///
    /// [`StorageInfo`]: crate::device::storage::info::StorageInfo
    /// [`StorageId`]: crate::device::storage::id::StorageId
    pub struct GetStorageInfo {
        code: 0x1005,
        visible_parameters: (storage: StorageId),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetStorageInfo,
        valid_error_codes: [
            OperationNotSupported,
            InvalidTransactionId,
            AccessDenied,
            InvalidStorageId,
            StoreNotAvailable,
            ParameterNotSupported,
        ]
    }

    /// Get the number of objects on the device
    ///
    /// ## Parameters
    ///
    /// * `storage` - The storage to query, or [`StorageId::ALL_STORAGES`] for an aggregated total across all storages.
    /// * `format` - Only count objects of a certain format, or omit to include objects of all types.
    /// * `parent` - Only count objects directly contained in the parent object.
    pub struct GetNumObjects {
        code: 0x1006,
        visible_parameters: (
            storage: StorageId,
            @DEFAULT(ObjectFormatCode::from(0))
            format: Option<ObjectFormatCode>,
            @DEFAULT(ObjectHandle::from(0))
            parent: Option<ObjectHandle>
        ),
        data_direction: None,
        response: response::GetNumObjects,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            InvalidStorageId,
            StoreNotAvailable,
            SpecificationByFormatUnsupported,
            InvalidCodeFormat,
            ParameterNotSupported,
            InvalidParentObject,
            InvalidObjectHandle,
            InvalidParameter,
        ]
    }

    /// Get the [`ObjectHandle`]s of the contents on the device
    ///
    /// ## Parameters
    ///
    /// * `storage` - The storage to query, or [`StorageId::ALL_STORAGES`] for an aggregated list across all storages.
    /// * `format` - Only include objects of a certain format, or omit to include objects of all types.
    /// * `parent` - Only include objects directly contained in the parent object.
    pub struct GetObjectHandles {
        code: 0x1007,
        visible_parameters: (
            storage: StorageId,
            @DEFAULT(ObjectFormatCode::from(0))
            format: Option<ObjectFormatCode>,
            @DEFAULT(ObjectHandle::from(0))
            parent: Option<ObjectHandle>
        ),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetObjectHandles,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            InvalidStorageId,
            StoreNotAvailable,
            SpecificationByFormatUnsupported,
            InvalidCodeFormat,
            InvalidObjectHandle,
            InvalidParameter,
            ParameterNotSupported,
            InvalidParentObject,
        ]
    }

    /// Get the [`ObjectInfo`] of the given [`ObjectHandle`].
    ///
    /// ## Parameters
    ///
    /// * `object` - The object to query.
    ///
    /// [`ObjectInfo`]: crate::object::info::ObjectInfo
    pub struct GetObjectInfo {
        code: 0x1008,
        visible_parameters: (object: ObjectHandle),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetObjectInfo,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            InvalidObjectHandle,
            StoreNotAvailable,
            ParameterNotSupported,
        ]
    }

    /// Get the binary contents of the given [`ObjectHandle`].
    ///
    /// ## Parameters
    ///
    /// * `object` - The object to query.
    pub struct GetObject {
        code: 0x1009,
        visible_parameters: (object: ObjectHandle),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetObject,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            InvalidObjectHandle,
            InvalidParameter,
            StoreNotAvailable,
            IncompleteTransfer,
            AccessDenied,
            ParameterNotSupported,
        ]
    }

    /// Get the thumbnail of an image object at the given [`ObjectHandle`].
    ///
    /// ## Parameters
    ///
    /// * `object` - The object to query.
    pub struct GetThumb {
        code: 0x100a,
        visible_parameters: (object: ObjectHandle),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetThumb,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            InvalidObjectHandle,
            NoThumbnailPresent,
            InvalidObjectFormatCode,
            StoreNotAvailable,
            ParameterNotSupported,
        ]
    }

    /// Delete an object on the responder
    ///
    /// ## Parameters
    ///
    /// * `object` - The object to delete, or [`ObjectHandle::ALL`] to delete all objects on the responder.
    /// * `format` - If `object` is [`ObjectHandle::ALL`], then this can be used to restrict the types of objects deleted.
    pub struct DeleteObject {
        code: 0x100b,
        visible_parameters: (
            object: ObjectHandle,
            @DEFAULT(ObjectFormatCode::Unknown(0))
            format: Option<ObjectFormatCode>
        ),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            InvalidObjectHandle,
            ObjectWriteProtected,
            StoreReadOnly,
            PartialDeletion,
            StoreNotAvailable,
            SpecificationByFormatUnsupported,
            InvalidCodeFormat,
            DeviceBusy,
            ParameterNotSupported,
            AccessDenied,
        ]
    }

    /// Allocate an object on the responder
    ///
    /// NOTE: This ***should***, on success, be followed by a [`SendObject`] operation.
    ///
    /// ## Parameters
    ///
    /// * `destination` - The storage to store the new object, or omit to leave it up to the responder.
    /// * `parent` - The parent in which the new object should be placed, or omit to leave it up to the responder.
    pub struct SendObjectInfo {
        code: 0x100c,
        visible_parameters: (
            @DEFAULT(StorageId::from(0))
            destination: Option<StorageId>,
            @DEFAULT(ObjectHandle::from(0))
            parent: Option<ObjectHandle>
        ),
        data_direction: Some(DataDirection::InitiatorToResponder),
        response: response::SendObjectInfo,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            AccessDenied,
            InvalidStorageId,
            StoreReadOnly,
            ObjectTooLarge,
            StoreFull,
            InvalidObjectFormatCode,
            StoreNotAvailable,
            ParameterNotSupported,
            InvalidParentObject,
            InvalidDataset,
            InvalidParameter,
            InvalidObjectHandle,
            SpecificationOfDestinationUnsupported,
        ]
    }

    /// Send the binary data of an object after a successful [`SendObjectInfo`]
    ///
    /// NOTE: This ***must*** be preceded by a successful [`SendObjectInfo`] operation.
    pub struct SendObject {
        code: 0x100d,
        visible_parameters: (),
        data_direction: Some(DataDirection::InitiatorToResponder),
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            AccessDenied,
            InvalidStorageId,
            StoreReadOnly,
            ObjectTooLarge,
            StoreFull,
            InvalidObjectFormatCode,
            StoreNotAvailable,
            ParameterNotSupported,
            InvalidParentObject,
        ]
    }

    /// Produce a new data object using an object capture mechanism
    ///
    /// ## Parameters
    ///
    /// * `storage` - The storage to store the captured object, or omit to leave it up to the responder.
    /// * `format` - The desired capture format, or omit to leave it up to the responder.
    pub struct InitiateCapture {
        code: 0x100e,
        visible_parameters: (
            @DEFAULT(StorageId::from(0))
            storage: Option<StorageId>,
            @DEFAULT(ObjectFormatCode::Unknown(0))
            format: Option<ObjectFormatCode>
        ),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            InvalidStorageId,
            StoreFull,
            InvalidObjectFormatCode,
            InvalidParameter,
            StoreNotAvailable,
            InvalidCodeFormat,
            DeviceBusy,
            ParameterNotSupported,
            StoreReadOnly,
        ]
    }

    /// Format the media indicated by the given [`StorageId`]
    ///
    /// ## Parameters
    ///
    /// * `storage` - The storage to format.
    /// * `fs` - The desired filesystem format, or omit to leave it up to the responder.
    pub struct FormatStore {
        code: 0x100f,
        visible_parameters: (
            storage: StorageId,
            @DEFAULT(FilesystemType::Undefined)
            fs: Option<FilesystemType>
        ),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            InvalidStorageId,
            StoreNotAvailable,
            DeviceBusy,
            ParameterNotSupported,
            InvalidParameter,
            StoreReadOnly,
        ]
    }

    /// Return the device to a default state
    pub struct ResetDevice {
        code: 0x1010,
        visible_parameters: (),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            DeviceBusy,
        ]
    }

    /// Perform some device-specific test
    ///
    /// ## Parameters
    ///
    /// * `test_type` - The type of self-test to perform.
    pub struct SelfTest {
        code: 0x1011,
        visible_parameters: (test_type: SelfTestType),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            DeviceBusy,
        ]
    }

    /// Set the write-protection status of an object
    ///
    /// ## Parameters
    ///
    /// * `object` - The object to update.
    /// * `status` - The new write-protection status of the `object`.
    pub struct SetObjectProtection {
        code: 0x1012,
        visible_parameters: (object: ObjectHandle, status: ProtectionStatus),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            AccessDenied,
            InvalidObjectHandle,
            InvalidParameter,
            StoreNotAvailable,
            ParameterNotSupported,
            StoreReadOnly,
        ]
    }

    /// Instruct the device to close all active sessions and power down
    pub struct PowerDown {
        code: 0x1013,
        visible_parameters: (),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            DeviceBusy,
            ParameterNotSupported,
        ]
    }

    /// Get the property descriptor for the given property code
    pub partial struct GetDevicePropDesc<T>
        where T: [DeviceProperty]
    {
        code: 0x1014,
        visible_parameters: (),
        operation_parameters: (Parameter::new(T::CODE as u32)),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetDevicePropDesc,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            AccessDenied,
            DevicePropNotSupported,
            DeviceBusy,
            ParameterNotSupported,
        ]
    }

    /// Get the value of the given property code
    ///
    /// NOTE: This is the same as the `current_value` field in [`DevicePropDesc`], provided by the
    ///       [`GetDevicePropDesc`] operation.
    ///
    /// [`DevicePropDesc`]: crate::device::properties::DevicePropDesc
    pub partial struct GetDevicePropValue<T>
        where T: [DeviceProperty]
    {
        code: 0x1015,
        visible_parameters: (),
        operation_parameters: (Parameter::new(T::CODE as u32)),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetDevicePropValue<T>,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            AccessDenied,
            DevicePropNotSupported,
            DeviceBusy,
            ParameterNotSupported,
        ]
    }

    /// Set the value of the given property code
    pub partial struct SetDevicePropValue<T>
        where T: [DeviceProperty]
    {
        code: 0x1016,
        visible_parameters: (),
        operation_parameters: (Parameter::new(T::CODE as u32)),
        data_direction: Some(DataDirection::InitiatorToResponder),
        response: response::Empty,
        valid_error_codes: [
            SessionNotOpen,
            InvalidTransactionId,
            AccessDenied,
            DevicePropNotSupported,
            ObjectPropNotSupported,
            InvalidDevicePropFormat,
            InvalidDevicePropValue,
            DeviceBusy,
            ParameterNotSupported,
        ]
    }

    /// Factory reset the device property value
    pub partial struct ResetDevicePropValue<T>
        where T: [DeviceProperty]
    {
        code: 0x1017,
        visible_parameters: (),
        operation_parameters: (Parameter::new(T::CODE as u32)),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            DevicePropNotSupported,
            DeviceBusy,
            ParameterNotSupported,
            AccessDenied,
        ]
    }

    /// End an [`InitiateOpenCapture`] operation
    ///
    /// ## Parameters
    ///
    /// * `transaction` - The transaction ID obtained from the corresponding [`InitiateOpenCapture`] operation.
    pub struct TerminateOpenCapture {
        code: 0x1018,
        visible_parameters: (transaction: TransactionId),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            ParameterNotSupported,
            InvalidParameter,
            CaptureAlreadyTerminated,
        ]
    }

    /// Change the location of an object
    ///
    /// ## Parameters
    ///
    /// * `object` - The object to move.
    /// * `storage` - The storage to move the object to.
    /// * `parent` - The parent object to move `object` into, or omit to place the `object` in the root of the `storage`.
    pub struct MoveObject {
        code: 0x1019,
        visible_parameters: (
            object: ObjectHandle,
            storage: StorageId,
            @DEFAULT(ObjectHandle::NONE)
            parent: Option<ObjectHandle>
        ),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            StoreReadOnly,
            StoreNotAvailable,
            InvalidObjectHandle,
            InvalidParentObject,
            DeviceBusy,
            ParameterNotSupported,
            InvalidStorageId,
            StoreFull,
            PartialDeletion,
        ]
    }

    /// Create a copy of an object and place it in a new location
    ///
    /// ## Parameters
    ///
    /// * `object` - The object to copy.
    /// * `storage` - The storage to copy the object to.
    /// * `parent` - The parent object the place the copy into, or omit to place the copy in the root of the `storage`.
    pub struct CopyObject {
        code: 0x101A,
        visible_parameters: (
            object: ObjectHandle,
            storage: StorageId,
            @DEFAULT(ObjectHandle::NONE)
            parent: Option<ObjectHandle>
        ),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            StoreReadOnly,
            InvalidObjectHandle,
            InvalidParentObject,
            DeviceBusy,
            StoreFull,
            ParameterNotSupported,
            InvalidStorageId,
        ]
    }

    /// Get a partial object from the device, may be used in place of [`GetObject`]
    ///
    /// ## Parameters
    ///
    /// * `object` - The object to query.
    /// * `offset` - The offset at which to start the selection.
    /// * `len` - The length of the selection.
    /// 	* If the entire object is desired, `len` can be set to [`u32::MAX`].
    pub struct GetPartialObject {
        code: 0x101B,
        visible_parameters: (object: ObjectHandle, @RAW(true) offset: u32, @RAW(true) len: u32),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetPartialObject,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            InvalidObjectHandle,
            InvalidObjectFormatCode,
            InvalidParameter,
            StoreNotAvailable,
            DeviceBusy,
            ParameterNotSupported,
        ]
    }

    /// Initiate the capture of multiple new objects
    ///
    /// NOTES:
    ///
    /// * This is an asynchronous operation, and must be terminated with a [`TerminateOpenCapture`] operation.
    /// * If the capture is ended manually with a [`TerminateOpenCapture`] operation, the responder will **not**
    ///   produce an [`Event::CaptureComplete`].
    ///
    /// ## Parameters
    ///
    /// * `storage` - The storage to store the captured objects, or omit to leave it up to the responder.
    /// * `format` - The desired capture format, or omit to leave it up to the responder.
    ///
    /// [`Event::CaptureComplete`]: crate::communication::event::Event::CaptureComplete
    pub struct InitiateOpenCapture {
        code: 0x101C,
        visible_parameters: (
            @DEFAULT(StorageId::from(0))
            storage: Option<StorageId>,
            @DEFAULT(ObjectFormatCode::Unknown(0))
            format: Option<ObjectFormatCode>
        ),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            InvalidStorageId,
            StoreFull,
            InvalidObjectFormatCode,
            InvalidParameter,
            StoreNotAvailable,
            InvalidCodeFormat,
            DeviceBusy,
            ParameterNotSupported,
            StoreReadOnly,
        ]
    }

    /// Get all supported [`ObjectPropertyCode`]s for the given format
    ///
    /// ## Parameters
    ///
    /// * `format` - The object format to query.
    pub struct GetObjectPropsSupported {
        code: 0x9801,
        visible_parameters: (format: ObjectFormatCode),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetObjectPropsSupported,
        valid_error_codes: [
            OperationNotSupported,
            DeviceBusy,
            InvalidTransactionId,
            InvalidObjectFormatCode,
        ]
    }

    /// Get the property description for the given object property code
    ///
    /// The parameter `T` specifies the object property to be returned.
    /// See [`crate::object::types::properties`] for a list of properties.
    ///
    /// ## Parameters
    ///
    /// * `format` - The object format to query.
    pub partial struct GetObjectPropDesc<T>
        where T: [ObjectProperty]
    {
        code: 0x9802,
        visible_parameters: (format: ObjectFormatCode),
        operation_parameters: (Parameter::new(T::CODE as u32), format),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetObjectPropDesc<T>,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            AccessDenied,
            InvalidObjectPropCode,
            InvalidObjectFormatCode,
            ObjectPropNotSupported,
            DeviceBusy,
        ]
    }

    /// Get the current value for the given object property code
    ///
    /// The parameter `T` specifies the object property to query.
    /// See [`crate::object::types::properties`] for a list of properties.
    ///
    /// ## Parameters
    ///
    /// * `object` - The object to query.
    pub partial struct GetObjectPropValue<T>
        where T: [ObjectProperty]
    {
        code: 0x9803,
        visible_parameters: (object: ObjectHandle),
        operation_parameters: (object, Parameter::new(T::CODE as u32)),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetObjectPropValue<T>,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            InvalidObjectPropCode,
            ObjectPropNotSupported,
            DeviceBusy,
            InvalidObjectHandle,
        ]
    }

    /// Set the value for the given object property code
    ///
    /// The parameter `T` specifies the object property to set.
    /// See [`crate::object::types::properties`] for a list of properties.
    ///
    /// ## Parameters
    ///
    /// * `object` - The object to update.
    pub partial struct SetObjectPropValue<T>
        where T: [ObjectProperty]
    {
        code: 0x9804,
        visible_parameters: (object: ObjectHandle),
        operation_parameters: (object, Parameter::new(T::CODE as u32)),
        data_direction: Some(DataDirection::InitiatorToResponder),
        response: response::Empty,
        valid_error_codes: [
            SessionNotOpen,
            InvalidTransactionId,
            AccessDenied,
            InvalidObjectPropCode,
            InvalidObjectHandle,
            ObjectPropNotSupported,
            DeviceBusy,
            InvalidObjectPropFormat,
            InvalidObjectPropValue,
        ]
    }

    /// Get an array of all active [`ObjectHandle`]s
    ///
    /// ## Parameters
    ///
    /// * `object` - The object to query.
    pub struct GetObjectReferences {
        code: 0x9810,
        visible_parameters: (object: ObjectHandle),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetObjectReferences,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            InvalidObjectHandle,
            StoreNotAvailable,
        ]
    }

    /// Replace the object references on an object
    ///
    /// ## Parameters
    ///
    /// * `object` - The target object.
    pub struct SetObjectReferences {
        code: 0x9811,
        visible_parameters: (object: ObjectHandle),
        data_direction: Some(DataDirection::InitiatorToResponder),
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            AccessDenied,
            InvalidStorageId,
            StoreReadOnly,
            StoreFull,
            StoreNotAvailable,
            InvalidObjectHandle,
            InvalidObjectReference,
        ]
    }

    /// Update the playback of the current object
    ///
    /// ## Parameters
    ///
    /// * `skip` - The depth and direction into the playback queue.
    /// 	* For example, a value of `1` indicates the device should skip ahead one media object,
    ///       and a value of `-1` indicates the device should skip back one media object.
    pub struct Skip {
        code: 0x9820,
        visible_parameters: (@RAW(true) skip: u32),
        data_direction: None,
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            AccessDenied,
            InvalidStorageId,
            StoreReadOnly,
            StoreFull,
            StoreNotAvailable,
            InvalidObjectHandle,
            InvalidObjectReference,
        ]
    }

    // == Enhanced Operations ==
    //
    // Defined in Appendix E

    /// Get a list containing all specified object properties
    ///
    /// This is a more optimized way of accessing object properties without needing to individually
    /// query each {object, property} pair.
    ///
    /// ## Parameters
    ///
    /// * `object` - The object to root the query.
    /// 	* This can also be [`ObjectHandle::ALL`] to query all objects, or [`ObjectHandle::NONE`] to query
    ///       all root-level objects.
    /// * `format` - Only query objects of the given format, or omit to query objects of all formats.
    /// * `prop` - The property being requested.
    /// 	* This can also be [`ObjectPropertyCode::All`] to query all properties, or omitted to instead
    ///       query by the `group`.
    /// * `group` - The retrieval group code to query.
    /// 	* This is only used if `prop` is omitted.
    /// * `depth` - Restrict the query to a certain depth in the folder hierarchy, down from the root `object`.
    /// 	* A value of `0` will query only the objects at the top (root) level, including the root `object`.
    /// 	* A value of [`u16::MAX`] will query all objects in the hierarchy, rooted at `object`.
    pub struct GetObjectPropList {
        code: 0x9805,
        visible_parameters: (
            object: ObjectHandle,
            @DEFAULT(ObjectFormatCode::from(0))
            format: Option<ObjectFormatCode>,
            @DEFAULT(ObjectPropertyCode::All)
            prop: Option<ObjectPropertyCode>,
            @RAW(true) group: u32,
            @RAW(true) depth: u32,
        ),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetObjectPropList,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            ObjectPropNotSupported,
            InvalidObjectHandle,
            GroupNotSupported,
            DeviceBusy,
            ParameterNotSupported,
            SpecificationByFormatUnsupported,
            SpecificationByGroupUnsupported,
            SpecificationByDepthUnsupported,
            InvalidCodeFormat,
            InvalidObjectPropCode,
            InvalidStorageId,
            StoreNotAvailable,
        ]
    }

    /// Update the object properties contained in the given [`ObjectPropList`]
    ///
    /// [`ObjectPropList`]: crate::object::types::properties::ObjectPropList
    pub struct SetObjectPropList {
        code: 0x9806,
        visible_parameters: (),
        data_direction: Some(DataDirection::InitiatorToResponder),
        response: response::Empty,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            AccessDenied,
            ObjectPropNotSupported,
            InvalidObjectPropFormat,
            InvalidObjectPropValue,
            InvalidObjectHandle,
            DeviceBusy,
            StoreNotAvailable,
            StoreFull,
        ]
    }

    pub struct GetInterdependentPropDesc {
        code: 0x9807,
        visible_parameters: (format: ObjectFormatCode),
        data_direction: Some(DataDirection::ResponderToInitiator),
        response: response::GetInterdependentPropDesc,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            DeviceBusy,
            InvalidCodeFormat,
        ]
    }

    /// Send a modified object property list to the responder
    ///
    /// This is the same as [`SendObjectInfo`], but allows setting object properties at creation time.
    ///
    /// This is to be used before a [`SendObject`] operation, to inform the responder of the properties
    /// of the objects to come.
    ///
    /// An `OK` response indicates that the responder can handle the intended object, and is ready
    /// for a [`SendObject`] operation.
    ///
    /// NOTES:
    ///
    /// * If `destination` is unspecified, the responder will determine the store to place it in.
    /// * If `parent` is specified, `destination` **must** also be specified. If it is unspecified,
    ///   the responder will determine the store to place it in.
    ///
    /// ## Parameters
    ///
    /// * `destination` - The storage to store the new object, or omit to leave it up to the responder.
    /// * `parent` - The parent object to store the new object into, or omit to leave it up to the responder.
    /// 	* `destination` **must** be specified if this is specified.
    /// 	* To guarantee that the object is stored in the root of the storage, [`ObjectHandle::ALL`] can be used.
    /// * `format` - The format of the new object.
    /// * `size` - The 64-bit size of the object's binary data.
    pub struct SendObjectPropList {
        code: 0x9808,
        visible_parameters: (
            @DEFAULT(StorageId::from(0))
            destination: Option<StorageId>,
            @DEFAULT(ObjectHandle::from(0))
            parent: Option<ObjectHandle>,
            format: ObjectFormatCode,
            @RAW(true) size_high: u32,
            @RAW(true) size_low: u32
        ),
        data_direction: Some(DataDirection::InitiatorToResponder),
        response: response::SendObjectPropList,
        valid_error_codes: [
            OperationNotSupported,
            SessionNotOpen,
            InvalidTransactionId,
            AccessDenied,
            ObjectPropNotSupported,
            InvalidObjectPropFormat,
            InvalidObjectPropValue,
            InvalidObjectHandle,
            DeviceBusy,
            StoreNotAvailable,
            StoreFull,
        ]
    }
}

/// |                        Value                        |          Description         |
/// |-----------------------------------------------------|------------------------------|
/// | 0x0000                                              | Default device-specific test |
/// | All other values with Bit 15 set to 0               | Reserved PTP                 |
/// | All values with Bit 15 set to 1 and Bit 14 set to 0 | MTP vendor extension         |
/// | All values with Bit 15 set to 1 and Bit 14 set to 1 | Reserved MTP                 |
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[repr(transparent)]
pub struct SelfTestType(u16);

impl From<u16> for SelfTestType {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<SelfTestType> for Parameter {
    fn from(value: SelfTestType) -> Self {
        Parameter::new(value.0 as u32)
    }
}
