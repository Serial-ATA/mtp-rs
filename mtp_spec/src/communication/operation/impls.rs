use crate::communication::{Parameter, ParameterPriv, SessionId, TransactionId, response};
use crate::device::storage::id::StorageId;
use crate::device::storage::info::FilesystemType;
use crate::object::info::ProtectionStatus;
use crate::object::types::properties::{ObjectProperty, ObjectPropertyCode};
use crate::object::types::{ObjectFormatCode, ObjectHandle};

use deku::{DekuRead, DekuWrite};

const fn counter<const N: usize>(_: [(); N]) -> usize {
	N
}

macro_rules! replace_expr {
	($_t:tt $sub:expr) => {
		$sub
	};
}

const MAX_PARAMETERS: usize = 5;

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
			$($variants:tt)*
		}
	]) => {
		#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, DekuRead, DekuWrite)]
		#[repr(u16)]
		#[deku(
			id_type = "u16",
			id_endian = "little",
			ctx = "_endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Little"
		)]
		pub enum Operation {
			$($variants)*
			#[deku(id_pat = "o if (0x9000_u16..=0x97FF_u16).contains(&o)")]
			VenderSpecific(u16),
		}
	};

	// Normal, non-generic structs
	(@ON_STRUCT
		OPERATIONS_ENUM: [$($operations_enum:tt)*]

		$(#[$meta:meta])*
		$([[session_id($session_id:ident)]])?
		pub struct $name:ident {
			code: $code:literal,
			parameters: (
				$(
					$(@DEFAULT($default:expr))?
					$(@RAW($bool:literal))?
					$param:ident: $ty:ty
				),* $(,)?
			),
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
				@CONSTRUCTOR $name, $code, $(SESSION_ID: $session_id,)? ($($(@DEFAULT($default))? $(@RAW($bool))? $param: $ty),*), [$($error),*]
			);
		}

		parse_operations!(@COMMON_OPERATIONS
		OPERATIONS_ENUM: [$($operations_enum)*]

		$name,
		GENERIC: [],
		NAME_WITH_GENERIC: [$name],
		WHERE_CLAUSE: [],

		$code, $response, parameters: ($($param),*), valid_error_codes: [$($error),*] $($rest)*
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
			parameters: (
				$(
					$(@DEFAULT($default:expr))?
					$(@RAW($bool:literal))?
					$param:ident: $ty:ty
				),* $(,)?
			),
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
			pub fn new(transaction_id: $crate::communication::TransactionId, session_id: SessionId, $($param: $ty),*) -> Self {
				Self {
					parameters: [ParameterPriv::new_raw(T::CODE as u32).0, $(parse_operations!(@PARAM_CONVERT $(@DEFAULT($default))? $(@RAW($bool))? $param: $ty)),*],
					session_id: Some(session_id),
					transaction_id,
					_phantom: Default::default(),
				}
			}
		}

		parse_operations!(@COMMON_OPERATIONS
		OPERATIONS_ENUM: [$($operations_enum)*]

		$name,
		GENERIC: [T],
		NAME_WITH_GENERIC: [$name<T>],
		WHERE_CLAUSE: [where T: $($where_clause)*],

		$code, $response, parameters: ($($param),*), valid_error_codes: [$($error),*] $($rest)*
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
		$response:ty,
		parameters: ($($param:ident),*),
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
			if counter([$(replace_expr!($param ())),*]) > MAX_PARAMETERS {
				panic!("Too many parameters");
			}
		};

		paste::paste! {
			impl<$($generic)*> $crate::communication::operation::DynOperation for $($name_with_generic)* $($where_clause)* {
				type Response = $response;
				type Error = [<$name Error>];
			}
		}

		paste::paste! {
			#[doc = "Errors that can occur when executing the [`" $name "`] operation"]
			#[derive(Debug, deku::DekuRead)]
			#[deku(ctx = "error_code: u16", id = "error_code")]
			pub enum [<$name Error>] {
				$(
				// TODO: Ask if this should be supported
				#[deku(id = crate::communication::response::$error::CODE)]
				$error($crate::communication::response::$error)
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

	(
		@CONSTRUCTOR
		$name:ident,
		$code:literal,
		SESSION_ID: false,
		($(
			$(@DEFAULT($default:expr))?
			$(@RAW($bool:literal))?
			$param:ident: $ty:ty),* $(,)?
		),
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

	(
		@CONSTRUCTOR
		$name:ident,
		$code:literal,
		($(
			$(@DEFAULT($default:expr))?
			$(@RAW($bool:literal))?
			$param:ident: $ty:ty
		),* $(,)?),
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
	pub struct GetDeviceInfo {
		code: 0x1001,
		parameters: (),
		response: response::GetDeviceInfo,
		valid_error_codes: [ParameterNotSupported]
	}

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
	/// If no session is currently open, a response of [`ResponseCode::SessionNotOpen`] will be returned.
	pub struct CloseSession {
		code: 0x1003,
		parameters: (),
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
		parameters: (),
		response: response::GetStorageIDs,
		valid_error_codes: [
			OperationNotSupported,
			SessionNotOpen,
			InvalidTransactionId,
			ParameterNotSupported,
		]
	}

	/// Get the [`StorageInfo`] of the given [`StorageId`].
	pub struct GetStorageInfo {
		code: 0x1005,
		parameters: (storage: StorageId),
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
		parameters: (
			storage: StorageId,
			@DEFAULT(ObjectFormatCode::from(0))
			format: Option<ObjectFormatCode>,
			@DEFAULT(ObjectHandle::from(0))
			parent: Option<ObjectHandle>
		),
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
		parameters: (
			storage: StorageId,
			@DEFAULT(ObjectFormatCode::from(0))
			format: Option<ObjectFormatCode>,
			@DEFAULT(ObjectHandle::from(0))
			object: Option<ObjectHandle>
		),
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
	pub struct GetObjectInfo {
		code: 0x1008,
		parameters: (object: ObjectHandle),
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
		parameters: (object: ObjectHandle),
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
		parameters: (object: ObjectHandle),
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
		parameters: (object: ObjectHandle, format: ObjectFormatCode),
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
		parameters: (
			@DEFAULT(StorageId::from(0))
			destination: Option<StorageId>,
			@DEFAULT(ObjectHandle::from(0))
			parent: Option<ObjectHandle>
		),
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
			SpecificationOfDestinationUnsupported,
		]
	}

	/// Received from the device after a successful [`SendObjectInfo`], contains the binary data of the new object
	pub struct SendObject {
		code: 0x100d,
		parameters: (),
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
		parameters: (
			@DEFAULT(StorageId::from(0))
			storage: Option<StorageId>,
			@DEFAULT(ObjectFormatCode::Unknown(0))
			format: Option<ObjectFormatCode>
		),
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
		parameters: (
			storage: StorageId,
			format: FilesystemType
		),
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
		parameters: (),
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
		parameters: (test_type: SelfTestType),
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
		parameters: (object: ObjectHandle, status: ProtectionStatus),
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
		parameters: (),
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
	pub struct GetDevicePropDesc {
		code: 0x1014,
		parameters: (code: DevicePropCode),
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
	pub struct GetDevicePropValue {
		code: 0x1015,
		parameters: (code: DevicePropCode),
		response: response::GetDevicePropValue,
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
		parameters: (code: DevicePropCode),
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
		parameters: (code: DevicePropCode),
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
		parameters: (transaction: TransactionId),
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
	pub struct MoveObject {
		code: 0x1019,
		parameters: (object: ObjectHandle, storage: StorageId, parent: ObjectHandle),
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
	pub struct CopyObject {
		code: 0x101A,
		parameters: (object: ObjectHandle, storage: StorageId, parent: ObjectHandle),
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
	pub struct GetPartialObject {
		code: 0x101B,
		parameters: (object: ObjectHandle, @RAW(true) offset: u32, @RAW(true) len: u32),
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
	pub struct InitiateOpenCapture {
		code: 0x101C,
		parameters: (
			@DEFAULT(StorageId::from(0))
			storage: Option<StorageId>,
			@DEFAULT(ObjectFormatCode::Unknown(0))
			format: Option<ObjectFormatCode>
		),
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
		parameters: (format: ObjectFormatCode),
		response: response::GetObjectPropsSupported,
		valid_error_codes: [
			OperationNotSupported,
			DeviceBusy,
			InvalidTransactionId,
			InvalidObjectFormatCode,
		]
	}

	/// Get the property description for the given object property code
	pub partial struct GetObjectPropDesc<T>
		where T: [ObjectProperty + core::cmp::Eq + core::fmt::Debug + Clone + for<'b> deku::DekuReader<'b>]
	{
		code: 0x9802,
		parameters: (format: ObjectFormatCode),
		response: response::GetObjectPropDesc<T>,
		valid_error_codes: [
			OperationNotSupported,
			SessionNotOpen,
			InvalidTransactionId,
			AccessDenied,
			InvalidObjectPropCode,
			InvalidObjectFormatCode,
			DeviceBusy,
		]
	}

	/// Get the value for the given object property code
	pub struct GetObjectPropValue {
		code: 0x9803,
		parameters: (object: ObjectHandle, code: ObjectPropertyCode),
		response: response::GetObjectPropValue,
		valid_error_codes: [
			OperationNotSupported,
			SessionNotOpen,
			InvalidTransactionId,
			InvalidObjectPropCode,
			DeviceBusy,
			InvalidObjectHandle,
		]
	}

	/// Set the value for the given object property code
	pub struct SetObjectPropValue {
		code: 0x9804,
		parameters: (object: ObjectHandle, code: ObjectPropertyCode),
		response: response::SetObjectPropValue,
		valid_error_codes: [
			SessionNotOpen,
			InvalidTransactionId,
			AccessDenied,
			InvalidObjectPropCode,
			InvalidObjectHandle,
			DeviceBusy,
			InvalidObjectPropFormat,
			InvalidObjectPropValue,
		]
	}

	/// Get an array of all active [`ObjectHandle`]s
	pub struct GetObjectReferences {
		code: 0x9810,
		parameters: (object: ObjectHandle),
		response: response::GetObjectReferences,
		valid_error_codes: [
			OperationNotSupported,
			SessionNotOpen,
			InvalidTransactionId,
			InvalidObjectHandle,
			StoreNotAvailable,
		]
	}

	// TODO: data on operation
	/// Replace the references on an object
	pub struct SetObjectReferences {
		code: 0x9811,
		parameters: (object: ObjectHandle),
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

	// TODO: bad newtypes
	/// Update the playback of the current object
	///
	/// The `skip` determines the depth and direction into the playback queue. Meaning a value of 1
	/// indicates the device should skip ahead one media object, and a value of -1 indicates the
	/// device should skip back one media object.
	pub struct Skip {
		code: 0x9820,
		parameters: (@RAW(true) skip: u32),
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

	// TODO: Optional parameters
	// TODO: group and depth
	/// Get a list containing all specified object properties
	///
	/// This is a more optimized way of accessing object properties without needing to individually
	/// query each {object, property} pair.
	pub struct GetObjectPropList {
		code: 0x9805,
		parameters: (object: ObjectHandle, format: ObjectFormatCode, prop: ObjectPropertyCode),
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
		parameters: (),
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
		parameters: (format: ObjectFormatCode),
		response: response::GetInterdependentPropDesc,
		valid_error_codes: [
			OperationNotSupported,
			SessionNotOpen,
			InvalidTransactionId,
			DeviceBusy,
			InvalidCodeFormat,
		]
	}

	// TODO: Optional parameters
	// TODO: object size
	pub struct SendObjectPropList {
		code: 0x9808,
		parameters: (destination: StorageId, parent: ObjectHandle, format: ObjectFormatCode),
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
#[deku(endian = "big")]
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
#[repr(transparent)]
pub struct DevicePropCode(u16);

impl From<u16> for DevicePropCode {
	fn from(value: u16) -> Self {
		Self(value)
	}
}

impl From<DevicePropCode> for Parameter {
	fn from(value: DevicePropCode) -> Self {
		Parameter::new(value.0 as u32)
	}
}
