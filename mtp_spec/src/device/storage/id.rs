use crate::communication::Parameter;
use crate::object::types::ArrayEncodable;

use alloc::vec::Vec;

use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek, Write};
use deku::prelude::{Reader, Writer};
use deku::{DekuError, DekuRead, DekuReader, DekuWrite, DekuWriter};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, DekuRead, DekuWrite)]
#[deku(endian = "big")]
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

impl<'a> DekuReader<'a, Endian> for StorageId {
	fn from_reader_with_ctx<R: Read + Seek>(
		reader: &mut Reader<R>,
		ctx: Endian,
	) -> Result<Self, DekuError>
	where
		Self: Sized,
	{
		u32::from_reader_with_ctx(reader, ctx).map(StorageId)
	}
}

impl DekuWriter<Endian> for StorageId {
	fn to_writer<W: Write + Seek>(
		&self,
		writer: &mut Writer<W>,
		ctx: Endian,
	) -> Result<(), DekuError> {
		self.0.to_writer(writer, ctx)
	}
}

// `StorageId` is simply a `u32` wrapper
impl ArrayEncodable for StorageId {}
