//! Abstractions over MTP responder devices
//!
//! This module contains two key traits:
//!
//! * [`Device`]: Convenience trait providing simple methods for sending [`operations`](crate::communication::operation)
//! * [`PtpIo`]: The backing I/O interface used by [`Device`], implemented by higher-level crates
//!   providing transport implementations

use crate::communication::operation::{
    CloseSession, CopyObject, DeleteObject, DevicePropCode, FormatStore, GetDeviceInfo,
    GetDevicePropDesc, GetDevicePropValue, GetInterdependentPropDesc, GetNumObjects, GetObject,
    GetObjectHandles, GetObjectInfo, GetObjectPropDesc, GetObjectPropList, GetObjectPropValue,
    GetObjectPropsSupported, GetObjectReferences, GetPartialObject, GetStorageIDs, GetStorageInfo,
    GetThumb, InitiateCapture, InitiateOpenCapture, MoveObject, OpenSession, PowerDown,
    ResetDevice, ResetDevicePropValue, SelfTest, SelfTestType, SendObject, SendObjectInfo,
    SendObjectPropList, SetDevicePropValue, SetObjectPropList, SetObjectPropValue,
    SetObjectProtection, SetObjectReferences, Skip, TerminateOpenCapture,
};
use crate::communication::response::Response;
use crate::communication::{SessionId, TransactionId};
use crate::device::storage::id::StorageId;
use crate::device::storage::info::FilesystemType;
use crate::object::info::{ObjectInfo, ProtectionStatus};
use crate::object::types::properties::{ObjectProperty, ObjectPropertyCode, SerializeableProperty};
use crate::object::types::{Array, ObjectFormatCode, ObjectHandle};
use alloc::boxed::Box;

use alloc::vec::Vec;
use deku::no_std_io::Cursor;
use deku::writer::Writer;
use deku::{DekuContainerWrite, DekuWriter};

pub mod info;
pub mod property_describing;
pub mod storage;

mod io;
pub use io::*;

/// An MTP responder device
///
/// This is a convenience trait to perform operations on a responder without having to interact with
/// [`operations`] directly.
///
/// [`operations`]: crate::communication::operation
pub trait Device: PtpIo {
    /// Whether the [`ObjectProperty`] `T` is writeable for the given `format`
    fn property_can_be_modified<T>(
        &mut self,
        session_id: SessionId,
        format: ObjectFormatCode,
    ) -> impl Future<Output = Result<bool, <Self as PtpIo>::Error>>
    where
        T: ObjectProperty,
    {
        async move {
            let _desc = self
                .get_object_prop_desc::<T>(session_id, format)
                .await?
                .map_err(Into::<crate::error::MtpError>::into)?;
            // TODO: actually check the GetSet field

            Ok(true)
        }
    }

    /// Send a [`GetDeviceInfo`] operation
    fn get_device_info(
        &mut self,
        session_id: Option<SessionId>,
    ) -> impl Future<Output = Result<Response<GetDeviceInfo>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = if session_id.is_some() {
                self.next_transaction_id()
            } else {
                TransactionId::NONE
            };

