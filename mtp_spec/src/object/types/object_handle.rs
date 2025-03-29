use crate::communication::Parameter;
use crate::object::types::ArrayEncodable;

use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek, Write};
use deku::prelude::{Reader, Writer};
use deku::{DekuError, DekuRead, DekuReader, DekuWrite, DekuWriter};

/// Identifiers that provide a device- and session-unique consistent reference to a
/// logical object on a device.
///
/// Object handles are used in MTP transactions to reference a logical object on the device,
/// but do not necessarily reference actual data constructs on the device.
///
/// Object handles are only persistent within an MTP session; once a session has been re-opened, all
/// previous values shall be assumed to be invalid, and the contents of the Responder must be
/// re-enumerated if object handles are needed
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct ObjectHandle(u32);

impl ObjectHandle {
	pub const NONE: Self = ObjectHandle(0);
}

impl From<u32> for ObjectHandle {
	fn from(value: u32) -> Self {
		ObjectHandle(value)
	}
}

impl From<ObjectHandle> for Parameter {
	fn from(value: ObjectHandle) -> Self {
		Parameter::new(value.0)
	}
}

impl<'a> DekuReader<'a, Endian> for ObjectHandle {
	fn from_reader_with_ctx<R: Read + Seek>(
		reader: &mut Reader<R>,
		ctx: Endian,
	) -> Result<Self, DekuError>
	where
		Self: Sized,
	{
		u32::from_reader_with_ctx(reader, ctx).map(ObjectHandle)
	}
}

impl DekuWriter<Endian> for ObjectHandle {
	fn to_writer<W: Write + Seek>(
		&self,
		writer: &mut Writer<W>,
		ctx: Endian,
	) -> Result<(), DekuError> {
		self.0.to_writer(writer, ctx)
	}
}

// `ObjectHandle` is simply a `u32` wrapper
impl ArrayEncodable for ObjectHandle {}
