use crate::communication::response::{Response, ResponseFlags};
use crate::communication::{response, Parameter, SessionId, TransactionId};
use crate::error::Result;

use alloc::vec::Vec;
use core::fmt::Debug;

use deku::no_std_io::Cursor;
use deku::reader::Reader;
use deku::writer::Writer;
use deku::{DekuContainerRead, DekuReader, DekuWrite, DekuWriter};

mod impls;
pub use impls::*;

/// A prepared operation, ready for transmission
///
/// Every operation type can be converted into this. It cannot be constructed directly.
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuWrite)]
pub struct SerializedOperation<'a> {
	code: u16,
	session_id: SessionId,
	transaction_id: TransactionId,
	parameters: &'a [Parameter],
}

impl SerializedOperation<'_> {
	const HEADER_SIZE: usize =
		size_of::<u16>() + size_of::<SessionId>() + size_of::<TransactionId>();

	pub fn size(&self) -> usize {
		Self::HEADER_SIZE + size_of_val(self.parameters)
	}

	pub fn to_bytes(&self) -> Result<Vec<u8>> {
		deku::DekuContainerWrite::to_bytes(self).map_err(Into::into)
	}

	pub fn write_to<W: deku::no_std_io::Write + deku::no_std_io::Seek>(
		&self,
		writer: &mut W,
	) -> Result<()> {
		self.to_writer(&mut Writer::new(writer), ())
			.map_err(Into::into)
	}
}

pub trait Operation
where
	for<'a> SerializedOperation<'a>: From<&'a Self>,
{
	type Response: Clone + Debug + Eq + PartialEq + ResponseFlags + for<'b> DekuContainerRead<'b>;
	type Error: for<'b> DekuReader<'b, u16>;

	fn encode(&self) -> SerializedOperation<'_> {
		self.into()
	}

	fn decode_data(bytes: &[u8]) -> Result<Self::Response> {
		match DekuContainerRead::from_bytes((bytes, 0)) {
			Ok((_remaining, response)) => Ok(response),
			Err(err) => Err(err.into()),
		}
	}

	fn decode_err(bytes: &[u8], code: u16) -> Result<Self::Error> {
		Self::Error::from_reader_with_ctx(&mut Reader::new(Cursor::new(bytes)), code)
			.map_err(Into::into)
	}
}
