//! Initiator -> Responder operation definitions

use crate::communication::response::errors::OperationError;
use crate::communication::{Parameter, SessionId, TransactionId};
use crate::error::SerializationError;
use crate::object::types::ArrayEncodable;

use alloc::vec::Vec;
use core::fmt::Debug;
use deku::ctx::Endian;
use deku::no_std_io::{Cursor, Read, Seek, Write};
use deku::reader::Reader;
use deku::writer::Writer;
use deku::{DekuError, DekuReader, DekuWrite, DekuWriter};

pub mod android;
use android::AndroidOperation;

mod impls;
pub use impls::*;

/// All operation codes
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Operation {
    /// Base operations from the MTP specification
    Base(BaseOperation),
    /// Android-specific MTP extensions
    Android(AndroidOperation),
    /// Some other vendor-specific MTP extension
    VendorSpecific(u16),
}

impl TryFrom<u16> for Operation {
    type Error = ();

    fn try_from(value: u16) -> core::result::Result<Self, Self::Error> {
        if (0x9000_u16..=0x97FF_u16).contains(&value) {
            return Ok(AndroidOperation::try_from(value)
                .map_or(Operation::VendorSpecific(value), Operation::Android));
        }

        BaseOperation::try_from(value).map(Operation::Base)
    }
}

impl DekuReader<'_, Endian> for Operation {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        ctx: Endian,
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let value = u16::from_reader_with_ctx(reader, ctx)?;

        match Operation::try_from(value) {
            Ok(op) => Ok(op),
            Err(_) => Err(DekuError::Parse(
                format!("Opcode {value:#X} is not valid").into(),
            )),
        }
    }
}

impl DekuWriter<Endian> for Operation {
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        ctx: Endian,
    ) -> Result<(), DekuError> {
        match self {
            Operation::Base(op) => (*op as u16).to_writer(writer, ctx),
            Operation::Android(op) => (*op as u16).to_writer(writer, ctx),
            Operation::VendorSpecific(op) => op.to_writer(writer, ctx),
        }
    }
}

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
    pub fn encode_parameters(&self, endian: Endian) -> Result<Vec<u8>, SerializationError> {
        let mut buf = Vec::with_capacity(size_of_val(self.parameters));

        let mut writer = Writer::new(Cursor::new(&mut buf));
        for param in self.parameters {
            param.to_writer(&mut writer, endian)?;
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
pub trait DynOperation: Send
where
    for<'a> SerializedOperation<'a>: From<&'a Self>,
{
    /// The direction in which data is transferred in an operation, if applicable
    const DATA_DIRECTION: Option<DataDirection>;

    /// The response type for this operation, see [`response`](crate::communication::response)
    type Response: Clone + Debug + PartialEq + for<'b> DekuReader<'b, Endian>;

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
    fn decode_data(bytes: &[u8], endian: Endian) -> Result<Self::Response, SerializationError> {
        match Self::Response::from_reader_with_ctx(&mut Reader::new(Cursor::new(bytes)), endian) {
            Ok(response) => Ok(response),
            Err(err) => Err(err.into()),
        }
    }

    /// Decode the error from the responder into the expected type
    ///
    /// # Errors
    ///
    /// This will fail if the data does not match the expected type, which may indicate an issue
    /// with the responder.
    fn decode_err(
        bytes: &[u8],
        endian: Endian,
        code: u16,
    ) -> Result<OperationError, SerializationError> {
        OperationError::from_reader_with_ctx(&mut Reader::new(Cursor::new(bytes)), (endian, code))
            .map_err(Into::into)
    }
}

impl ArrayEncodable for Operation {}
