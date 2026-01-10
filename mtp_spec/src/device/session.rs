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
use crate::device::storage::id::StorageId;
use crate::device::storage::info::FilesystemType;
use crate::device::{Device, PtpIo, properties};
use crate::error::MtpError;
use crate::object::info::{ObjectInfo, ProtectionStatus};
use crate::object::types::properties::{ObjectPropList, ObjectProperty, ObjectPropertyCode};
use crate::object::types::{Array, ObjectFormatCode, ObjectHandle, PtpString};

use std::ops::{Deref, DerefMut};

use deku::DekuWriter;
use deku::no_std_io::Cursor;
use deku::prelude::Writer;

// TODO: Could use an example
/// An active MTP session
///
/// NOTE: Operations that don't require an open session are available on [`Device`] directly.
pub struct MtpSession<D> {
    id: SessionId,
    device: D,
}

// TODO: Could use an example
impl<D> MtpSession<D> {
    /// This session's ID
    pub fn id(&self) -> SessionId {
        self.id
    }
}

impl<D: Device> MtpSession<D> {
    // TODO: Could use an example
    /// Open a new MTP session on the given `device`
    ///
    /// NOTES:
    ///
    /// * The session ID will be automatically assigned based on the [`PtpIo::next_session_id()`] implementation.
    /// * If the operation fails with [`SessionAlreadyOpen`], the error will be ignored and the existing session ID will be used.
    ///
    /// [`SessionAlreadyOpen`]: crate::communication::response::errors::SessionAlreadyOpen
    pub async fn open(mut device: D) -> Result<Self, MtpError<<D as PtpIo>::TransportError>> {
        match device.open_session().await {
            Ok((_res, session_id)) => Ok(Self {
                id: session_id,
                device,
            }),
            Err(MtpError::Protocol(OperationError::SessionAlreadyOpen(e))) => {
                log::warn!("Session {} already open", e.session_id);
                Ok(Self {
                    id: e.session_id,
                    device,
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
    pub async fn battery_level(&mut self) -> Result<u8, MtpError<<D as PtpIo>::TransportError>> {
        let prop = self
            .get_device_prop_value::<properties::BatteryLevel>()
            .await?;
        Ok(prop.data.data)
    }

    /// A human-readable description of the device
    pub async fn friendly_name(
        &mut self,
    ) -> Result<PtpString, MtpError<<D as PtpIo>::TransportError>> {
        let prop = self
            .get_device_prop_value::<properties::DeviceFriendlyName>()
            .await?;
        Ok(prop.data.data)
    }

    /// Whether the [`ObjectProperty`] `T` is writeable for the given `format`
    pub async fn object_property_can_be_modified<T>(
        &mut self,
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
    pub async fn device_property_can_be_modified<T>(
        &mut self,
    ) -> Result<bool, MtpError<<D as PtpIo>::TransportError>>
    where
        T: DeviceProperty,
    {
        let desc = self.get_device_prop_desc::<T>().await?;
        Ok(desc.data.data.get_set == GetSet::ReadWrite)
    }

    // === Operation wrappers ===

    /// Send a [`CloseSession`] operation
    pub async fn close_session(
        &mut self,
    ) -> Response<CloseSession, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(CloseSession::new(transaction_id, session_id), None)
            .await
    }

    /// Send a [`GetStorageIDs`] operation
    pub async fn get_storage_ids(
        &mut self,
    ) -> Response<GetStorageIDs, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(GetStorageIDs::new(transaction_id, session_id), None)
            .await
    }

    /// Send a [`GetStorageInfo`] operation
    pub async fn get_storage_info(
        &mut self,
        storage: StorageId,
    ) -> Response<GetStorageInfo, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            GetStorageInfo::new(transaction_id, session_id, storage),
            None,
        )
        .await
    }

    /// Send a [`GetNumObjects`] operation
    pub async fn get_num_objects(
        &mut self,
        storage: StorageId,
        format: Option<ObjectFormatCode>,
        parent: Option<ObjectHandle>,
    ) -> Response<GetNumObjects, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            GetNumObjects::new(transaction_id, session_id, storage, format, parent),
            None,
        )
        .await
    }

