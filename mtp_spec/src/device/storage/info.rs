#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u16)]
pub enum StorageType {
	Undefined = 0x0000,
	FixedRom = 0x0001,
	RemovableRom = 0x0002,
	FixedRam = 0x0003,
	RemovableRam = 0x0004,
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u16)]
pub enum FilesystemType {
	Undefined = 0x0000,
	GenericFlat = 0x0001,
	GenericHierarchical = 0x0002,
	Dcf = 0x0003,
	Reserved,
	MtpVendorExtension,
	MtpDefined,
}

impl From<u16> for FilesystemType {
	fn from(value: u16) -> Self {
		match value {
			0x0000 => FilesystemType::Undefined,
			0x0001 => FilesystemType::GenericFlat,
			0x0002 => FilesystemType::GenericHierarchical,
			0x0003 => FilesystemType::Dcf,
			_ => FilesystemType::Reserved,
		}
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u16)]
pub enum AccessCapability {
	ReadWrite = 0x0000,
	ReadOnlyNoObjectDeletion = 0x0001,
	ReadOnlyWithObjectDeletion = 0x0002,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageInfo {
	pub storage_type: StorageType,
	pub filesystem_type: FilesystemType,
	pub access_capability: AccessCapability,
	pub max_capacity: u64,
	/// How much space remains to be written to on the drive (**in bytes**).
	pub free_space: u64,
	pub free_space_in_objects: Option<u32>,
	/// A human-readable string identifying this storage, such as "256Mb SD Card" or "20Gb HDD"
	pub storage_description: Option<String>,
	/// A unique, programmatically relevant volume identifier, such as a serial
	/// number. This field may be up to 255 characters long, however, only the first 128
	/// characters will be used to identify the device, and these first 128 characters must be
	/// unique for all storages.
	pub volume_identifier: String,
}
