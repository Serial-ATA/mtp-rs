use crate::communication::Parameter;
use crate::object::PtpString;

use deku::DekuRead;

/// The physical nature of a storage, as described in [`StorageInfo`]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, DekuRead)]
#[repr(u16)]
#[deku(
    id_type = "u16",
    id_endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[allow(missing_docs)]
pub enum StorageType {
    Undefined = 0x0000,
    FixedRom = 0x0001,
    RemovableRom = 0x0002,
    FixedRam = 0x0003,
    RemovableRam = 0x0004,
    #[deku(id_pat = "_", default)]
    Reserved,
}

impl From<u16> for StorageType {
    fn from(value: u16) -> Self {
        match value {
            0x0000 => StorageType::Undefined,
            0x0001 => StorageType::FixedRom,
            0x0002 => StorageType::RemovableRom,
            0x0003 => StorageType::FixedRam,
            0x0004 => StorageType::RemovableRam,
            _ => StorageType::Reserved,
        }
    }
}

/// The logical file system in use on a storage, as described in [`StorageInfo`]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, DekuRead)]
#[repr(u16)]
#[deku(
    id_type = "u16",
    id_endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[allow(missing_docs)]
pub enum FilesystemType {
    Undefined = 0x0000,
    GenericFlat = 0x0001,
    GenericHierarchical = 0x0002,
    Dcf = 0x0003,
    #[deku(id_pat = "t if (0x0000_u16..=0x7FFF_u16).contains(t)")]
    Reserved,
    #[deku(id_pat = "t if (0x8000_u16..=0xBFFF_u16).contains(t)")]
    MtpVendorExtension,
    #[deku(id_pat = "t if (0xC000_u16..=0xFFFF_u16).contains(t)")]
    MtpDefined,
}

impl From<FilesystemType> for Parameter {
    fn from(value: FilesystemType) -> Self {
        Parameter::new(value as u32)
    }
}

/// Globally applicable write-protection affecting a storage, as described in [`StorageInfo`]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, DekuRead)]
#[repr(u16)]
#[deku(
    id_type = "u16",
    id_endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[allow(missing_docs)]
pub enum AccessCapability {
    ReadWrite = 0x0000,
    ReadOnlyNoObjectDeletion = 0x0001,
    ReadOnlyWithObjectDeletion = 0x0002,
    #[deku(id_pat = "_", default)]
    Reserved,
}

impl From<u16> for AccessCapability {
    fn from(value: u16) -> Self {
        match value {
            0x0000 => AccessCapability::ReadWrite,
            0x0001 => AccessCapability::ReadOnlyNoObjectDeletion,
            0x0002 => AccessCapability::ReadOnlyWithObjectDeletion,
            _ => AccessCapability::Reserved,
        }
    }
}

/// Description of a storage contained in a device
#[derive(Clone, Debug, Eq, PartialEq, DekuRead)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct StorageInfo {
    /// The physical nature of the storage
    pub storage_type: StorageType,
    /// The logical file system in use on the storage
    pub filesystem_type: FilesystemType,
    /// Globally-applicable write-protection affecting this storage
    pub access_capability: AccessCapability,
    /// The maximum capacity of the storage (**in bytes**).
    ///
    /// NOTE: This field is optional if the access capability is not [`ReadWrite`](AccessCapability::ReadWrite).
    pub max_capacity: u64,
    /// How much space remains to be written to on the drive (**in bytes**).
    ///
    /// NOTES:
    ///
    /// * This field is optional if the access capability is not [`ReadWrite`](AccessCapability::ReadWrite).
    /// * If the access capability is [`ReadWrite`](AccessCapability::ReadWrite), but this field doesn't apply,
    ///   then its value will be [`u64::MAX`].
    /// * The `free_space_in_objects` field may be used by the responder instead.
    pub free_space: u64,
    /// The number of additional objects that can be written to this storage.
    ///
    /// NOTE: If the field is unused, its value will be [`u32::MAX`].
    pub free_space_in_objects: u32,
    /// A human-readable string identifying this storage, such as "256Mb SD Card" or "20Gb HDD"
    #[deku(map = "PtpString::parse_optional")]
    pub storage_description: Option<PtpString>,
    /// A unique, programmatically relevant volume identifier, such as a serial number.
    pub volume_identifier: PtpString,
}