            self.send_operation(
                GetDeviceInfo::new(transaction_id, session_id.unwrap_or(SessionId::NONE)),
                None,
            )
            .await
        }
    }

    /// Send a [`OpenSession`] operation
    fn open_session(
        &mut self,
    ) -> impl Future<Output = Result<(Response<OpenSession>, SessionId), <Self as PtpIo>::Error>>
    {
        async move {
            let transaction_id = self.next_transaction_id();
            let session_id = self.next_session_id();
            self.send_operation(OpenSession::new(transaction_id, session_id), None)
                .await
                .map(|res| (res, session_id))
        }
    }

    /// Send a [`CloseSession`] operation
    fn close_session(
        &mut self,
        session_id: SessionId,
    ) -> impl Future<Output = Result<Response<CloseSession>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(CloseSession::new(transaction_id, session_id), None)
                .await
        }
    }

    /// Send a [`GetStorageIDs`] operation
    fn get_storage_ids(
        &mut self,
        session_id: SessionId,
    ) -> impl Future<Output = Result<Response<GetStorageIDs>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(GetStorageIDs::new(transaction_id, session_id), None)
                .await
        }
    }

    /// Send a [`GetStorageInfo`] operation
    fn get_storage_info(
        &mut self,
        session_id: SessionId,
        storage: StorageId,
    ) -> impl Future<Output = Result<Response<GetStorageInfo>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                GetStorageInfo::new(transaction_id, session_id, storage),
                None,
            )
            .await
        }
    }

    /// Send a [`GetNumObjects`] operation
    fn get_num_objects(
        &mut self,
        session_id: SessionId,
        storage: StorageId,
        format: Option<ObjectFormatCode>,
        parent: Option<ObjectHandle>,
    ) -> impl Future<Output = Result<Response<GetNumObjects>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                GetNumObjects::new(transaction_id, session_id, storage, format, parent),
                None,
            )
            .await
        }
    }

    /// Send a [`GetObjectHandles`] operation
    fn get_object_handles(
        &mut self,
        session_id: SessionId,
        storage: StorageId,
        format: Option<ObjectFormatCode>,
        parent: Option<ObjectHandle>,
    ) -> impl Future<Output = Result<Response<GetObjectHandles>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                GetObjectHandles::new(transaction_id, session_id, storage, format, parent),
                None,
            )
            .await
        }
    }

    /// Send a [`GetObjectInfo`] operation
    fn get_object_info(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
    ) -> impl Future<Output = Result<Response<GetObjectInfo>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(GetObjectInfo::new(transaction_id, session_id, object), None)
                .await
        }
    }

    /// Send a [`GetObject`] operation
    fn get_object(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
    ) -> impl Future<Output = Result<Response<GetObject>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(GetObject::new(transaction_id, session_id, object), None)
                .await
        }
    }

    /// Send a [`GetThumb`] operation
    fn get_thumb(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
    ) -> impl Future<Output = Result<Response<GetThumb>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(GetThumb::new(transaction_id, session_id, object), None)
                .await
        }
    }

    /// Send a [`DeleteObject`] operation
    fn delete_object(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
        format: Option<ObjectFormatCode>,
    ) -> impl Future<Output = Result<Response<DeleteObject>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                DeleteObject::new(
                    transaction_id,
                    session_id,
                    object,
                    format.unwrap_or(ObjectFormatCode::Unknown(0)),
                ),
                None,
            )
            .await
        }
    }

    /// Send a [`SendObjectInfo`] operation
    fn send_object_info(
        &mut self,
        session_id: SessionId,
        storage: Option<StorageId>,
        parent: Option<ObjectHandle>,
        object_info: ObjectInfo,
    ) -> impl Future<Output = Result<Response<SendObjectInfo>, <Self as PtpIo>::Error>> + Send {
        async move {
            let encoded_object_info = object_info
                .to_bytes()
                .map_err(Into::<crate::error::MtpError>::into)?;

            let transaction_id = self.next_transaction_id();
            self.send_operation(
                SendObjectInfo::new(transaction_id, session_id, storage, parent),
                Some(encoded_object_info),
            )
            .await
        }
    }

    /// Send a [`SendObject`] operation
    fn send_object<T>(
        &mut self,
        session_id: SessionId,
        object_data: T,
    ) -> impl Future<Output = Result<Response<SendObject>, <Self as PtpIo>::Error>>
    where
        T: Into<Vec<u8>>,
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                SendObject::new(transaction_id, session_id),
                Some(object_data.into()),
            )
            .await
        }
    }

    /// Send a [`InitiateCapture`] operation
    fn initiate_capture(
        &mut self,
        session_id: SessionId,
        storage: Option<StorageId>,
        format: Option<ObjectFormatCode>,
    ) -> impl Future<Output = Result<Response<InitiateCapture>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                InitiateCapture::new(transaction_id, session_id, storage, format),
                None,
            )
            .await
        }
    }

    /// Send a [`FormatStore`] operation
    fn format_store(
        &mut self,
        session_id: SessionId,
        storage: StorageId,
        fs: FilesystemType,
    ) -> impl Future<Output = Result<Response<FormatStore>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                FormatStore::new(transaction_id, session_id, storage, fs),
                None,
            )
            .await
        }
    }

    /// Send a [`ResetDevice`] operation
    fn reset_device(
        &mut self,
        session_id: SessionId,
    ) -> impl Future<Output = Result<Response<ResetDevice>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(ResetDevice::new(transaction_id, session_id), None)
                .await
        }
    }

    /// Send a [`SelfTest`] operation
    fn self_test(
        &mut self,
        session_id: SessionId,
        test_type: SelfTestType,
    ) -> impl Future<Output = Result<Response<SelfTest>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(SelfTest::new(transaction_id, session_id, test_type), None)
                .await
        }
    }

    /// Send a [`SetObjectProtection`] operation
    fn set_object_protection(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
        status: ProtectionStatus,
    ) -> impl Future<Output = Result<Response<SetObjectProtection>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                SetObjectProtection::new(transaction_id, session_id, object, status),
                None,
            )
            .await
        }
    }

    /// Send a [`PowerDown`] operation
    fn power_down(
        &mut self,
        session_id: SessionId,
    ) -> impl Future<Output = Result<Response<PowerDown>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(PowerDown::new(transaction_id, session_id), None)
                .await
        }
    }

    /// Send a [`GetDevicePropDesc`] operation
    fn get_device_prop_desc(
        &mut self,
        session_id: SessionId,
        code: DevicePropCode,
    ) -> impl Future<Output = Result<Response<GetDevicePropDesc>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                GetDevicePropDesc::new(transaction_id, session_id, code),
                None,
            )
            .await
        }
    }

    /// Send a [`GetDevicePropValue`] operation
    fn get_device_prop_value(
        &mut self,
        session_id: SessionId,
        code: DevicePropCode,
    ) -> impl Future<Output = Result<Response<GetDevicePropValue>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                GetDevicePropValue::new(transaction_id, session_id, code),
                None,
            )
            .await
        }
    }

    /// Send a [`SetDevicePropValue`] operation
    fn set_device_prop_value(
        &mut self,
        session_id: SessionId,
        code: DevicePropCode,
        value: Vec<u8>,
    ) -> impl Future<Output = Result<Response<SetDevicePropValue>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                SetDevicePropValue::new(transaction_id, session_id, code),
                Some(value),
            )
            .await
        }
    }

    /// Send a [`ResetDevicePropValue`] operation
    fn reset_device_prop_value(
        &mut self,
        session_id: SessionId,
        code: DevicePropCode,
    ) -> impl Future<Output = Result<Response<ResetDevicePropValue>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                ResetDevicePropValue::new(transaction_id, session_id, code),
                None,
            )
            .await
        }
    }

    /// Send a [`TerminateOpenCapture`] operation
    fn terminate_open_capture(
        &mut self,
        session_id: SessionId,
        transaction_id: TransactionId,
    ) -> impl Future<Output = Result<Response<TerminateOpenCapture>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let next_transaction_id = self.next_transaction_id();
            self.send_operation(
                TerminateOpenCapture::new(next_transaction_id, session_id, transaction_id),
                None,
            )
            .await
        }
    }

    /// Send a [`MoveObject`] operation
    fn move_object(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
        storage: StorageId,
        parent: Option<ObjectHandle>,
    ) -> impl Future<Output = Result<Response<MoveObject>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                MoveObject::new(transaction_id, session_id, object, storage, parent),
                None,
            )
            .await
        }
    }

    /// Send a [`CopyObject`] operation
    fn copy_object(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
        storage: StorageId,
        parent: Option<ObjectHandle>,
    ) -> impl Future<Output = Result<Response<CopyObject>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                CopyObject::new(transaction_id, session_id, object, storage, parent),
                None,
            )
            .await
        }
    }

    /// Send a [`GetPartialObject`] operation
    fn get_partial_object(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
        offset: u32,
        len: u32,
    ) -> impl Future<Output = Result<Response<GetPartialObject>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                GetPartialObject::new(transaction_id, session_id, object, offset, len),
                None,
            )
            .await
        }
    }

    /// Send an [`InitiateOpenCapture`] operation
    fn initiate_open_capture(
        &mut self,
        session_id: SessionId,
        storage: Option<StorageId>,
        format: Option<ObjectFormatCode>,
    ) -> impl Future<Output = Result<Response<InitiateOpenCapture>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                InitiateOpenCapture::new(transaction_id, session_id, storage, format),
                None,
            )
            .await
        }
    }

    /// Send a [`GetObjectPropsSupported`] operation
    fn get_object_props_supported(
        &mut self,
        session_id: SessionId,
        format: ObjectFormatCode,
    ) -> impl Future<Output = Result<Response<GetObjectPropsSupported>, <Self as PtpIo>::Error>>
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                GetObjectPropsSupported::new(transaction_id, session_id, format),
                None,
            )
            .await
        }
    }

    /// Send a [`GetObjectPropDesc`] operation
    fn get_object_prop_desc<T>(
        &mut self,
        session_id: SessionId,
        format: ObjectFormatCode,
    ) -> impl Future<Output = Result<Response<GetObjectPropDesc<T>>, <Self as PtpIo>::Error>>
    where
        T: ObjectProperty,
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                GetObjectPropDesc::<T>::new(transaction_id, session_id, format),
                None,
            )
            .await
        }
    }

    /// Send a [`GetObjectPropValue`] operation
    fn get_object_prop_value<T>(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
    ) -> impl Future<Output = Result<Response<GetObjectPropValue<T>>, <Self as PtpIo>::Error>>
    where
        T: ObjectProperty,
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                GetObjectPropValue::<T>::new(transaction_id, session_id, object),
                None,
            )
            .await
        }
    }

    /// Send a [`SetObjectPropValue`] operation
    fn set_object_prop_value<T>(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
        value: T::DataType,
    ) -> impl Future<Output = Result<Response<SetObjectPropValue<T>>, <Self as PtpIo>::Error>>
    where
        T: ObjectProperty,
    {
        async move {
            let mut encoded_value = Cursor::new(Vec::new());
            let mut writer = Writer::new(&mut encoded_value);
            value
                .to_writer(&mut writer, self.endian())
                .map_err(Into::<crate::error::MtpError>::into)?;

            let transaction_id = self.next_transaction_id();
            self.send_operation(
                SetObjectPropValue::<T>::new(transaction_id, session_id, object),
                Some(encoded_value.into_inner()),
            )
            .await
        }
    }

    /// Send a [`GetObjectReferences`] operation
    fn get_object_references(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
    ) -> impl Future<Output = Result<Response<GetObjectReferences>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                GetObjectReferences::new(transaction_id, session_id, object),
                None,
            )
            .await
        }
    }

    /// Send a [`SetObjectReferences`] operation
    fn set_object_references(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
        references: Array<ObjectHandle>,
    ) -> impl Future<Output = Result<Response<SetObjectReferences>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let encoded_references = references
                .to_bytes()
                .map_err(Into::<crate::error::MtpError>::into)?;

            let transaction_id = self.next_transaction_id();
            self.send_operation(
                SetObjectReferences::new(transaction_id, session_id, object),
                Some(encoded_references),
            )
            .await
        }
    }

    /// Send a [`Skip`] operation
    fn skip(
        &mut self,
        session_id: SessionId,
        skip: u32,
    ) -> impl Future<Output = Result<Response<Skip>, <Self as PtpIo>::Error>> + Send {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(Skip::new(transaction_id, session_id, skip), None)
                .await
        }
    }

    // == Enhanced Operations ==
    //
    // Defined in Appendix E

    /// Send a [`GetObjectPropList`] operation
    fn get_object_prop_list(
        &mut self,
        session_id: SessionId,
        object: ObjectHandle,
        format: Option<ObjectFormatCode>,
        property: ObjectPropertyCode,
        group: Option<u32>,
        depth: Option<u32>,
    ) -> impl Future<Output = Result<Response<GetObjectPropList>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                GetObjectPropList::new(
                    transaction_id,
                    session_id,
                    object,
                    format,
                    property,
                    group.unwrap_or(0),
                    depth.unwrap_or(0),
                ),
                None,
            )
            .await
        }
    }

    /// Send a [`SetObjectPropList`] operation
    fn set_object_prop_list(
        &mut self,
        session_id: SessionId,
        props: Vec<u8>, // TODO: Actually define the ObjectPropList
    ) -> impl Future<Output = Result<Response<SetObjectPropList>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                SetObjectPropList::new(transaction_id, session_id),
                Some(props),
            )
            .await
        }
    }

    /// Send a [`GetInterdependentPropDesc`] operation
    fn get_interdependent_prop_desc(
        &mut self,
        session_id: SessionId,
        format: ObjectFormatCode,
    ) -> impl Future<Output = Result<Response<GetInterdependentPropDesc>, <Self as PtpIo>::Error>>
    {
        async move {
            let transaction_id = self.next_transaction_id();
            self.send_operation(
                GetInterdependentPropDesc::new(transaction_id, session_id, format),
                None,
            )
            .await
        }
    }

    /// Send a [`SendObjectPropList`] operation
    fn send_object_prop_list<'a>(
        &mut self,
        session_id: SessionId,
        destination: Option<StorageId>,
        parent: Option<ObjectHandle>,
        format: ObjectFormatCode,
        size: u64,
        properties: impl IntoIterator<Item = Box<dyn SerializeableProperty>> + Send,
    ) -> impl Future<Output = Result<Response<SendObjectPropList>, <Self as PtpIo>::Error>> + Send
    {
        async move {
            let mut object_prop_list = Cursor::new(Vec::new());

            let mut writer = Writer::new(&mut object_prop_list);
            for property in properties
                .into_iter()
                .map(|p| p.serialize(ObjectHandle::NONE))
            {
                property
                    .to_writer(&mut writer, self.endian())
                    .map_err(Into::<crate::error::MtpError>::into)?;
            }

            let high = (size >> 32) as u32;
            let low = size as u32;

            let transaction_id = self.next_transaction_id();
            self.send_operation(
                SendObjectPropList::new(
                    transaction_id,
                    session_id,
                    destination,
                    parent,
                    format,
                    high,
                    low,
                ),
                Some(object_prop_list.into_inner()),
            )
            .await
        }
    }
}
