//! Responder -> Initiator event definitions

mod impls;
pub use impls::*;

use crate::error::SerializationError;
use crate::object::ArrayEncodable;

use alloc::vec::Vec;

use deku::DekuContainerRead;

impl TryFrom<Vec<u8>> for Event {
    type Error = SerializationError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        match EventsParser::from_bytes((&value, 0)) {
            Ok((_, event)) => Ok(Event::from(event)),
            Err(e) => Err(SerializationError::from(e)),
        }
    }
}

impl ArrayEncodable for EventCode {}
