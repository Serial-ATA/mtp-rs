use crate::communication::Parameter;
use crate::device::storage::id::StorageId;
use crate::object::types::association::Association;
use crate::object::types::datetime::DateTime;
use crate::object::types::format_code::ObjectFormatCode;
use crate::object::types::object_handle::ObjectHandle;
use crate::object::types::PtpString;

use alloc::format;
use alloc::vec::Vec;

use deku::{DekuRead, DekuWrite};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, DekuRead, DekuWrite)]
#[repr(u16)]
#[deku(id_type = "u16", endian = "big")]
pub enum ProtectionStatus {
	/// This object has no protection; it may be modified or deleted arbitrarily and its properties may be modified freely.
	#[deku(id = "0x0000")]
	NoProtection = 0x0000,
	/// This object cannot be deleted or modified; none of the properties of
	/// this object can be modified by the initiator. (However, properties can be modified
	/// by the device that contains the object.)
	#[deku(id = "0x0001")]
	ReadOnly = 0x0001,
	/// This object’s binary component cannot be deleted or modified;
	/// however; any object properties may be modified if allowed by the object property
	/// constraints.
	#[deku(id = "0x8002")]
	ReadOnlyData = 0x8002,
	/// This object’s properties may be read and modified, and it
	/// may be moved or deleted on the device, but this object’s binary data may not be
	/// retrieved from the device using a GetObject operation.
	#[deku(id = "0x8003")]
	NonTransferableData = 0x8003,
	Reserved,
}

impl From<u16> for ProtectionStatus {
	fn from(value: u16) -> Self {
		match value {
			0x0000 => ProtectionStatus::NoProtection,
			0x0001 => ProtectionStatus::ReadOnly,
			0x8002 => ProtectionStatus::ReadOnlyData,
			0x8003 => ProtectionStatus::NonTransferableData,
			_ => ProtectionStatus::Reserved,
		}
	}
}

impl From<ProtectionStatus> for Parameter {
	fn from(value: ProtectionStatus) -> Self {
		Parameter::new(value as u32)
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
pub struct Thumbnail {
	pub format: ObjectFormatCode,
	#[deku(endian = "big")]
	pub compressed_size: u32,
	#[deku(endian = "big")]
	pub width: u32,
	#[deku(endian = "big")]
	pub height: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, DekuRead)]
pub struct ObjectInfo {
	pub storage_id: StorageId,
	pub object_format: ObjectFormatCode,
	pub protection_status: ProtectionStatus,
	/// The size of the data component of the object in bytes. If the object is larger than `2^32`
	/// bytes in size (4GB), this field shall contain a value of `0xFFFFFFFF`.
	#[deku(endian = "big")]
	pub compressed_size: u32,
	pub thumbnail: Option<Thumbnail>,
	#[deku(endian = "big")]
	pub image_pix_width: u32,
	#[deku(endian = "big")]
	pub image_pix_height: u32,
	#[deku(endian = "big")]
	pub image_bit_depth: u32,
	pub parent_object: ObjectHandle,
	pub association: Association,
	#[deku(endian = "big")]
	pub sequence_number: u32,
	/// The file name of this object, without any directory or file system
	/// information. This string is also accessible and defined via an Object Property, and
	/// restrictions on its format may be identified in the Object Property Description for this
	/// object property.
	pub filename: PtpString,
	pub date_created: DateTime,
	pub date_modified: DateTime,
	pub keywords: PtpString,
}
