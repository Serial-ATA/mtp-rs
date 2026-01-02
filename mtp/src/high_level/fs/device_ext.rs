use super::{File, Folder};

use std::future::Future;

use mtp_spec::communication::{SessionId, response};
use mtp_spec::device::storage::id::StorageId;
use mtp_spec::device::{Device, PtpIo};
use mtp_spec::error::MtpError;
use mtp_spec::object::info::{ObjectInfo, ProtectionStatus};
use mtp_spec::object::types::properties::ObjectSize;
use mtp_spec::object::types::{Association, FolderType, ObjectFormatCode, PtpString};

/// Filesystem extension trait for [`Device`]s
///
/// This provides higher-level methods to perform operations on MTP-compatible devices as if they
/// were real filesystems.
pub trait DeviceFsExt {
    fn mkdir<N>(
        &mut self,
        session_id: SessionId,
        parent: Option<&Folder>,
        name: N,
    ) -> impl Future<Output = Result<Folder, MtpError<<Self as PtpIo>::TransportError>>> + Send
    where
        Self: Device,
        N: AsRef<str> + Send;

    fn create<N>(
        &mut self,
        session_id: SessionId,
        parent: Option<&Folder>,
        name: N,
        format: ObjectFormatCode,
        data: Vec<u8>,
    ) -> impl Future<Output = Result<File, MtpError<<Self as PtpIo>::TransportError>>> + Send
    where
        Self: Device,
        N: AsRef<str> + Send;
}

impl<D> DeviceFsExt for D
where
    D: Device,
{
    async fn mkdir<N>(
        &mut self,
        session_id: SessionId,
        parent: Option<&Folder>,
        name: N,
    ) -> Result<Folder, MtpError<<Self as PtpIo>::TransportError>>
    where
        Self: Device,
        N: AsRef<str> + Send,
    {
        let storage = parent.map(|p| p.storage_id);
        let parent_object = parent.map(|p| p.id);

        // TODO: Getting invalid parameter when trying to create within a directory and InvalidObjectHandle when trying to create at root.
        //       Maybe samsung issue?
        let name_ptp = PtpString::try_from(name.as_ref())?;
        let response = self
            .send_object_info(
                session_id,
                storage,
                parent_object,
                ObjectInfo {
                    storage_id: storage.unwrap_or_default(),
                    object_format: ObjectFormatCode::Association,
                    protection_status: ProtectionStatus::NoProtection,
                    parent_object,
                    association: Some(Association::GenericFolder {
                        ty: FolderType::Generic,
                    }),
                    filename: name_ptp,
                    ..Default::default()
                },
            )
            .await?;

        self.send_object(session_id, Vec::new()).await?;

        let response::SendObjectInfo {
            storage_id,
            parent: _,
            reserved_handle,
        } = response.data;

        let object_info = self
            .get_object_info(session_id, reserved_handle)
            .await?
            .data
            .data;

        Ok(Folder {
            id: reserved_handle,
            storage_id,
            name: object_info.filename.to_string(),
            format: ObjectFormatCode::Association,
            protection_status: ProtectionStatus::NoProtection,
            date_created: object_info.date_created,
            date_modified: object_info.date_modified,
            children: vec![],
        })
    }

    // TODO: Error if format is association
    async fn create<N>(
        &mut self,
        session_id: SessionId,
        parent: Option<&Folder>,
        name: N,
        format: ObjectFormatCode,
        data: Vec<u8>,
    ) -> Result<File, MtpError<<Self as PtpIo>::TransportError>>
    where
        Self: Device,
        N: AsRef<str> + Send,
    {
        let storage = parent.map(|p| p.storage_id);
        let parent_object = parent.map(|p| p.id);

        let name_ptp = PtpString::try_from(name.as_ref())?;
        let response = self
            .send_object_info(
                session_id,
                storage,
                parent_object,
                ObjectInfo {
                    object_format: format,
                    compressed_size: data.len() as u32,
                    association: None,
                    filename: name_ptp,

                    // Not required for SendObjectInfo
                    storage_id: StorageId::DEFAULT_STORE,
                    protection_status: ProtectionStatus::default(),
                    thumbnail: None,
                    parent_object: None,
                    sequence_number: 0,
                    date_created: None,
                    date_modified: None,
                    keywords: Default::default(),
                },
            )
            .await?;

        let response::SendObjectInfo {
            storage_id,
            parent,
            reserved_handle,
        } = response.data;

        self.send_object(session_id, data).await?;

        let object_info = self
            .get_object_info(session_id, reserved_handle)
            .await?
            .data
            .data;

        let size_response = self
            .get_object_prop_value::<ObjectSize>(session_id, reserved_handle)
            .await?;

        Ok(File {
            storage_id,
            id: reserved_handle,
            parent,
            name: object_info.filename.to_string(),
            size: size_response.data.data,
            format: object_info.object_format,
            protection_status: object_info.protection_status,
            date_created: object_info.date_created,
            date_modified: object_info.date_modified,
        })
    }
}
