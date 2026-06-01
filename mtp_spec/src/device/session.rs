//! MTP session wrapper

use crate::communication::operation::{
    CloseSession, CopyObject, DeleteObject, FormatStore, GetDevicePropDesc, GetDevicePropValue,
    GetInterdependentPropDesc, GetNumObjects, GetObject, GetObjectHandles, GetObjectInfo,
    GetObjectPropDesc, GetObjectPropList, GetObjectPropValue, GetObjectPropsSupported,
    GetObjectReferences, GetPartialObject, GetStorageIDs, GetStorageInfo, GetThumb,
    InitiateCapture, InitiateOpenCapture, MoveObject, PowerDown, ResetDevice, ResetDevicePropValue,
    SelfTest, SelfTestType, SendObject, SendObjectInfo, SendObjectPropList, SetDevicePropValue,
    SetObjectPropList, SetObjectPropValue, SetObjectProtection, SetObjectReferences, Skip,
    TerminateOpenCapture,
};
use crate::communication::response::Response;
use crate::communication::response::errors::OperationError;
use crate::communication::{SessionId, TransactionId};
use crate::device::properties::{DeviceProperty, GetSet};
use crate::device::storage::{FilesystemType, StorageId};
use crate::device::{Device, OperationBundle, PtpIo, properties};
use crate::error::MtpError;
use crate::object::properties::{ObjectPropList, ObjectProperty, ObjectPropertyCode};
use crate::object::{
    Array, ObjectFormatCode, ObjectHandle, ObjectInfo, ProtectionStatus, PtpString,
};

use alloc::sync::Arc;
use core::ops::Deref;

use deku::DekuWriter;
use deku::no_std_io::Cursor;
use deku::prelude::Writer;

/// An active MTP session
///
/// NOTE: Operations that don't require an open session are available on [`Device`] directly.
///
/// This type is cheaply cloneable.
pub struct MtpSession<D> {
    id: SessionId,
    device: Arc<D>,
}

impl<D> Clone for MtpSession<D> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            device: Arc::clone(&self.device),
        }
    }
}

impl<D> MtpSession<D> {
    /// This session's ID
    pub fn id(&self) -> SessionId {
        self.id
    }
}

