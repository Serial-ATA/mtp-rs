use super::ResponseFlags;
use crate::device::info::DeviceInfo;
use crate::device::property_describing::{DevicePropDesc, PropertyValue, PropertyValueWrapper};
use crate::device::storage::id::StorageId;
use crate::device::storage::info::StorageInfo;
use crate::object::info::{ObjectInfo, Thumbnail};
use crate::object::types::{Array, ObjectHandle};

use alloc::vec::Vec;

pub(super) const fn counter<const N: usize>(_: [(); N]) -> usize {
	N
}

macro_rules! replace_expr {
	($_t:tt $sub:expr) => {
		$sub
	};
}

pub(super) const MAX_PARAMETERS: usize = 5;

macro_rules! define_response {
	(
		$(#[$meta:meta])*
		pub struct $name:ident {
			$(
			$(#[$deku_meta:meta])*
			data: $data:ty,
			)?
			$(parameters: (
				$(
					$(#[$param_meta:meta])*
					$param:ident: $ty:ty
				),* $(,)?
			),)?
		}
	) => {
		$(
			const _: () = {
				if $crate::communication::response::impls::counter(
					[$($crate::communication::response::impls::replace_expr!($param ())),*]
				) > MAX_PARAMETERS {
					panic!("Too many parameters");
				}
			};
		)?

		$(#[$meta])*
		#[derive(Clone, Debug, PartialEq, Eq, deku::DekuRead)]
		pub struct $name {
			$(
			$(#[$deku_meta])*
			pub data: $data,
			)?
			$(
				$(
				$(#[$param_meta])*
				pub $param: $ty
				),*
			)?
		}

		impl $crate::communication::response::ResponseFlags for $name {}

		impl super::sealed::Sealed for $name {}
	}
}

pub(super) use {define_response, replace_expr};

/// Empty response, device has nothing to provide
#[derive(Clone, Debug, PartialEq, Eq, deku::DekuRead)]
pub struct Empty;

impl ResponseFlags for Empty {
	const EXPECTS_DATA: bool = false;
}

define_response! {
	/// Response to the [`GetDeviceInfo`] operation.
	pub struct GetDeviceInfo {
		data: DeviceInfo,
	}
}

define_response! {
	/// Response to the [`GetStorageIDs`] operation.
	pub struct GetStorageIDs {
		data: Array<StorageId>,
	}
}

define_response! {
	/// Response to the [`GetStorageInfo`] operation.
	pub struct GetStorageInfo {
		data: StorageInfo,
	}
}

define_response! {
	/// Response to the [`GetNumObjects`] operation.
	pub struct GetNumObjects {
		parameters: (num_objects: u32),
	}
}

define_response! {
	/// Response to the [`GetObjectHandles`] operation.
	pub struct GetObjectHandles {
		data: Array<ObjectHandle>,
	}
}

define_response! {
	/// Response to the [`GetObjectInfo`] operation.
	pub struct GetObjectInfo {
		data: ObjectInfo,
	}
}

define_response! {
	/// Response to the [`GetObject`] operation.
	pub struct GetObject {
		#[deku(read_all)]
		data: Vec<u8>,
	}
}

define_response! {
	/// Response to the [`GetThumb`] operation.
	pub struct GetThumb {
		data: Thumbnail,
	}
}

define_response! {
	/// Response to the [`SendObjectInfo`] operation.
	pub struct SendObjectInfo {
		data: ObjectInfo,
		parameters: (
			/// The storage id of the incoming object
			storage_id: StorageId,
			/// The parent of the incoming object
			parent: ObjectHandle,
			/// The responder's reserved handle for the incoming object
			reserved_handle: ObjectHandle,
		),
	}
}

define_response! {
	/// Response to the [`SendObject`] operation.
	pub struct SendObject {
		#[deku(read_all)]
		data: Vec<u8>,
	}
}

define_response! {
	/// Response to the [`GetDevicePropDesc`] operation.
	pub struct GetDevicePropDesc {
		data: DevicePropDesc,
	}
}

define_response! {
	/// Response to the [`GetDevicePropValue`] operation.
	pub struct GetDevicePropValue {
		data: PropertyValueWrapper,
	}
}

define_response! {
	/// Response to the [`SetDevicePropValue`] operation.
	pub struct SetDevicePropValue {
		data: PropertyValueWrapper,
	}
}

define_response! {
	/// Response to the [`GetPartialObject`] operation.
	pub struct GetPartialObject {
		#[deku(read_all)]
		data: Vec<u8>,
	}
}

define_response! {
	/// Response to the [`GetObjectPropsSupported`] operation.
	pub struct GetObjectPropsSupported {
		data: Array<u32>,
	}
}

define_response! {
	/// Response to the [`GetObjectPropDesc`] operation.
	pub struct GetObjectPropDesc {
		// TODO: Determine what this even is
		#[deku(read_all)]
		data: Vec<u8>,
	}
}

define_response! {
	/// Response to the [`GetObjectPropValue`] operation.
	pub struct GetObjectPropValue {
		// TODO: Determine what this even is
		#[deku(read_all)]
		data: Vec<u8>,
	}
}

define_response! {
	/// Response to the [`SetObjectPropValue`] operation.
	pub struct SetObjectPropValue {
		// TODO: Determine what this even is
		#[deku(read_all)]
		data: Vec<u8>,
	}
}

define_response! {
	/// Response to the [`GetObjectReferences`] operation.
	pub struct GetObjectReferences {
		data: Array<ObjectHandle>,
	}
}

define_response! {
	/// Response to the [`SetObjectReferences`] operation.
	pub struct SetObjectReferences {
		data: Array<ObjectHandle>,
	}
}
