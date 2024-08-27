/// The type of the collection to which an object is associated.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u16)]
pub enum AssociationType {
	Undefined = 0x0000,
	GenericFolder = 0x0001,
	Album = 0x0002,
	TimeSequence = 0x0003,
	HorizontalPanoramic = 0x0004,
	VerticalPanoramic = 0x0005,
	Panoramic2d = 0x0006,
	AncillaryData = 0x0007,
	/// All other values with bit 15 set to 0
	Reserved,
	/// All other values with bit 15 set to 1 and bit 14 set to 0
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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Association {
	/// The type of the collection to which the object is associated.
	pub ty: AssociationType,
	/// An optional descriptor whose meaning varies depending on the association type.
	pub desc: u32,
}