impl<D: Device> MtpSession<D> {
    /// Open a new MTP session on the given `device`
    ///
    /// NOTES:
    ///
    /// * The session ID will be automatically assigned based on the [`PtpIo::next_session_id()`] implementation.
    /// * If the operation fails with [`SessionAlreadyOpen`], the error will be ignored and the existing session ID will be used.
    ///
    /// [`SessionAlreadyOpen`]: crate::communication::response::errors::SessionAlreadyOpen
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::open_session()`].
    pub async fn open(device: D) -> Result<Self, MtpError<<D as PtpIo>::TransportError>> {
        match device.open_session().await {
            Ok((_res, session_id)) => Ok(Self {
                id: session_id,
                device: Arc::new(device),
            }),
            Err(MtpError::Protocol(OperationError::SessionAlreadyOpen(e))) => {
                tracing::warn!("Session {} already open", e.session_id);
                Ok(Self {
                    id: e.session_id,
                    device: Arc::new(device),
                })
            },
            Err(e) => Err(e),
        }
    }
}

impl<D> MtpSession<D>
where
    D: Device,
{
    // === Property checking ===

    /// Check the device's battery level
    ///
    /// This should always be `0..=100`, but the device could be doing something weird. The range of
    /// values can be verified by checking [`Device::get_device_prop_desc()`] with [`BatteryLevel`].
    ///
    /// [`BatteryLevel`]: properties::BatteryLevel
    ///
    /// # Errors
    ///
    /// See [`MtpSession::get_device_prop_value()`]
    pub async fn battery_level(&self) -> Result<u8, MtpError<<D as PtpIo>::TransportError>> {
        let prop = self
            .get_device_prop_value::<properties::BatteryLevel>()
            .await?;
        Ok(prop.data.data)
    }

    /// A human-readable description of the device
    ///
    /// # Errors
    ///
    /// See [`MtpSession::get_device_prop_value()`]
    pub async fn friendly_name(&self) -> Result<PtpString, MtpError<<D as PtpIo>::TransportError>> {
        let prop = self
            .get_device_prop_value::<properties::DeviceFriendlyName>()
            .await?;
        Ok(prop.data.data)
    }

    /// Whether the [`ObjectProperty`] `T` is writeable for the given `format`
    ///
    /// # Errors
    ///
    /// See [`MtpSession::get_object_prop_desc()`]
    pub async fn object_property_can_be_modified<T>(
        &self,
        format: ObjectFormatCode,
    ) -> Result<bool, MtpError<<D as PtpIo>::TransportError>>
    where
        T: ObjectProperty,
    {
        match self.get_object_prop_desc::<T>(format).await {
            Ok(desc) => Ok(desc.data.data.get_set() == GetSet::ReadWrite),
            Err(MtpError::Protocol(OperationError::ObjectPropNotSupported(_))) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Whether the [`DeviceProperty`] `T` is writeable for the current device
    ///
    /// # Errors
    ///
    /// See [`MtpSession::get_object_prop_desc()`]
    pub async fn device_property_can_be_modified<T>(
        &self,
    ) -> Result<bool, MtpError<<D as PtpIo>::TransportError>>
    where
        T: DeviceProperty,
    {
        let desc = self.get_device_prop_desc::<T>().await?;
        Ok(desc.data.data.get_set == GetSet::ReadWrite)
    }

    // === Operation wrappers ===

    /// Send a [`CloseSession`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn close_session(
        &self,
    ) -> Response<CloseSession, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            CloseSession::new(transaction_id, session_id),
            None,
        )?)
        .await
    }

    /// Send a [`GetStorageIDs`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_storage_ids(
        &self,
    ) -> Response<GetStorageIDs, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetStorageIDs::new(transaction_id, session_id),
            None,
        )?)
        .await
    }

    /// Send a [`GetStorageInfo`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_storage_info(
        &self,
        storage: StorageId,
    ) -> Response<GetStorageInfo, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetStorageInfo::new(transaction_id, session_id, storage),
            None,
        )?)
        .await
    }

    /// Send a [`GetNumObjects`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_num_objects(
        &self,
        storage: StorageId,
        format: Option<ObjectFormatCode>,
        parent: Option<ObjectHandle>,
    ) -> Response<GetNumObjects, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetNumObjects::new(transaction_id, session_id, storage, format, parent),
            None,
        )?)
        .await
    }

    /// Send a [`GetObjectHandles`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_object_handles(
        &self,
        storage: StorageId,
        format: Option<ObjectFormatCode>,
        parent: Option<ObjectHandle>,
    ) -> Response<GetObjectHandles, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetObjectHandles::new(transaction_id, session_id, storage, format, parent),
            None,
        )?)
        .await
    }

    /// Send a [`GetObjectInfo`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_object_info(
        &self,
        object: ObjectHandle,
    ) -> Response<GetObjectInfo, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetObjectInfo::new(transaction_id, session_id, object),
            None,
        )?)
        .await
    }

    /// Send a [`GetObject`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_object(
        &self,
        object: ObjectHandle,
    ) -> Response<GetObject, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetObject::new(transaction_id, session_id, object),
            None,
        )?)
        .await
    }

    /// Send a [`GetThumb`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_thumb(
        &self,
        object: ObjectHandle,
    ) -> Response<GetThumb, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetThumb::new(transaction_id, session_id, object),
            None,
        )?)
        .await
    }

    /// Send a [`DeleteObject`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn delete_object(
        &self,
        object: ObjectHandle,
        format: Option<ObjectFormatCode>,
    ) -> Response<DeleteObject, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            DeleteObject::new(transaction_id, session_id, object, format),
            None,
        )?)
        .await
    }

    /// Send a [`SendObjectInfo`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn send_object_info(
        &self,
        object_info: ObjectInfo,
    ) -> Response<SendObjectInfo, MtpError<<D as PtpIo>::TransportError>> {
        let storage = if object_info.storage_id == StorageId::DEFAULT_STORE {
            None
        } else {
            Some(object_info.storage_id)
        };
        let parent = object_info.parent_object;

        let mut encoded_object_info = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut encoded_object_info);
        object_info
            .to_writer(&mut writer, self.endian())
            .map_err(|e| MtpError::Serialization(e.into()))?;

        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            SendObjectInfo::new(transaction_id, session_id, storage, parent),
            Some(encoded_object_info.into_inner()),
        )?)
        .await
    }

    /// Send a [`SendObject`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn send_object<T>(
        &self,
        object_data: T,
    ) -> Response<SendObject, MtpError<<D as PtpIo>::TransportError>>
    where
        T: Into<Vec<u8>>,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            SendObject::new(transaction_id, session_id),
            Some(object_data.into()),
        )?)
        .await
    }

    /// Send a [`InitiateCapture`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn initiate_capture(
        &self,
        storage: Option<StorageId>,
        format: Option<ObjectFormatCode>,
    ) -> Response<InitiateCapture, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            InitiateCapture::new(transaction_id, session_id, storage, format),
            None,
        )?)
        .await
    }

    /// Send a [`FormatStore`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn format_store(
        &self,
        storage: StorageId,
        fs: Option<FilesystemType>,
    ) -> Response<FormatStore, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            FormatStore::new(transaction_id, session_id, storage, fs),
            None,
        )?)
        .await
    }

    /// Send a [`ResetDevice`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn reset_device(
        &self,
    ) -> Response<ResetDevice, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            ResetDevice::new(transaction_id, session_id),
            None,
        )?)
        .await
    }

    /// Send a [`SelfTest`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn self_test(
        &self,
        test_type: SelfTestType,
    ) -> Response<SelfTest, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            SelfTest::new(transaction_id, session_id, test_type),
            None,
        )?)
        .await
    }

    /// Send a [`SetObjectProtection`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn set_object_protection(
        &self,
        object: ObjectHandle,
        status: ProtectionStatus,
    ) -> Response<SetObjectProtection, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            SetObjectProtection::new(transaction_id, session_id, object, status),
            None,
        )?)
        .await
    }

    /// Send a [`PowerDown`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn power_down(&self) -> Response<PowerDown, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            PowerDown::new(transaction_id, session_id),
            None,
        )?)
        .await
    }

    /// Send a [`GetDevicePropDesc`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_device_prop_desc<T>(
        &self,
    ) -> Response<GetDevicePropDesc<T>, MtpError<<D as PtpIo>::TransportError>>
    where
        T: DeviceProperty,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetDevicePropDesc::<T>::new(transaction_id, session_id),
            None,
        )?)
        .await
    }

    /// Send a [`GetDevicePropValue`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_device_prop_value<T>(
        &self,
    ) -> Response<GetDevicePropValue<T>, MtpError<<D as PtpIo>::TransportError>>
    where
        T: DeviceProperty,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetDevicePropValue::<T>::new(transaction_id, session_id),
            None,
        )?)
        .await
    }

    /// Send a [`SetDevicePropValue`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn set_device_prop_value<T>(
        &self,
        value: Vec<u8>,
    ) -> Response<SetDevicePropValue<T>, MtpError<<D as PtpIo>::TransportError>>
    where
        T: DeviceProperty,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            SetDevicePropValue::<T>::new(transaction_id, session_id),
            Some(value),
        )?)
        .await
    }

    /// Send a [`ResetDevicePropValue`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn reset_device_prop_value<T>(
        &self,
    ) -> Response<ResetDevicePropValue<T>, MtpError<<D as PtpIo>::TransportError>>
    where
        T: DeviceProperty,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            ResetDevicePropValue::<T>::new(transaction_id, session_id),
            None,
        )?)
        .await
    }

    /// Send a [`TerminateOpenCapture`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn terminate_open_capture(
        &self,
        transaction_id: TransactionId,
    ) -> Response<TerminateOpenCapture, MtpError<<D as PtpIo>::TransportError>> {
        let next_transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            TerminateOpenCapture::new(next_transaction_id, session_id, transaction_id),
            None,
        )?)
        .await
    }

    /// Send a [`MoveObject`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn move_object(
        &self,
        object: ObjectHandle,
        storage: StorageId,
        parent: Option<ObjectHandle>,
    ) -> Response<MoveObject, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            MoveObject::new(transaction_id, session_id, object, storage, parent),
            None,
        )?)
        .await
    }

    /// Send a [`CopyObject`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn copy_object(
        &self,
        object: ObjectHandle,
        storage: StorageId,
        parent: Option<ObjectHandle>,
    ) -> Response<CopyObject, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            CopyObject::new(transaction_id, session_id, object, storage, parent),
            None,
        )?)
        .await
    }

    /// Send a [`GetPartialObject`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_partial_object(
        &self,
        object: ObjectHandle,
        offset: u32,
        len: u32,
    ) -> Response<GetPartialObject, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetPartialObject::new(transaction_id, session_id, object, offset, len),
            None,
        )?)
        .await
    }

    /// Send an [`InitiateOpenCapture`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn initiate_open_capture(
        &self,
        storage: Option<StorageId>,
        format: Option<ObjectFormatCode>,
    ) -> Response<InitiateOpenCapture, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            InitiateOpenCapture::new(transaction_id, session_id, storage, format),
            None,
        )?)
        .await
    }

    /// Send a [`GetObjectPropsSupported`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_object_props_supported(
        &self,
        format: ObjectFormatCode,
    ) -> Response<GetObjectPropsSupported, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetObjectPropsSupported::new(transaction_id, session_id, format),
            None,
        )?)
        .await
    }

    /// Send a [`GetObjectPropDesc`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_object_prop_desc<T>(
        &self,
        format: ObjectFormatCode,
    ) -> Response<GetObjectPropDesc<T>, MtpError<<D as PtpIo>::TransportError>>
    where
        T: ObjectProperty,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetObjectPropDesc::<T>::new(transaction_id, session_id, format),
            None,
        )?)
        .await
    }

    /// Send a [`GetObjectPropValue`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_object_prop_value<T>(
        &self,
        object: ObjectHandle,
    ) -> Response<GetObjectPropValue<T>, MtpError<<D as PtpIo>::TransportError>>
    where
        T: ObjectProperty,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetObjectPropValue::<T>::new(transaction_id, session_id, object),
            None,
        )?)
        .await
    }

    /// Send a [`SetObjectPropValue`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn set_object_prop_value<T>(
        &self,
        object: ObjectHandle,
        value: T::DataType,
    ) -> Response<SetObjectPropValue<T>, MtpError<<D as PtpIo>::TransportError>>
    where
        T: ObjectProperty,
    {
        let mut encoded_value = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut encoded_value);
        value
            .to_writer(&mut writer, self.endian())
            .map_err(|e| MtpError::Serialization(e.into()))?;

        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            SetObjectPropValue::<T>::new(transaction_id, session_id, object),
            Some(encoded_value.into_inner()),
        )?)
        .await
    }

    /// Send a [`GetObjectReferences`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_object_references(
        &self,
        object: ObjectHandle,
    ) -> Response<GetObjectReferences, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetObjectReferences::new(transaction_id, session_id, object),
            None,
        )?)
        .await
    }

    /// Send a [`SetObjectReferences`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn set_object_references(
        &self,
        object: ObjectHandle,
        references: Array<ObjectHandle>,
    ) -> Response<SetObjectReferences, MtpError<<D as PtpIo>::TransportError>> {
        let mut encoded_references = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut encoded_references);
        references
            .to_writer(&mut writer, self.endian())
            .map_err(|e| MtpError::Serialization(e.into()))?;

        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            SetObjectReferences::new(transaction_id, session_id, object),
            Some(encoded_references.into_inner()),
        )?)
        .await
    }

    /// Send a [`Skip`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn skip(&self, skip: u32) -> Response<Skip, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            Skip::new(transaction_id, session_id, skip),
            None,
        )?)
        .await
    }

    // == Enhanced Operations ==
    //
    // Defined in Appendix E

    /// Send a [`GetObjectPropList`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_object_prop_list(
        &self,
        object: ObjectHandle,
        format: Option<ObjectFormatCode>,
        property: Option<ObjectPropertyCode>,
        group: Option<u32>,
        depth: Option<u32>,
    ) -> Response<GetObjectPropList, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
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
        )?)
        .await
    }

    /// Send a [`SetObjectPropList`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn set_object_prop_list(
        &self,
        props: ObjectPropList,
    ) -> Response<SetObjectPropList, MtpError<<D as PtpIo>::TransportError>> {
        let mut object_prop_list = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut object_prop_list);
        props.to_writer(&mut writer, self.endian())?;

        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            SetObjectPropList::new(transaction_id, session_id),
            Some(object_prop_list.into_inner()),
        )?)
        .await
    }

    /// Send a [`GetInterdependentPropDesc`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn get_interdependent_prop_desc(
        &self,
        format: ObjectFormatCode,
    ) -> Response<GetInterdependentPropDesc, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
            GetInterdependentPropDesc::new(transaction_id, session_id, format),
            None,
        )?)
        .await
    }

    /// Send a [`SendObjectPropList`] operation
    ///
    /// # Errors
    ///
    /// Depends on the [`Device`], see the implementation of [`Device::send_operation()`].
    pub async fn send_object_prop_list(
        &self,
        destination: Option<StorageId>,
        parent: Option<ObjectHandle>,
        format: ObjectFormatCode,
        size: u64,
        properties: ObjectPropList,
    ) -> Response<SendObjectPropList, MtpError<<D as PtpIo>::TransportError>> {
        let mut object_prop_list = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut object_prop_list);
        properties.to_writer(&mut writer, self.endian())?;

        let high = (size >> 32) as u32;
        let low = size as u32;

        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(OperationBundle::new(
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
        )?)
        .await
    }
}

impl<D> Deref for MtpSession<D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}
