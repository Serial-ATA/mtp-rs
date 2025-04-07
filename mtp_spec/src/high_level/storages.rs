use crate::communication::SessionId;
use crate::device::storage::id::StorageId;
use crate::device::storage::info::{AccessCapability, FilesystemType, StorageType};
use crate::device::{Device, PtpIo};
use crate::error::MtpError;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

pub struct Storage {
    /// The device-specific ID of the storage
    pub id: StorageId,
    /// The physical nature of the storage
    pub ty: StorageType,
    /// The logical file system in use on the storage
    pub filesystem_type: FilesystemType,
    /// Globally-applicable write-protection affecting this storage
    pub access_capability: AccessCapability,
    /// The maximum capacity of the storage (**in bytes**).
    pub max_capacity: u64,
    /// How much space remains to be written to on the drive (**in bytes**).
    pub free_space: u64,
    /// The number of additional objects that can be written to this storage.
    pub free_space_in_objects: u32,
    /// A human-readable string identifying this storage, such as "256Mb SD Card" or "20Gb HDD"
    pub description: Option<String>,
    /// A unique, programmatically relevant volume identifier, such as a serial number.
    pub volume_identifier: String,
}

pub trait DeviceStorageExt: Device
where
    <Self as PtpIo>::Error: From<MtpError>,
{
    /// Get all [`Storage`]s on the device
    async fn storages(
        &mut self,
        session_id: SessionId,
    ) -> Result<Vec<Storage>, <Self as PtpIo>::Error>
    where
        Self: Device,
        <Self as PtpIo>::Error: From<MtpError>;
}

impl<D> DeviceStorageExt for D
where
    D: Device,
    <D as PtpIo>::Error: From<MtpError>,
{
    async fn storages(
        &mut self,
        session_id: SessionId,
    ) -> Result<Vec<Storage>, <D as PtpIo>::Error> {
        let storage_ids = match self.get_storage_ids(session_id).await? {
            Ok(storages) => storages.data.data,
            Err(e) => todo!(),
        };

        let mut storages = Vec::with_capacity(storage_ids.len());
        for storage_id in storage_ids.iter().copied() {
            let storage_info = match self.get_storage_info(session_id, storage_id).await? {
                Ok(info) => info.data.data,
                Err(e) => todo!(),
            };

            storages.push(Storage {
                id: storage_id,
                ty: storage_info.storage_type,
                filesystem_type: storage_info.filesystem_type,
                access_capability: storage_info.access_capability,
                max_capacity: storage_info.max_capacity,
                free_space: storage_info.free_space,
                free_space_in_objects: storage_info.free_space_in_objects,
                description: storage_info
                    .storage_description
                    .as_ref()
                    .map(ToString::to_string),
                volume_identifier: storage_info.volume_identifier.to_string(),
            });
        }

        Ok(storages)
    }
}
