use alloc::format;
use alloc::vec::Vec;

use deku::{DekuRead, DekuWrite};

/// The type of the collection to which an object is associated.
#[repr(u16)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, DekuRead, DekuWrite)]
#[deku(id_type = "u16", endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub enum AssociationType {
	#[deku(id = "0x0000")]
	Undefined = 0x0000,
	#[deku(id = "0x0001")]
	GenericFolder = 0x0001,
	#[deku(id = "0x0002")]
	Album = 0x0002,
	#[deku(id = "0x0003")]
	TimeSequence = 0x0003,
	#[deku(id = "0x0004")]
	HorizontalPanoramic = 0x0004,
	#[deku(id = "0x0005")]
	VerticalPanoramic = 0x0005,
	#[deku(id = "0x0006")]
	Panoramic2d = 0x0006,
	#[deku(id = "0x0007")]
	AncillaryData = 0x0007,
	/// All other values with bit 15 set to 0
	#[deku(id_pat = "t if t & 0x8000 == 0")]
	Reserved,
	/// All other values with bit 15 set to 1 and bit 14 set to 0
	#[deku(id_pat = "t if t & 0xC000 == 0x8000")]
	VendorDefined,
	/// All other values with bit 15 set to 1 and bit 14 set to 1
	Mtp,
}

impl From<u16> for AssociationType {
	fn from(value: u16) -> Self {
		match value {
			0x0000 => Self::Undefined,
			0x0001 => Self::GenericFolder,
			0x0002 => Self::Album,
			0x0003 => Self::TimeSequence,
			0x0004 => Self::HorizontalPanoramic,
			0x0005 => Self::VerticalPanoramic,
			0x0006 => Self::Panoramic2d,
			0x0007 => Self::AncillaryData,
			value if value & 0x8000 == 0 => Self::Reserved,
			value if value & 0xC000 == 0x8000 => Self::VendorDefined,
			_ => Self::Mtp,
		}
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct Association {
	/// The type of the collection to which the object is associated.
	pub ty: AssociationType,
	/// An optional descriptor whose meaning varies depending on the association type.
	pub desc: u32,
}
