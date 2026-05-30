//! Simplified storage implementations

use crate::device::session::MtpSession;
use crate::device::storage::{AccessCapability, FilesystemType, StorageId, StorageType};
use crate::device::{Device, PtpIo};
use crate::error::MtpError;

/// An easier-to-use variant of [`StorageInfo`]
///
/// This has the same contents as [`StorageInfo`], but with the [`PtpString`]s pre-converted to
/// [`String`]s.
///
/// These are obtained from [`SessionStorageExt::storages()`].
///
/// [`StorageInfo`]: crate::device::storage::info::StorageInfo
/// [`PtpString`]: crate::object::types::PtpString
pub struct Storage {
    /// The device-specific ID of the storage
    pub id: StorageId,
    /// The physical nature of the storage
    pub ty: StorageType,
    /// The logical file system in use on the storage
    pub filesystem_type: FilesystemType,
    /// Globally applicable write-protection affecting this storage
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

/// High-level methods to work with storages on a device
pub trait SessionStorageExt<D>
where
    D: Device,
{
    /// Get all [`Storage`]s on the device
    ///
    /// This is a combination of the [`GetStorageIDs`] and [`GetStorageInfo`] operations.
    ///
    /// [`GetStorageIDs`]: crate::communication::operation::GetStorageIDs
    /// [`GetStorageInfo`]: crate::communication::operation::GetStorageInfo
    fn storages(
        &mut self,
    ) -> impl Future<Output = Result<Vec<Storage>, MtpError<<D as PtpIo>::TransportError>>> + Send;
}

impl<D> SessionStorageExt<D> for MtpSession<D>
where
    D: Device,
{
    async fn storages(&mut self) -> Result<Vec<Storage>, MtpError<<D as PtpIo>::TransportError>> {
        let storage_ids_response = self.get_storage_ids().await?;
        let storage_ids = storage_ids_response.data.data;

        let mut storages = Vec::with_capacity(storage_ids.len());
        for storage_id in storage_ids.iter().copied() {
            let storage_info_response = self.get_storage_info(storage_id).await?;
            let storage_info = storage_info_response.data.data;

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
