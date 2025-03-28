use crate::communication::response::ResponseFlags;
use crate::communication::{Parameter, SessionId, TransactionId};
use crate::error::Result;
use crate::object::types::ArrayEncodable;

use alloc::vec::Vec;
use core::fmt::Debug;

use deku::no_std_io::{Cursor, Read, Seek};
use deku::reader::Reader;
use deku::writer::Writer;
use deku::{DekuContainerRead, DekuContainerWrite, DekuReader, DekuWrite, DekuWriter};

mod impls;
pub use impls::*;

/// A prepared operation, ready for transmission
///
/// Every operation type can be converted into this. It cannot be constructed directly.
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuWrite)]
pub struct SerializedOperation<'a> {
	pub code: u16,
	pub session_id: SessionId,
	pub transaction_id: TransactionId,
	pub parameters: &'a [Parameter],
}

impl SerializedOperation<'_> {
	pub fn encode_parameters(&self) -> Result<Vec<u8>> {
		let mut buf = Vec::with_capacity(size_of_val(self.parameters));
		for param in self.parameters.iter() {
			buf.extend(param.to_bytes()?);
		}
		Ok(buf)
	}
}

pub trait DynOperation
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

impl ArrayEncodable for Operation {}
