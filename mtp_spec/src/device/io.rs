use crate::communication::event::Event;
use crate::communication::operation::{DataDirection, DynOperation, SerializedOperation};
use crate::communication::response::Response;
use crate::communication::{SessionId, TransactionId};

use alloc::vec::Vec;

use futures_core::Stream;

/// I/O abstraction for MTP devices
///
/// Any implementation of this trait also acts as an [`Event`] stream.
///
/// See [`Device`](super::Device) for a higher-level interface for sending operations.
pub trait PtpIo: Stream<Item = Result<Event, Self::Error>> {
    /// Implementation-specific errors that can occur during I/O operations
    type Error: core::error::Error;

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

    /// Send the operation to the device and wait for a response
    fn send_operation<O>(
        &mut self,
        operation: O,
        data: Option<Vec<u8>>,
    ) -> impl Future<Output = Result<Response<O>, Self::Error>>
    where
        O: DynOperation,
        for<'a> SerializedOperation<'a>: From<&'a O>,
    {
        async move {
            match &data {
                Some(_) => match O::DATA_DIRECTION {
                    Some(DataDirection::ResponderToInitiator) => {
                        todo!("error, wrong data direction")
                    },
                    None => {
                        todo!("error, provided data when not expected")
                    },
                    _ => {},
                },
                None => {
                    if O::DATA_DIRECTION == Some(DataDirection::InitiatorToResponder) {
                        todo!("error, missing data when expected")
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
    ) -> impl Future<Output = Result<Response<O>, Self::Error>>
    where
        O: DynOperation,
        for<'a> SerializedOperation<'a>: From<&'a O>;
}
