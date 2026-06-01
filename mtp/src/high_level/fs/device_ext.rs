use super::{File, Folder};
use std::collections::HashMap;

use mtp_spec::communication::response;
use mtp_spec::device::session::MtpSession;
use mtp_spec::device::{Device, PtpIo};
use mtp_spec::error::MtpError;
use mtp_spec::object::properties::ObjectSize;
use mtp_spec::object::{
    Association, FolderType, ObjectFormatCode, ObjectHandle, ObjectInfo, ProtectionStatus,
    PtpString,
};
use std::future::Future;
use std::sync::Weak;
use tokio::sync::{OnceCell, RwLock};

/// Filesystem extension trait for [`Device`]s
///
/// This provides higher-level methods to perform operations on MTP-compatible devices as if they
/// were real filesystems.
pub trait SessionFsExt<D>
where
    D: Device,
{
    /// Create a new directory on the target device
    fn mkdir<N>(
        &self,
        parent: Option<&Folder<D>>,
        name: N,
    ) -> impl Future<Output = Result<Folder<D>, MtpError<<D as PtpIo>::TransportError>>> + Send
    where
        N: AsRef<str> + Send;

    /// Create a new file with the given `data` on the target device
    fn create<N>(
        &self,
        parent: Option<&Folder<D>>,
        name: N,
        format: ObjectFormatCode,
        data: Vec<u8>,
    ) -> impl Future<Output = Result<File<D>, MtpError<<D as PtpIo>::TransportError>>> + Send
    where
        N: AsRef<str> + Send;
}

impl<D> SessionFsExt<D> for MtpSession<D>
where
    D: Device,
{
    async fn mkdir<N>(
        &self,
        parent: Option<&Folder<D>>,
        name: N,
    ) -> Result<Folder<D>, MtpError<<D as PtpIo>::TransportError>>
    where
        N: AsRef<str> + Send,
    {
        let storage = parent.map(|p| p.storage_id);
        let parent_object = parent.map(|p| p.id);
        let fs_context = parent.map(|p| p.fs.clone()).unwrap_or_else(Weak::new);

        // TODO: Getting invalid parameter when trying to create within a directory and InvalidObjectHandle when trying to create at root.
        //       Maybe samsung issue?
        let name_ptp = PtpString::try_from(name.as_ref())?;
        let response = self
            .send_object_info(ObjectInfo {
                storage_id: storage.unwrap_or_default(),
                object_format: ObjectFormatCode::Association,
                protection_status: ProtectionStatus::NoProtection,
                parent_object,
                association: Some(Association::GenericFolder {
                    ty: FolderType::Generic,
                }),
                filename: name_ptp,
                ..Default::default()
            })
            .await?;

        self.send_object(Vec::new()).await?;

        let response::SendObjectInfo {
            storage_id,
            parent: _,
            reserved_handle,
        } = response.data;

        let object_info = self.get_object_info(reserved_handle).await?.data.data;

        Ok(Folder {
            fs: fs_context,
            storage_id,
            id: reserved_handle,
            parent_id: parent.map(|p| p.id).unwrap_or(ObjectHandle::NONE),
            name: object_info.filename.to_string(),
            format: ObjectFormatCode::Association,
            protection_status: ProtectionStatus::NoProtection,
            date_created: object_info.date_created,
            date_modified: object_info.date_modified,
            children: RwLock::new(HashMap::new()),
        })
    }

    async fn create<N>(
        &self,
        parent: Option<&Folder<D>>,
        name: N,
        format: ObjectFormatCode,
        data: Vec<u8>,
    ) -> Result<File<D>, MtpError<<D as PtpIo>::TransportError>>
    where
        N: AsRef<str> + Send,
    {
        if format == ObjectFormatCode::Association {
            return Err(MtpError::UnsupportedOperation);
        }

        let storage = parent.map(|p| p.storage_id);
        let parent_object = parent.map(|p| p.id);

        // Grab the filesystem context from the parent, or create a detached one if root
        let fs_context = parent.map(|p| p.fs.clone()).unwrap_or_else(Weak::new);

        let name_ptp = PtpString::try_from(name.as_ref())?;
        let response = self
            .send_object_info(ObjectInfo {
                storage_id: storage.unwrap_or_default(),
                parent_object,
                object_format: format,
                compressed_size: data.len() as u32,
                association: None,
                filename: name_ptp,

                // Not required for SendObjectInfo
                protection_status: ProtectionStatus::default(),
                thumbnail: None,
                image_details: None,
                sequence_number: 0,
                date_created: None,
                date_modified: None,
                keywords: Default::default(),
            })
            .await?;

        let response::SendObjectInfo {
            storage_id,
            parent: _,
            reserved_handle,
        } = response.data;

        self.send_object(data).await?;

        let object_info = self.get_object_info(reserved_handle).await?.data.data;

        let size_response = self
            .get_object_prop_value::<ObjectSize>(reserved_handle)
            .await?;

        Ok(File {
            fs: fs_context,
            storage_id,
            id: reserved_handle,
            parent_id: parent.map(|p| p.id).unwrap_or(ObjectHandle::NONE),
            name: object_info.filename.to_string(),
            size: size_response.data.data,
            format: object_info.object_format,
            protection_status: object_info.protection_status,
            date_modified: object_info.date_modified,
            date_created: object_info.date_created,
            spool: OnceCell::new(),
        })
    }
}
