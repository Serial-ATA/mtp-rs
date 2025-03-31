use crate::communication::Parameter;
use crate::object::types::ArrayEncodable;

/// A storage identifier
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, deku::DekuRead, deku::DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct StorageId(u32);

impl StorageId {
    pub const ALL_STORAGES: Self = StorageId(0xFFFF_FFFF);
    pub const DEFAULT_STORE: Self = StorageId(0x0000_0000);
}

impl From<u32> for StorageId {
    fn from(value: u32) -> Self {
        StorageId(value)
    }
}

impl From<StorageId> for Parameter {
    fn from(value: StorageId) -> Self {
        Parameter::new(value.0)
    }
}

// `StorageId` is simply a `u32` wrapper
impl ArrayEncodable for StorageId {}
