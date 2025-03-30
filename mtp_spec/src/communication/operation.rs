use crate::communication::response::ResponseFlags;
use crate::communication::{Parameter, SessionId, TransactionId};
use crate::error::Result;
use crate::object::types::ArrayEncodable;

use alloc::vec::Vec;
use core::fmt::Debug;

use deku::no_std_io::Cursor;
use deku::reader::Reader;
use deku::{DekuContainerRead, DekuContainerWrite, DekuReader, DekuWrite};

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
    /// Encode the operation parameters for transport
    pub fn encode_parameters(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(size_of_val(self.parameters));
        for param in self.parameters.iter() {
            buf.extend(param.to_bytes()?);
        }
        Ok(buf)
    }

    /// The raw operation code
    pub fn code(&self) -> u16 {
        self.code
    }

    /// The session id (may be [`SessionId::NONE`] if not applicable)
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// The associated transaction id
    pub fn transaction_id(&self) -> TransactionId {
        self.transaction_id
    }
}

/// Common methods for all [`operations`](crate::communication::operation)
pub trait DynOperation
where
    for<'a> SerializedOperation<'a>: From<&'a Self>,
{
    /// The direction in which data is transferred in an operation, if applicable
    const DATA_DIRECTION: Option<DataDirection>;

    /// The response type for this operation, see [`response`](crate::communication::response)
    type Response: Clone + Debug + Eq + PartialEq + ResponseFlags + for<'b> DekuContainerRead<'b>;

    /// The error type for this operation
    ///
    /// This comes from the responder, see [`response::errors`](crate::communication::response::errors)
    type Error: for<'b> DekuReader<'b, u16>;

    /// Encode the operation for transport
    fn encode(&self) -> SerializedOperation<'_> {
        self.into()
    }

    /// Decode the data from the responder into the expected type
    ///
    /// # Errors
    ///
    /// This will fail if the data does not match the expected type, which may indicate an issue
    /// with the responder.
    fn decode_data(bytes: &[u8]) -> Result<Self::Response> {
        match DekuContainerRead::from_bytes((bytes, 0)) {
            Ok((_remaining, response)) => Ok(response),
            Err(err) => Err(err.into()),
        }
    }

    /// Decode the error from the responder into the expected type
    ///
    /// # Errors
    ///
    /// This will fail if the data does not match the expected type, which may indicate an issue
    /// with the responder.
    fn decode_err(bytes: &[u8], code: u16) -> Result<Self::Error> {
        Self::Error::from_reader_with_ctx(&mut Reader::new(Cursor::new(bytes)), code)
            .map_err(Into::into)
    }
}

impl ArrayEncodable for Operation {}