    /// Send a [`GetObjectHandles`] operation
    pub async fn get_object_handles(
        &mut self,
        storage: StorageId,
        format: Option<ObjectFormatCode>,
        parent: Option<ObjectHandle>,
    ) -> Response<GetObjectHandles, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            GetObjectHandles::new(transaction_id, session_id, storage, format, parent),
            None,
        )
        .await
    }

    /// Send a [`GetObjectInfo`] operation
    pub async fn get_object_info(
        &mut self,
        object: ObjectHandle,
    ) -> Response<GetObjectInfo, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(GetObjectInfo::new(transaction_id, session_id, object), None)
            .await
    }

    /// Send a [`GetObject`] operation
    pub async fn get_object(
        &mut self,
        object: ObjectHandle,
    ) -> Response<GetObject, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(GetObject::new(transaction_id, session_id, object), None)
            .await
    }

    /// Send a [`GetThumb`] operation
    pub async fn get_thumb(
        &mut self,
        object: ObjectHandle,
    ) -> Response<GetThumb, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(GetThumb::new(transaction_id, session_id, object), None)
            .await
    }

    /// Send a [`DeleteObject`] operation
    pub async fn delete_object(
        &mut self,
        object: ObjectHandle,
        format: Option<ObjectFormatCode>,
    ) -> Response<DeleteObject, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            DeleteObject::new(transaction_id, session_id, object, format),
            None,
        )
        .await
    }

    /// Send a [`SendObjectInfo`] operation
    pub async fn send_object_info(
        &mut self,
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
        self.send_operation(
            SendObjectInfo::new(transaction_id, session_id, storage, parent),
            Some(encoded_object_info.into_inner()),
        )
        .await
    }

    /// Send a [`SendObject`] operation
    pub async fn send_object<T>(
        &mut self,
        object_data: T,
    ) -> Response<SendObject, MtpError<<D as PtpIo>::TransportError>>
    where
        T: Into<Vec<u8>>,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            SendObject::new(transaction_id, session_id),
            Some(object_data.into()),
        )
        .await
    }

    /// Send a [`InitiateCapture`] operation
    pub async fn initiate_capture(
        &mut self,
        storage: Option<StorageId>,
        format: Option<ObjectFormatCode>,
    ) -> Response<InitiateCapture, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            InitiateCapture::new(transaction_id, session_id, storage, format),
            None,
        )
        .await
    }

    /// Send a [`FormatStore`] operation
    pub async fn format_store(
        &mut self,
        storage: StorageId,
        fs: Option<FilesystemType>,
    ) -> Response<FormatStore, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            FormatStore::new(transaction_id, session_id, storage, fs),
            None,
        )
        .await
    }

    /// Send a [`ResetDevice`] operation
    pub async fn reset_device(
        &mut self,
    ) -> Response<ResetDevice, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(ResetDevice::new(transaction_id, session_id), None)
            .await
    }

    /// Send a [`SelfTest`] operation
    pub async fn self_test(
        &mut self,
        test_type: SelfTestType,
    ) -> Response<SelfTest, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(SelfTest::new(transaction_id, session_id, test_type), None)
            .await
    }

    /// Send a [`SetObjectProtection`] operation
    pub async fn set_object_protection(
        &mut self,
        object: ObjectHandle,
        status: ProtectionStatus,
    ) -> Response<SetObjectProtection, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            SetObjectProtection::new(transaction_id, session_id, object, status),
            None,
        )
        .await
    }

    /// Send a [`PowerDown`] operation
    pub async fn power_down(
        &mut self,
    ) -> Response<PowerDown, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(PowerDown::new(transaction_id, session_id), None)
            .await
    }

    /// Send a [`GetDevicePropDesc`] operation
    pub async fn get_device_prop_desc<T>(
        &mut self,
    ) -> Response<GetDevicePropDesc<T>, MtpError<<D as PtpIo>::TransportError>>
    where
        T: DeviceProperty,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            GetDevicePropDesc::<T>::new(transaction_id, session_id),
            None,
        )
        .await
    }

    /// Send a [`GetDevicePropValue`] operation
    pub async fn get_device_prop_value<T>(
        &mut self,
    ) -> Response<GetDevicePropValue<T>, MtpError<<D as PtpIo>::TransportError>>
    where
        T: DeviceProperty,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            GetDevicePropValue::<T>::new(transaction_id, session_id),
            None,
        )
        .await
    }

    /// Send a [`SetDevicePropValue`] operation
    pub async fn set_device_prop_value<T>(
        &mut self,
        value: Vec<u8>,
    ) -> Response<SetDevicePropValue<T>, MtpError<<D as PtpIo>::TransportError>>
    where
        T: DeviceProperty,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            SetDevicePropValue::<T>::new(transaction_id, session_id),
            Some(value),
        )
        .await
    }

    /// Send a [`ResetDevicePropValue`] operation
    pub async fn reset_device_prop_value<T>(
        &mut self,
    ) -> Response<ResetDevicePropValue<T>, MtpError<<D as PtpIo>::TransportError>>
    where
        T: DeviceProperty,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            ResetDevicePropValue::<T>::new(transaction_id, session_id),
            None,
        )
        .await
    }

    /// Send a [`TerminateOpenCapture`] operation
    pub async fn terminate_open_capture(
        &mut self,
        transaction_id: TransactionId,
    ) -> Response<TerminateOpenCapture, MtpError<<D as PtpIo>::TransportError>> {
        let next_transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            TerminateOpenCapture::new(next_transaction_id, session_id, transaction_id),
            None,
        )
        .await
    }

    /// Send a [`MoveObject`] operation
    pub async fn move_object(
        &mut self,
        object: ObjectHandle,
        storage: StorageId,
        parent: Option<ObjectHandle>,
    ) -> Response<MoveObject, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            MoveObject::new(transaction_id, session_id, object, storage, parent),
            None,
        )
        .await
    }

    /// Send a [`CopyObject`] operation
    pub async fn copy_object(
        &mut self,
        object: ObjectHandle,
        storage: StorageId,
        parent: Option<ObjectHandle>,
    ) -> Response<CopyObject, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            CopyObject::new(transaction_id, session_id, object, storage, parent),
            None,
        )
        .await
    }

    /// Send a [`GetPartialObject`] operation
    pub async fn get_partial_object(
        &mut self,
        object: ObjectHandle,
        offset: u32,
        len: u32,
    ) -> Response<GetPartialObject, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            GetPartialObject::new(transaction_id, session_id, object, offset, len),
            None,
        )
        .await
    }

    /// Send an [`InitiateOpenCapture`] operation
    pub async fn initiate_open_capture(
        &mut self,
        storage: Option<StorageId>,
        format: Option<ObjectFormatCode>,
    ) -> Response<InitiateOpenCapture, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            InitiateOpenCapture::new(transaction_id, session_id, storage, format),
            None,
        )
        .await
    }

    /// Send a [`GetObjectPropsSupported`] operation
    pub async fn get_object_props_supported(
        &mut self,
        format: ObjectFormatCode,
    ) -> Response<GetObjectPropsSupported, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            GetObjectPropsSupported::new(transaction_id, session_id, format),
            None,
        )
        .await
    }

    /// Send a [`GetObjectPropDesc`] operation
    pub async fn get_object_prop_desc<T>(
        &mut self,
        format: ObjectFormatCode,
    ) -> Response<GetObjectPropDesc<T>, MtpError<<D as PtpIo>::TransportError>>
    where
        T: ObjectProperty,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            GetObjectPropDesc::<T>::new(transaction_id, session_id, format),
            None,
        )
        .await
    }

    /// Send a [`GetObjectPropValue`] operation
    pub async fn get_object_prop_value<T>(
        &mut self,
        object: ObjectHandle,
    ) -> Response<GetObjectPropValue<T>, MtpError<<D as PtpIo>::TransportError>>
    where
        T: ObjectProperty,
    {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            GetObjectPropValue::<T>::new(transaction_id, session_id, object),
            None,
        )
        .await
    }

    /// Send a [`SetObjectPropValue`] operation
    pub async fn set_object_prop_value<T>(
        &mut self,
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
        self.send_operation(
            SetObjectPropValue::<T>::new(transaction_id, session_id, object),
            Some(encoded_value.into_inner()),
        )
        .await
    }

    /// Send a [`GetObjectReferences`] operation
    pub async fn get_object_references(
        &mut self,
        object: ObjectHandle,
    ) -> Response<GetObjectReferences, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            GetObjectReferences::new(transaction_id, session_id, object),
            None,
        )
        .await
    }

    /// Send a [`SetObjectReferences`] operation
    pub async fn set_object_references(
        &mut self,
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
        self.send_operation(
            SetObjectReferences::new(transaction_id, session_id, object),
            Some(encoded_references.into_inner()),
        )
        .await
    }

    /// Send a [`Skip`] operation
    pub async fn skip(
        &mut self,
        skip: u32,
    ) -> Response<Skip, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(Skip::new(transaction_id, session_id, skip), None)
            .await
    }

    // == Enhanced Operations ==
    //
    // Defined in Appendix E

    /// Send a [`GetObjectPropList`] operation
    pub async fn get_object_prop_list(
        &mut self,
        object: ObjectHandle,
        format: Option<ObjectFormatCode>,
        property: Option<ObjectPropertyCode>,
        group: Option<u32>,
        depth: Option<u32>,
    ) -> Response<GetObjectPropList, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
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

    /// Send a [`SetObjectPropList`] operation
    pub async fn set_object_prop_list(
        &mut self,
        props: ObjectPropList,
    ) -> Response<SetObjectPropList, MtpError<<D as PtpIo>::TransportError>> {
        let mut object_prop_list = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut object_prop_list);
        props.to_writer(&mut writer, self.endian())?;

        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            SetObjectPropList::new(transaction_id, session_id),
            Some(object_prop_list.into_inner()),
        )
        .await
    }

    /// Send a [`GetInterdependentPropDesc`] operation
    pub async fn get_interdependent_prop_desc(
        &mut self,
        format: ObjectFormatCode,
    ) -> Response<GetInterdependentPropDesc, MtpError<<D as PtpIo>::TransportError>> {
        let transaction_id = self.next_transaction_id();
        let session_id = self.id;
        self.send_operation(
            GetInterdependentPropDesc::new(transaction_id, session_id, format),
            None,
        )
        .await
    }

    /// Send a [`SendObjectPropList`] operation
    pub async fn send_object_prop_list(
        &mut self,
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

impl<D> Deref for MtpSession<D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl<D> DerefMut for MtpSession<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.device
    }
}
