use super::Folder;
use crate::error::Error;

use std::future::Future;

use mtp_spec::communication::SessionId;
use mtp_spec::device::{Device, PtpIo};
use mtp_spec::error::MtpError;
use mtp_spec::object::info::{ObjectInfo, ProtectionStatus};
use mtp_spec::object::types::{
    Association, AssociationType, FolderType, ObjectFormatCode, PtpString,
};

/// Filesystem extension trait for [`Device`]s
///
/// This provides higher-level methods to perform operations on MTP-compatible devices as if they
/// were real filesystems.
pub trait DeviceFsExt {
    fn mkdir(
        &mut self,
        session_id: SessionId,
        parent: Option<&Folder>,
        name: String,
    ) -> impl Future<Output = Result<Folder, <Self as PtpIo>::Error>> + Send
    where
        Self: Device,
        <Self as PtpIo>::Error: From<Error>,
        <Self as PtpIo>::Error: From<MtpError>;
}

impl<D> DeviceFsExt for D
where
    D: Device,
    <D as PtpIo>::Error: From<MtpError>,
{
    async fn mkdir(
        &mut self,
        session_id: SessionId,
        parent: Option<&Folder>,
        name: String,
    ) -> Result<Folder, <Self as PtpIo>::Error>
    where
        Self: Device,
        <Self as PtpIo>::Error: From<Error>,
        <Self as PtpIo>::Error: From<MtpError>,
    {
        let storage = parent.map(|p| p.storage_id);
        let parent_object = parent.map(|p| p.id);

        // TODO: Getting invalid parameter when trying to create within a directory and InvalidObjectHandle when trying to create at root.
        //       Maybe samsung issue?
        let name_ptp = PtpString::try_from(name.clone())?;
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
            .await?
            .map_err(Into::<MtpError>::into)?;

        self.send_object(session_id, Vec::new())
            .await?
            .map_err(Into::<MtpError>::into)?;

        Ok(Folder {
            id: response.data.reserved_handle,
            storage_id: response.data.storage_id,
            name,
            format: ObjectFormatCode::Association,
            protection_status: ProtectionStatus::NoProtection,
            date_created: None,
            date_modified: None,
            children: vec![],
        })
    }
}
