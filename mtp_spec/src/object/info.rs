use crate::communication::Parameter;
use crate::device::storage::id::StorageId;
use crate::object::types::{Association, DateTime, ObjectFormatCode, ObjectHandle, PtpString};

use deku::{DekuRead, DekuWrite};

/// The write-protection status of an object
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, DekuRead, DekuWrite)]
#[repr(u16)]
#[deku(
	id_type = "u16",
	endian = "big",
	ctx = "_endian: deku::ctx::Endian",
	ctx_default = "deku::ctx::Endian::Little"
)]
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
	/// retrieved from the device using a [`GetObject`] operation.
	///
	/// [`GetObject`]: crate::communication::operation::GetObject
	#[deku(id = "0x8003")]
	NonTransferableData = 0x8003,
	/// All other values
	#[deku(id_pat = "_", default)]
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

/// PTP-compatible thumbnail information for image objects
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
pub struct Thumbnail {
	/// The format of the image
	format: ObjectFormatCode,
	/// The size of the data component of the object in bytes.
	///
	/// If the object is larger than `2^32` bytes in size (4GB), this field shall contain a value
	/// of [`u32::MAX`].
	compressed_size: u32,
	/// The width in pixels
	#[deku(endian = "big")]
	pub width: u32,
	/// The height in pixels
	#[deku(endian = "big")]
	pub height: u32,
	/// The bit depth of the image
	#[deku(endian = "big")]
	pub bit_depth: u32,
}

/// Information about an object residing on the responder
#[derive(Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
pub struct ObjectInfo {
	/// The storage in which this object is located
	pub storage_id: StorageId,
	/// The format of the object's binary data
	pub object_format: ObjectFormatCode,
	/// The write-protection status of this object
	pub protection_status: ProtectionStatus,
	/// The size of the data component of the object in bytes.
	///
	/// If the object is larger than `2^32` bytes in size (4GB), this field shall contain a value
	/// of [`u32::MAX`].
	#[deku(endian = "big")]
	pub compressed_size: u32,
	/// PTP-compatible thumbnail information for image objects
	///
	/// This field will most likely be unused by responders.
	pub thumbnail: Option<Thumbnail>,
	/// The parent of this object, if it exists in a hierarchy
	pub parent_object: ObjectHandle,
	/// The association of this object, if it is an association
	pub association: Association,
	/// Unused in MTP, but required by PTP
	#[deku(endian = "big")]
	pub sequence_number: u32,
	/// The file name of this object, without any directory or file system information.
	pub filename: PtpString,
	/// The creation date of this object
	pub date_created: DateTime,
	/// The last modification date of this object
	pub date_modified: DateTime,
	/// Keywords associated with the object, separated by ' '
	pub keywords: PtpString,
}
