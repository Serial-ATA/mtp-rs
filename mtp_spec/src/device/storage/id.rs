use crate::communication::Parameter;
use crate::object::types::ArrayEncodable;

/// A storage identifier
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, deku::DekuRead, deku::DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct StorageId(u32);

impl StorageId {
    /// Used to perform operations across all storages
    pub const ALL_STORAGES: Self = StorageId(0xFFFF_FFFF);

    /// Leave storage selection up to the responder
    ///
    /// This can be used in contexts like object creation, when there's no preference for storage devices.
    pub const DEFAULT_STORE: Self = StorageId(0x0000_0000);
}

impl Default for StorageId {
    fn default() -> Self {
        StorageId::DEFAULT_STORE
    }
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
