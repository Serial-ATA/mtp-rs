use crate::communication::{Parameter, ParameterPriv, SessionId, TransactionId, response};
use crate::device::properties::{DeviceProperty, DevicePropertyCode};
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

const fn counter<const N: usize>(_: [(); N]) -> usize {
    N
}

macro_rules! replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

macro_rules! define_operations {
	($($tt:tt)*) => {
		parse_operations!(@ON_STRUCT
			OPERATIONS_ENUM: [
				pub enum Operation {}
			]

			$($tt)*
		);
	};
}

macro_rules! parse_operations {
	// Base case, done parsing
	(@ON_STRUCT OPERATIONS_ENUM: [
		pub enum Operation {
			$(
				$(#[$attr:meta])*
				$variant:ident = $code:literal
			),* $(,)?
		}
	]) => {
		/// All operation codes
		#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, DekuRead, DekuWrite)]
		#[repr(u16)]
		#[deku(
			id_type = "u16",
			id_endian = "little",
			ctx = "_endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Little"
		)]
		#[allow(missing_docs)]
		pub enum Operation {
			$(
				$(#[$attr])*
				$variant = $code,
			)*
			#[deku(id_pat = "o if (0x9000_u16..=0x97FF_u16).contains(&o)")]
			VendorSpecific(u16),
		}

		impl TryFrom<u16> for Operation {
			type Error = ();

			fn try_from(value: u16) -> core::result::Result<Self, Self::Error> {
				match value {
					$(
						$code => Ok(Operation::$variant),
					)*
					_ if (0x9000_u16..=0x97FF_u16).contains(&value) => Ok(Operation::VendorSpecific(value)),
					_ => Err(())
				}
			}
		}

		paste::paste! {
			#[derive(Clone, Debug)]
			#[allow(missing_docs)]
			pub enum OperationErrorKind {
				$(
				$variant([<$variant Error>])
				),*
			}

			impl core::fmt::Display for OperationErrorKind {
				fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
					match self {
						$(
							Self::$variant(error) => write!(f, "{error}"),
						)*
					}
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
			parameters: [$crate::communication::Parameter; counter([$(replace_expr!($param ())),*])],
			session_id: Option<$crate::communication::SessionId>,
			transaction_id: $crate::communication::TransactionId,
		}

		impl $name {
			parse_operations!(
				@CONSTRUCTOR $name, $code, $(SESSION_ID: $session_id,)?

				visible_parameters: ($($(@DEFAULT($default))? $(@RAW($bool))? $param: $ty),*),
				operation_parameters: ($($($operation_param_expr),*)?),
				[$($error),*]
			);
		}

		parse_operations!(@COMMON_OPERATIONS
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
			parameters: [$crate::communication::Parameter; counter([(), $(replace_expr!($param ())),*])],
			session_id: Option<$crate::communication::SessionId>,
			transaction_id: $crate::communication::TransactionId,
			_phantom: core::marker::PhantomData<T>,
		}

		impl<T> $name<T>
			where T: $($where_clause)*
		{
			parse_operations!(
				@CONSTRUCTOR $name, $code, $(SESSION_ID: $session_id,)?

				visible_parameters: ($($(@DEFAULT($default))? $(@RAW($bool))? $param: $ty),*),
				operation_parameters: ($($($operation_param_expr),*)?),
				[$($error),*]
			);
		}

		parse_operations!(@COMMON_OPERATIONS
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
			pub enum Operation {
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
					session_id: value.session_id.unwrap_or(SessionId::NONE),
					transaction_id: value.transaction_id,
					parameters: value.parameters.as_slice(),
				}
			}
		}

		const _: () = {
			const MAX_PARAMETERS: usize = 5;

			if counter([$(replace_expr!($param ())),*]) > MAX_PARAMETERS {
				panic!("Too many parameters");
			}
		};

		paste::paste! {
			impl<$($generic)*> $crate::communication::operation::DynOperation for $($name_with_generic)* $($where_clause)* {
				const DATA_DIRECTION: Option<DataDirection> = $data_direction;

				type Response = $response;
				type Error = [<$name Error>];
			}
		}

		paste::paste! {
			// TODO: Need a generic variant. Some operations may return 0x2002 (General error) for example
			#[doc = "Errors that can occur when executing the [`" $name "`] operation"]
			#[derive(Clone, Debug, deku::DekuRead)]
			#[deku(
				ctx = "endian: deku::ctx::Endian, error_code: u16",
				id = "error_code",
				id_endian = "endian",
			)]
			#[allow(missing_docs)]
			pub enum [<$name Error>] {
				$(
				// TODO: Ask if this should be supported
				#[deku(id = crate::communication::response::errors::$error::CODE)]
				$error($crate::communication::response::errors::$error)
				),*
			}

			impl core::fmt::Display for [<$name Error>] {
				fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
					match self {
						$(
							Self::$error(error) => write!(f, "{error}"),
						)*
					}
				}
			}

			impl core::error::Error for [<$name Error>] {}

			impl From<[<$name Error>]> for OperationErrorKind {
				fn from(value: [<$name Error>]) -> Self {
					OperationErrorKind::$name(value)
				}
			}
		}

		parse_operations!(
			@ON_STRUCT
			OPERATIONS_ENUM: [
				pub enum Operation {
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
		pub fn new(transaction_id: $crate::communication::TransactionId, $($param: $ty),*) -> Self {
			Self {
				parameters: [$(parse_operations!(@PARAM_CONVERT $(@RAW($bool))? $(@DEFAULT($default))? $param: $ty)),*],
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
		pub fn new(transaction_id: $crate::communication::TransactionId, session_id: SessionId, $($param: $ty),*) -> Self {
			Self {
				parameters: [$(parse_operations!(@PARAM_CONVERT $(@DEFAULT($default))? $(@RAW($bool))? $param: $ty)),*],
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
		pub fn new(transaction_id: $crate::communication::TransactionId, session_id: SessionId, $($param: $ty),*) -> Self {
			Self {
				parameters: [$(ParameterPriv::new($operation_param_expr).0),*],
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
		ParameterPriv::new($param.unwrap_or($default)).0
	};

	(
		@PARAM_CONVERT
		@RAW($_bool:literal)
		$param:ident: $_ty:ty
	) => {
		ParameterPriv::new_raw($param).0
	};

	(
		@PARAM_CONVERT
		$param:ident: $_ty:ty
	) => {
		ParameterPriv::new($param).0
	};
}

define_operations! {
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

    // TODO: Explain the optional parameters
    /// Get the number of objects on the device
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

    // TODO: Explain the optional parameters
    /// Get the [`ObjectHandle`]s of the contents on the device
    pub struct GetObjectHandles {
        code: 0x1007,
        visible_parameters: (
            storage: StorageId,
            @DEFAULT(ObjectFormatCode::from(0))
            format: Option<ObjectFormatCode>,
            @DEFAULT(ObjectHandle::from(0))
            object: Option<ObjectHandle>
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

    /// Delete the object at the given [`ObjectHandle`].
    pub struct DeleteObject {
        code: 0x100b,
        visible_parameters: (object: ObjectHandle, format: ObjectFormatCode),
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

    // TODO: explain optional parameters
    /// Received from the device, indicating it wishes to send a new object
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

    /// Received from the device after a successful [`SendObjectInfo`], contains the binary data of the new object
    pub struct SendObject {
        code: 0x100d,
        visible_parameters: (),
        data_direction: Some(DataDirection::InitiatorToResponder),
        response: response::SendObject,
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

    // TODO: optional parameters
    /// Produce a new data object using an object capture mechanism
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

    // TODO: Explain optional parameters
    /// Format the media indicated by the given [`StorageId`]
    pub struct FormatStore {
        code: 0x100f,
        visible_parameters: (
            storage: StorageId,
            fs: FilesystemType
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

    /// Return the device to a default state
    ///
    /// See [`SelfTestType`]
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
    pub struct SetDevicePropValue {
        code: 0x1016,
        visible_parameters: (code: DevicePropertyCode),
        data_direction: Some(DataDirection::InitiatorToResponder),
        response: response::SetDevicePropValue,
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
    pub struct ResetDevicePropValue {
        code: 0x1017,
        visible_parameters: (code: DevicePropertyCode),
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
    /// If no `parent` object is specified, the copy will be placed in the root of the `storage`.
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
    /// If no `parent` object is specified, the copy will be placed in the root of the `storage`.
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
    /// If the entire object is desired, `len` can be set to [`u32::MAX`].
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
    /// * If `storage` is not specified, the responder determines the location
    /// * If `format` is not specified, the responder determines the format
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

    /// Get all supported object property codes for the given format
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

    /// Get the value for the given object property code
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

    /// Replace the references on an object
    pub struct SetObjectReferences {
        code: 0x9811,
        visible_parameters: (object: ObjectHandle),
        data_direction: Some(DataDirection::InitiatorToResponder),
        response: response::SetObjectReferences,
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
    /// The `skip` determines the depth and direction into the playback queue. Meaning a value of 1
    /// indicates the device should skip ahead one media object, and a value of -1 indicates the
    /// device should skip back one media object.
    pub struct Skip {
        code: 0x9820,
        visible_parameters: (@RAW(true) skip: u32),
        data_direction: None,
        response: response::SetObjectReferences,
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
    /// NOTES:
    ///
    /// * The `format` can be specified to limit the response to only the properties of objects
    ///   of the given format. If unspecified, the response will contain the properties of all
    ///   formats.
    /// * The `depth` can be specified to limit the query to objects at a certain level of a folder
    ///   hierarchy. If unspecified, the response will contain the properties of objects only at the
    ///   top level.
    pub struct GetObjectPropList {
        code: 0x9805,
        visible_parameters: (
            object: ObjectHandle,
            @DEFAULT(ObjectFormatCode::from(0))
            format: Option<ObjectFormatCode>,
            prop: ObjectPropertyCode,
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

    /// Set object properties container in the given dataset
    pub struct SetObjectPropList {
        code: 0x9806,
        visible_parameters: (),
        data_direction: Some(DataDirection::InitiatorToResponder),
        response: response::SetObjectPropList,
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

    /// Send a modified [`ObjectPropList`] to the responder
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
