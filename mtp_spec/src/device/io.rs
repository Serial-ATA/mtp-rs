use crate::communication::event::Event;
use crate::communication::operation::{DataDirection, DynOperation, SerializedOperation};
use crate::communication::response::Response;
use crate::communication::{SessionId, TransactionId};
use crate::error::{MtpError, SerializationError};

use alloc::vec::Vec;

use deku::ctx::Endian;
use futures_core::Stream;

/// A bundle of an operation and its associated data
#[non_exhaustive]
pub struct OperationBundle<O> {
    /// The operation to be sent
    pub operation: O,
    /// The data to be sent with the operation, if applicable
    pub data: Option<Vec<u8>>,
}

impl<O: DynOperation> OperationBundle<O>
where
    for<'a> SerializedOperation<'a>: From<&'a O>,
{
    /// Create a new `OperationBundle`
    ///
    /// # Errors
    ///
    /// Depending on the operation's [`DataDirection`]:
    ///
    /// * [`DataDirection::ResponderToInitiator`] - Will error if `data` is not `None`
    /// * [`DataDirection::InitiatorToResponder`] - Will error if `data` is `None`
    /// * `None` - Will error if `data` is not `None`
    ///
    /// [`DataDirection`]: DynOperation::DATA_DIRECTION
    pub fn new(operation: O, data: Option<Vec<u8>>) -> Result<Self, SerializationError> {
        match &data {
            Some(_) => match O::DATA_DIRECTION {
                Some(DataDirection::ResponderToInitiator) => {
                    return Err(SerializationError::WrongDataDirection);
                },
                None => {
                    return Err(SerializationError::UnexpectedDataProvided);
                },
                _ => {},
            },
            None => {
                if O::DATA_DIRECTION == Some(DataDirection::InitiatorToResponder) {
                    return Err(SerializationError::NoDataProvided);
                }
            },
        }

        Ok(Self { operation, data })
    }
}

/// I/O abstraction for MTP devices
///
/// Any implementation of this trait also acts as an [`Event`] stream.
///
/// See [`Device`](super::Device) for a higher-level interface for sending operations.
pub trait PtpIo: Send {
    /// Transport implementation-specific error type (e.g. `UsbError`)
    type TransportError: core::error::Error + Send;
    /// Implementation-specific errors that can occur during I/O operations
    type Error: core::error::Error + From<MtpError<Self::TransportError>> + Send;
    /// Implementation-specific event stream, see [`Self::event_stream()`]
    type EventStream: Stream<Item = Result<Event, Self::Error>> + Send + Unpin + 'static;

    /// Get the next transaction ID
    ///
    /// While not required, the resulting ID **should** be used in the next operation, to keep the
    /// transaction IDs monotonically increasing.
    #[must_use]
    fn next_transaction_id(&self) -> TransactionId;
    /// Get the next session ID
    ///
    /// While not required, the resulting ID **should** be used in the next operation, to keep the
    /// session IDs monotonically increasing.
    #[must_use]
    fn next_session_id(&self) -> SessionId;
    /// Get the endianness of the current device
    ///
    /// The value of this is expected to be consistent for the duration of the session.
    fn endian(&self) -> Endian;
    /// Get an instance of the responder -> initiator event stream
    ///
    /// This is used by the responder to send important updates (object modifications, property changes, etc.)
    /// at any time.
    fn event_stream(&self) -> Self::EventStream;

    /// Send the operation to the device and wait for a response
    fn send_operation<O>(
        &self,
        operation: OperationBundle<O>,
    ) -> impl Future<Output = Response<O, MtpError<Self::TransportError>>> + Send
    where
        O: DynOperation,
        for<'a> SerializedOperation<'a>: From<&'a O>;
}
