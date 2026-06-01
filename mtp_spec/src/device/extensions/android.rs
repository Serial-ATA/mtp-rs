//! Android-specific device extensions

use crate::communication::operation::android::{
    BeginEditObject, EndEditObject, GetPartialObject64, SendPartialObject, TruncateObject,
};
use crate::communication::response::Response;
use crate::device::session::MtpSession;
use crate::device::{Device, OperationBundle, PtpIo};
use crate::error::MtpError;
use crate::object::ObjectHandle;

/// Android-specific device operations
pub trait AndroidDevice<D>
where
    D: Device,
{
    /// Send a [`GetPartialObject64`] operation
    fn get_partial_object_64(
        &self,
        object: ObjectHandle,
        offset: u64,
        size: u32,
    ) -> impl Future<Output = Response<GetPartialObject64, MtpError<<D as PtpIo>::TransportError>>> + Send;

    /// Send a [`SendPartialObject`] operation
    fn send_partial_object(
        &self,
        object: ObjectHandle,
        offset: u64,
        size: u32,
        data: impl Into<Vec<u8>> + Send,
    ) -> impl Future<Output = Response<SendPartialObject, MtpError<<D as PtpIo>::TransportError>>> + Send;

    /// Send a [`TruncateObject`] operation
    fn truncate_object(
        &self,
        object: ObjectHandle,
        size: u64,
    ) -> impl Future<Output = Response<TruncateObject, MtpError<<D as PtpIo>::TransportError>>> + Send;

    /// Send a [`BeginEditObject`] operation
    fn begin_edit_object(
        &self,
        object: ObjectHandle,
    ) -> impl Future<Output = Response<BeginEditObject, MtpError<<D as PtpIo>::TransportError>>> + Send;

    /// Send an [`EndEditObject`] operation
    fn end_edit_object(
        &self,
        object: ObjectHandle,
    ) -> impl Future<Output = Response<EndEditObject, MtpError<<D as PtpIo>::TransportError>>> + Send;
}

impl<D> AndroidDevice<D> for MtpSession<D>
where
    D: Device,
{
    /// Send a [`GetPartialObject64`] operation
    async fn get_partial_object_64(
        &self,
        object: ObjectHandle,
        offset: u64,
        size: u32,
    ) -> Response<GetPartialObject64, MtpError<<D as PtpIo>::TransportError>> {
        let high = (offset & 0xFFFF_FFFF) as u32;
        let low = (offset >> 32) as u32;
        let transaction_id = self.next_transaction_id();
        let session_id = self.id();
        self.send_operation(OperationBundle::new(
            GetPartialObject64::new(transaction_id, session_id, object, high, low, size),
            None,
        )?)
        .await
    }

    /// Send a [`SendPartialObject`] operation
    async fn send_partial_object(
        &self,
        object: ObjectHandle,
        offset: u64,
        size: u32,
        data: impl Into<Vec<u8>> + Send,
    ) -> Response<SendPartialObject, MtpError<<D as PtpIo>::TransportError>> {
        let offset_high = (offset & 0xFFFF_FFFF) as u32;
        let offset_low = (offset >> 32) as u32;
        let data = data.into();
        let transaction_id = self.next_transaction_id();
        let session_id = self.id();
        self.send_operation(OperationBundle::new(
            SendPartialObject::new(
                transaction_id,
                session_id,
                object,
                offset_high,
                offset_low,
                size,
            ),
            Some(data),
        )?)
        .await
    }

    /// Send a [`TruncateObject`] operation
    async fn truncate_object(
        &self,
        object: ObjectHandle,
        size: u64,
    ) -> Response<TruncateObject, MtpError<<D as PtpIo>::TransportError>> {
        let high = (size & 0xFFFF_FFFF) as u32;
        let low = (size >> 32) as u32;
        let transaction_id = self.next_transaction_id();
        let session_id = self.id();
        self.send_operation(OperationBundle::new(
            TruncateObject::new(transaction_id, session_id, object, high, low),
            None,
        )?)
        .await
    }

    /// Send a [`BeginEditObject`] operation
    async fn begin_edit_object(
        &self,
        object: ObjectHandle,
    ) -> Response<BeginEditObject, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id();
        self.send_operation(OperationBundle::new(
            BeginEditObject::new(transaction_id, session_id, object),
            None,
        )?)
        .await
    }

    async fn end_edit_object(
        &self,
        object: ObjectHandle,
    ) -> Response<EndEditObject, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id();
        self.send_operation(OperationBundle::new(
            EndEditObject::new(transaction_id, session_id, object),
            None,
        )?)
        .await
    }
}
