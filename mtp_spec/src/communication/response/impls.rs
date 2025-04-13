use crate::device::info::DeviceInfo;
use crate::device::property_describing::{DevicePropDesc, PropertyValueWrapper};
use crate::device::storage::id::StorageId;
use crate::device::storage::info::StorageInfo;
use crate::object::info::{ObjectInfo, Thumbnail};
use crate::object::types::properties::ObjectPropertyCode;
use crate::object::types::{Array, ObjectHandle};

use alloc::vec::Vec;
use deku::ctx::Endian;

macro_rules! replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

macro_rules! define_response {
	(
		$(#[$meta:meta])*
		pub struct $name:ident [$($generics:tt)*][$($where_clause:tt)*] {
			$(
				$(#[$deku_meta:meta])*
				data: $data:ty,
			)?
			$(
				parameters: (
					$(
						$(#[$param_meta:meta])*
						$param:ident: $ty:ty
					),* $(,)?
				),
			)?
		}
	) => {
		$(
			const _: () = {
				const fn counter<const N: usize>(_: [(); N]) -> usize {
					N
				}

				const MAX_PARAMETERS: usize = 5;

				if counter(
					[$($crate::communication::response::impls::replace_expr!($param ())),*]
				) > MAX_PARAMETERS {
					panic!("Too many parameters");
				}
			};
		)?

		$(#[$meta])*
		#[derive(Clone, Debug, PartialEq, Eq, deku::DekuRead)]
		#[allow(missing_docs)]
		#[deku(
			endian = "_endian",
			ctx = "_endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Big"
		)]
		pub struct $name $($generics)* $($where_clause)* {
			$(
			$(#[$deku_meta])*
			/// The decoded data from the responder
			pub data: $data,
			)?
			$(
				$(
				$(#[$param_meta])*
				pub $param: $ty
				),*
			)?
		}
	}
}

pub(super) use {define_response, replace_expr};

define_response! {
    /// Empty response, responder has nothing to provide
    pub struct Empty[][] {
        data: (),
    }
}

define_response! {
    /// Response to the [`GetDeviceInfo`] operation.
    pub struct GetDeviceInfo[][] {
        data: DeviceInfo,
    }
}

define_response! {
    /// Response to the [`GetStorageIDs`] operation.
    pub struct GetStorageIDs[][] {
        data: Array<StorageId>,
    }
}

define_response! {
    /// Response to the [`GetStorageInfo`] operation.
    pub struct GetStorageInfo[][] {
        data: StorageInfo,
    }
}

define_response! {
    /// Response to the [`GetNumObjects`] operation.
    pub struct GetNumObjects[][] {
        parameters: (num_objects: u32),
    }
}

define_response! {
    /// Response to the [`GetObjectHandles`] operation.
    pub struct GetObjectHandles[][] {
        data: Array<ObjectHandle>,
    }
}

define_response! {
    /// Response to the [`GetObjectInfo`] operation.
    pub struct GetObjectInfo[][] {
        data: ObjectInfo,
    }
}

define_response! {
    /// Response to the [`GetObject`] operation.
    pub struct GetObject[][] {
        #[deku(read_all)]
        data: Vec<u8>,
    }
}

define_response! {
    /// Response to the [`GetThumb`] operation.
    pub struct GetThumb[][] {
        data: Thumbnail,
    }
}

define_response! {
    /// Response to the [`SendObjectInfo`] operation.
    pub struct SendObjectInfo[][] {
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
    pub struct SendObject[][] {
        #[deku(read_all)]
        data: Vec<u8>,
    }
}

define_response! {
    /// Response to the [`GetDevicePropDesc`] operation.
    pub struct GetDevicePropDesc[][] {
        data: DevicePropDesc,
    }
}

define_response! {
    /// Response to the [`GetDevicePropValue`] operation.
    pub struct GetDevicePropValue[][] {
        data: PropertyValueWrapper,
    }
}

define_response! {
    /// Response to the [`SetDevicePropValue`] operation.
    pub struct SetDevicePropValue[][] {
        data: PropertyValueWrapper,
    }
}

define_response! {
    /// Response to the [`GetPartialObject`] operation.
    pub struct GetPartialObject[][] {
        #[deku(read_all)]
        data: Vec<u8>,
    }
}

define_response! {
    /// Response to the [`GetObjectPropsSupported`] operation.
    pub struct GetObjectPropsSupported[][] {
        data: Array<ObjectPropertyCode>,
    }
}

define_response! {
    /// Response to the [`GetObjectPropDesc`] operation.
    pub struct GetObjectPropDesc[<T>][where T: for<'a> deku::DekuReader<'a, Endian>] {
        data: T,
    }
}

define_response! {
    /// Response to the [`GetObjectPropValue`] operation.
    pub struct GetObjectPropValue[][] {
        // TODO: Determine what this even is
        #[deku(read_all)]
        data: Vec<u8>,
    }
}

define_response! {
    /// Response to the [`GetObjectReferences`] operation.
    pub struct GetObjectReferences[][] {
        data: Array<ObjectHandle>,
    }
}

define_response! {
    /// Response to the [`SetObjectReferences`] operation.
    pub struct SetObjectReferences[][] {
        data: Array<ObjectHandle>,
    }
}

// Enhanced Operations
//
// Defined in Appendix E

define_response! {
    /// Response to the [`GetObjectPropList`] operation.
    pub struct GetObjectPropList[][] {
        // TODO: Determine what this even is
        #[deku(read_all)]
        data: Vec<u8>,
    }
}

define_response! {
    /// Response to the [`SetObjectPropList`] operation.
    pub struct SetObjectPropList[][] {
        // TODO: Determine what this even is
        #[deku(read_all)]
        data: Vec<u8>,
    }
}

define_response! {
    /// Response to the [`GetInterdependentPropDesc`] operation.
    pub struct GetInterdependentPropDesc[][] {
        // TODO: Determine what this even is
        #[deku(read_all)]
        data: Vec<u8>,
    }
}

define_response! {
    /// Response to the [`SendObjectPropList`] operation.
    pub struct SendObjectPropList[][] {
        // TODO: Determine what this even is
        #[deku(read_all)]
        data: Vec<u8>,
    }
}
