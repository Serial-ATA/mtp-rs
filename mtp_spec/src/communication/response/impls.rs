use crate::device::info::DeviceInfo;
use crate::device::storage::id::StorageId;
use crate::object::types::Array;

use deku::DekuRead;

const fn counter<const N: usize>(_: [(); N]) -> usize {
	N
}

macro_rules! replace_expr {
	($_t:tt $sub:expr) => {
		$sub
	};
}

const MAX_PARAMETERS: usize = 5;

macro_rules! define_response {
	(
		$(#[$meta:meta])*
		pub struct $name:ident {
			$(data: $data:ty,)?
			$(parameters: ($($param:ident: $ty:ty),* $(,)?),)?
		}
	) => {
		$(
			const _: usize = {
				if count_helper([$(replace_expr!($param ())),*]) > MAX_PARAMETERS {
					panic!("Too many parameters");
				}
			}
		)?

		paste::paste! {
			$(#[$meta])*
			#[derive(Clone, Debug, PartialEq, Eq, DekuRead)]
			pub struct [<$name Response>] {
				$(pub data: $data,)?
				$(
					$(pub $param: $ty),*
				)?
			}
		}
	}
}

define_response! {
	/// Empty response, device has nothing to provide
	pub struct Empty {}
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
