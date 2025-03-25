use super::ResponseFlags;
use crate::device::info::DeviceInfo;
use crate::device::storage::id::StorageId;
use crate::object::types::Array;

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
			$(data: $data:ty,)?
			$(parameters: ($($param:ident: $ty:ty),* $(,)?),)?
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
			$(pub data: $data,)?
			$(
				$(pub $param: $ty),*
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
