use crate::communication::event::Event;
use crate::communication::operation::{DataDirection, DynOperation, SerializedOperation};
use crate::communication::response::Response;
use crate::communication::{SessionId, TransactionId};
use crate::error::MtpError;

use alloc::vec::Vec;

use deku::ctx::Endian;
use futures_core::Stream;

/// I/O abstraction for MTP devices
///
/// Any implementation of this trait also acts as an [`Event`] stream.
///
/// See [`Device`](super::Device) for a higher-level interface for sending operations.
pub trait PtpIo: Send {
    /// Implementation-specific errors that can occur during I/O operations
    type Error: core::error::Error + From<MtpError>;
    type EventStream: Stream<Item = Result<Event, Self::Error>>;

    /// Get the next transaction ID
    ///
    /// While not required, the resulting ID **should** be used in the next operation, to keep the
    /// transaction IDs monotonically increasing.
    #[must_use]
    fn next_transaction_id(&mut self) -> TransactionId;
    /// Get the next session ID
    ///
    /// While not required, the resulting ID **should** be used in the next operation, to keep the
    /// session IDs monotonically increasing.
    #[must_use]
    fn next_session_id(&mut self) -> SessionId;
    /// Get the endianness of the current device
    ///
    /// The value of this is expected to be consistent for the duration of the session.
    fn endian(&self) -> Endian;
    fn event_stream(&self) -> Self::EventStream;

    /// Send the operation to the device and wait for a response
    fn send_operation<O>(
        &mut self,
        operation: O,
        data: Option<Vec<u8>>,
    ) -> impl Future<Output = Result<Response<O>, Self::Error>> + Send
    where
        O: DynOperation,
        for<'a> SerializedOperation<'a>: From<&'a O>,
    {
        async move {
            match &data {
                Some(_) => match O::DATA_DIRECTION {
                    Some(DataDirection::ResponderToInitiator) => {
                        return Err(MtpError::WrongDataDirection.into());
                    },
                    None => {
                        return Err(MtpError::UnexpectedDataProvided.into());
                    },
                    _ => {},
                },
                None => {
                    if O::DATA_DIRECTION == Some(DataDirection::InitiatorToResponder) {
                        return Err(MtpError::NoDataProvided.into());
                    }
                },
            }

            self.__send_operation(operation, data).await
        }
    }

    /// The actual format-specific operation sending logic
    ///
    /// This should not be used directly, it is only exposed for use in implementations.
    /// Validation is handled by [`PtpIo::send_operation`], skipping it **will** result in panics.
    fn __send_operation<O>(
        &mut self,
        operation: O,
        data: Option<Vec<u8>>,
    ) -> impl Future<Output = Result<Response<O>, Self::Error>> + Send
    where
        O: DynOperation,
        for<'a> SerializedOperation<'a>: From<&'a O>;
}
