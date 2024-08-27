use crate::communication::Parameter;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct StorageId(u16, u16);

impl StorageId {
	pub const ALL_STORAGES: Self = StorageId(0xFFFF, 0xFFFF);
	pub const DEFAULT_STORE: Self = StorageId(0x0000, 0x0000);
}

impl From<u32> for StorageId {
	fn from(value: u32) -> Self {
		StorageId((value >> 16) as u16, value as u16)
	}
}

impl From<StorageId> for Parameter {
	fn from(value: StorageId) -> Self {
		Parameter::new((value.0 as u32) << 16 | value.1 as u32)
	}
}
