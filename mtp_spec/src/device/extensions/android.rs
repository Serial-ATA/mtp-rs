use crate::communication::SessionId;
use crate::communication::operation::android::{
    BeginEditObject, EndEditObject, GetPartialObject64, SendPartialObject, TruncateObject,
};
use crate::communication::response::Response;
use crate::device::PtpIo;
use crate::error::MtpError;
use crate::object::types::ObjectHandle;

/// Android-specific device operations
pub trait AndroidDevice: PtpIo {
    /// Send a [`GetPartialObject64`] operation
    fn get_partial_object_64(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
        offset: u64,
        size: u32,
    ) -> impl Future<
        Output = Response<GetPartialObject64, MtpError<<Self as PtpIo>::TransportError>>,
    > + Send {
        let high = (offset & 0xFFFFFFFF) as u32;
        let low = (offset >> 32) as u32;
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                GetPartialObject64::new(transaction_id, session_id, object, high, low, size),
                None,
            )
            .await
        }
    }

    /// Send a [`SendPartialObject`] operation
    fn send_partial_object(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
        offset: u64,
        size: u32,
        data: impl Into<Vec<u8>>,
    ) -> impl Future<Output = Response<SendPartialObject, MtpError<<Self as PtpIo>::TransportError>>>
    + Send {
        let offset_high = (offset & 0xFFFFFFFF) as u32;
        let offset_low = (offset >> 32) as u32;
        let data = data.into();
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                SendPartialObject::new(
                    transaction_id,
                    session_id,
                    object,
                    offset_high,
                    offset_low,
                    size,
                ),
                Some(data),
            )
            .await
        }
    }

    /// Send a [`TruncateObject`] operation
    fn truncate_object(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
        size: u64,
    ) -> impl Future<Output = Response<TruncateObject, MtpError<<Self as PtpIo>::TransportError>>> + Send
    {
        let high = (size & 0xFFFFFFFF) as u32;
        let low = (size >> 32) as u32;
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                TruncateObject::new(transaction_id, session_id, object, high, low),
                None,
            )
            .await
        }
    }

    /// Send a [`BeginEditObject`] operation
    fn begin_edit_object(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
    ) -> impl Future<Output = Response<BeginEditObject, MtpError<<Self as PtpIo>::TransportError>>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                BeginEditObject::new(transaction_id, session_id, object),
                None,
            )
            .await
        }
    }

    /// Send an [`EndEditObject`] operation
    fn end_edit_object(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
    ) -> impl Future<Output = Response<EndEditObject, MtpError<<Self as PtpIo>::TransportError>>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(EndEditObject::new(transaction_id, session_id, object), None)
                .await
        }
    }
}
