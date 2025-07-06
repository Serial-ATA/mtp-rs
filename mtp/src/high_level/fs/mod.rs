mod device_ext;
pub use device_ext::*;

use crate::communication::SessionId;
use crate::device::storage::id::StorageId;
use crate::device::{Device, PtpIo};
use crate::error::{Error, MtpError};
use crate::object::info::ProtectionStatus;
use crate::object::types::properties::{ObjectFileName, ObjectFormat, ObjectSize, ParentObject};
use crate::object::types::{DateTime, ObjectFormatCode, ObjectHandle, PtpString};

use mtp_spec::object::info::ObjectInfo;
use std::io::{Read, Write};
use std::sync::Arc;

/// Representation of a file on an MTP-compatible device
///
/// Note that it is **not** guaranteed that a device will support any or all of the operations available
/// on `File`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct File {
    /// The device-specific ID of the storage where this file lives
    pub storage_id: StorageId,
    pub id: ObjectHandle,
    pub parent: ObjectHandle,
    pub name: String,
    pub size: u64,
    pub format: ObjectFormatCode,
    pub protection_status: ProtectionStatus,
    pub date_created: Option<DateTime>,
    pub date_modified: Option<DateTime>,
}

impl File {
    /// Open the file
    ///
    /// This will fetch the data from the device, and then write it to a temp file. The returned handle
    /// is **temporary**, be sure to copy data to another file if persistence is needed.
    ///
    /// # Errors
    ///
    /// This can fail for a variety of reasons, namely:
    ///
    /// * The device doesn't support fetching object data (either for this object or in general)
    /// * The object no longer exists on the device
    /// * Any errors from the transport backend
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use mtp::high_level::fs::{FileSystem, FolderEntry};
    /// use mtp::high_level::storages::DeviceStorageExt;
    /// use mtp::usb::device_list;
    ///
    /// # async fn main() -> mtp::error::Result<()> {
    /// // Get the first MTP-eligible device
    /// use std::io::Read;
    /// let device = device_list()?.next().expect("No devices");
    /// let (mut handle, session_id) = device?.open().await?;
    ///
    /// // Get whatever the first storage happens to be
    /// let storages = handle.storages(session_id).await?;
    /// let storage = storages.first().expect("no storages");
    ///
    /// // Load the storage and find the first file
    /// let fs = FileSystem::load(&mut handle, session_id, storage.id).await?;
    /// 'outer: for folder in fs.contents {
    ///     for child in folder.children {
    ///         let FolderEntry::File(file) = child else {
    ///             continue;
    ///         };
    ///
    ///         // Print out whatever the first file's contents happen to be
    ///         let mut open_file = file.open(&mut handle, session_id).await?;
    ///
    ///         println!("Contents of: {}", file.name);
    ///
    ///         let mut contents = Vec::new();
    ///         open_file.read_to_end(&mut contents)?;
    ///
    ///         println!("{:X?}", contents);
    ///         break 'outer;
    ///     }
    /// }
    ///
    /// # Ok(()) }
    /// ```
    pub async fn open<D>(
        &self,
        device: &mut D,
        session_id: SessionId,
    ) -> Result<std::fs::File, <D as PtpIo>::Error>
    where
        D: Device,
        <D as PtpIo>::Error: From<Error>,
        <D as PtpIo>::Error: From<MtpError>,
    {
        let object = device
            .get_object(session_id, self.id)
            .await?
            .map_err(Into::<MtpError>::into)?;
        let file_data = object.data.data;

        let mut tmp = tempfile::tempfile().map_err(Into::<Error>::into)?;
        tmp.write_all(&file_data).map_err(Into::<Error>::into)?;

        Ok(tmp)
    }

    /// Attempt to rename this file on the device
    ///
    /// # Errors
    ///
    /// This can fail for a variety of reasons, namely:
    ///
    /// * The device doesn't support renaming
    /// * The object no longer exists on the device
    /// * Any errors from the transport backend
    pub async fn rename<D, N>(
        &self,
        device: &mut D,
        session_id: SessionId,
        name: N,
    ) -> Result<Self, <D as PtpIo>::Error>
    where
        D: Device,
        N: Into<String>,
        <D as PtpIo>::Error: From<Error>,
        <D as PtpIo>::Error: From<MtpError>,
    {
        let id;
        let parent;
        let storage_id;

        let name_str = name.into();
        if name_str == self.name {
            return Ok(self.clone());
        }

        let name_ptp = PtpString::try_from(name_str.clone())?;
        match device
            .object_property_can_be_modified::<ObjectFileName>(session_id, self.format)
            .await
        {
            Ok(true) => {
                id = self.id;
                parent = self.parent;
                storage_id = self.storage_id;
                let _ = device
                    .set_object_prop_value::<ObjectFileName>(session_id, self.id, name_ptp)
                    .await?
                    .map_err(Into::<MtpError>::into)?;
            },
            // Fallback to copy and delete
            Ok(false) => {
                let mut f = self.open(device, session_id).await?;

                let mut data = Vec::new();
                f.read_to_end(&mut data).map_err(Into::<Error>::into)?;

                let response = device
                    .send_object_info(
                        session_id,
                        Some(self.storage_id),
                        Some(self.parent),
                        ObjectInfo {
                            storage_id: self.storage_id,
                            object_format: self.format,
                            protection_status: self.protection_status,
                            compressed_size: 0,
                            thumbnail: None,
                            parent_object: None,
                            association_type: None,
                            sequence_number: 0,
                            filename: name_ptp,
                            date_created: self.date_created,
                            date_modified: self.date_modified,
                            keywords: Default::default(),
                        },
                    )
                    .await?
                    .map_err(Into::<MtpError>::into)?;

                id = response.data.reserved_handle;
                parent = response.data.parent;
                storage_id = response.data.storage_id;

                device
                    .send_object(session_id, data)
                    .await?
                    .map_err(Into::<MtpError>::into)?;

                device
                    .delete_object(session_id, self.id, Some(self.format))
                    .await?
                    .map_err(Into::<MtpError>::into)?;
            },
            Err(e) => return Err(e),
        }

        Ok(Self {
            storage_id,
            parent,
            id,
            name: name_str,
            size: self.size,
            format: self.format,
            protection_status: self.protection_status,
            date_created: self.date_created,
            date_modified: self.date_modified,
        })
    }
}

/// Representation of a folder on an MTP-compatible device
///
/// Note that it is **not** guaranteed that a device will support any or all of the operations available
/// on `Folder`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Folder {
    /// The device-specific ID of the storage where this folder lives
    pub id: ObjectHandle,
    pub storage_id: StorageId,
    pub name: String,
    pub format: ObjectFormatCode,
    pub protection_status: ProtectionStatus,
    pub date_created: Option<DateTime>,
    pub date_modified: Option<DateTime>,
    pub children: Vec<Arc<FolderEntry>>,
}

impl Folder {
    /// Attempt to rename this folder on the device
    ///
    /// # Errors
    ///
    /// This can fail for a variety of reasons, namely:
    ///
    /// * The device doesn't support renaming
    /// * The object no longer exists on the device
    /// * Any errors from the transport backend
    pub async fn rename<D, N>(
        &self,
        device: &mut D,
        session_id: SessionId,
        name: N,
    ) -> Result<Self, <D as PtpIo>::Error>
    where
        D: Device,
        N: Into<String>,
        <D as PtpIo>::Error: From<MtpError>,
    {
        let name = PtpString::try_from(name.into())?;
        let name_str = name.to_string();
        let _ = device
            .set_object_prop_value::<ObjectFileName>(session_id, self.id, name)
            .await?
            .map_err(Into::<MtpError>::into)?;

        Ok(Self {
            storage_id: self.storage_id,
            id: self.id,
            name: name_str,
            format: self.format,
            protection_status: self.protection_status,
            date_created: self.date_created,
            date_modified: self.date_modified,
            children: self.children.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderEntry {
    File(File),
    Folder(Folder),
}

impl FolderEntry {
    /// Get the name of this entry
    pub fn name(&self) -> &str {
        match self {
            FolderEntry::File(f) => &f.name,
            FolderEntry::Folder(f) => &f.name,
        }
    }

    /// Attempt to rename this entry on the device
    ///
    /// # Errors
    ///
    /// See [`File::rename()`] and [`Folder::rename()`]
    pub async fn rename<D, N>(
        &self,
        device: &mut D,
        session_id: SessionId,
        name: N,
    ) -> Result<Self, <D as PtpIo>::Error>
    where
        D: Device,
        N: Into<String>,
        <D as PtpIo>::Error: From<Error>,
        <D as PtpIo>::Error: From<MtpError>,
    {
        match self {
            FolderEntry::File(f) => {
                Ok(FolderEntry::File(f.rename(device, session_id, name).await?))
            },
            FolderEntry::Folder(f) => Ok(FolderEntry::Folder(
                f.rename(device, session_id, name).await?,
            )),
        }
    }
}

pub struct FileSystem {
    session_id: SessionId,
    storage_id: StorageId,
    pub contents: Vec<Folder>,
}

impl FileSystem {
    /// Load a `FileSystem` from the given `storage_id`
    ///
    /// Note that the speed of this depends entirely on the device and the numbers of files on the storage.
    /// See [Performance Considerations].
    ///
    /// [Performance Considerations]: https://docs.rs/mtp/latest/mtp/#performance-considerations
    pub async fn load<D>(
        device: &mut D,
        session_id: SessionId,
        storage_id: StorageId,
    ) -> Result<Self, <D as PtpIo>::Error>
    where
        D: Device,
        <D as PtpIo>::Error: From<MtpError>,
    {
        Self::load_with_callback(device, session_id, storage_id, |_| {}).await
    }

    /// Load a `FileSystem` from the given `storage_id`
    ///
    /// This takes a callback that will be called for *every* [`ObjectHandle`] that is received during
    /// the load.
    ///
    /// Note that the speed of this depends entirely on the device and the numbers of files on the storage.
    /// See [Performance Considerations].
    ///
    /// [Performance Considerations]: https://docs.rs/mtp/latest/mtp/#performance-considerations
    pub async fn load_with_callback<D, F>(
        device: &mut D,
        session_id: SessionId,
        storage_id: StorageId,
        callback: F,
    ) -> Result<Self, <D as PtpIo>::Error>
    where
        D: Device,
        <D as PtpIo>::Error: From<MtpError>,
        F: FnMut(ObjectHandle),
    {
        let mut ret = Self {
            session_id,
            storage_id,
            contents: Vec::new(),
        };

        ret.refresh_with_callback(device, callback).await?;
        Ok(ret)
    }

    /// Refresh the `FileSystem` to match the new state of the device
    ///
    /// Note that the speed of this depends entirely on the device and the numbers of files on the storage.
    /// See [Performance Considerations].
    ///
    /// [Performance Considerations]: https://docs.rs/mtp/latest/mtp/#performance-considerations
    pub async fn refresh<D>(&mut self, device: &mut D) -> Result<(), <D as PtpIo>::Error>
    where
        D: Device,
        <D as PtpIo>::Error: From<MtpError>,
    {
        self.refresh_with_callback(device, |_| {}).await
    }

    /// Refresh the `FileSystem` to match the new state of the device
    ///
    /// This takes a callback that will be called for *every* [`ObjectHandle`] that is received during
    /// the load.
    ///
    /// Note that the speed of this depends entirely on the device and the numbers of files on the storage.
    /// See [Performance Considerations].
    ///
    /// [Performance Considerations]: https://docs.rs/mtp/latest/mtp/#performance-considerations
    pub async fn refresh_with_callback<D>(
        &mut self,
        device: &mut D,
        mut callback: impl FnMut(ObjectHandle),
    ) -> Result<(), <D as PtpIo>::Error>
    where
        D: Device,
        <D as PtpIo>::Error: From<MtpError>,
    {
        let objects_response = device
            .get_object_handles(self.session_id, self.storage_id, None, None)
            .await?
            .map_err(Into::<MtpError>::into)?;
        let objects = objects_response.data.data;

        let mut entries = Vec::with_capacity(objects.len());
        for object in objects.iter().copied() {
            callback(object);

            let parent_response = device
                .get_object_prop_value::<ParentObject>(self.session_id, object)
                .await?;

            let parent = match parent_response {
                Ok(parent) => {
                    if parent.data.data == ObjectHandle::NONE {
                        None
                    } else {
                        Some(parent.data.data)
                    }
                },
                Err(e) => {
                    log::warn!("Failed to get parent object, skipping: {e}");
                    continue;
                },
            };

            let name_response = device
                .get_object_prop_value::<ObjectFileName>(self.session_id, object)
                .await?;

            let name = match name_response {
                Ok(name) => name.data.data,
                Err(e) => {
                    log::warn!("Failed to get object name, skipping: {e}");
                    continue;
                },
            };

            let format_response = device
                .get_object_prop_value::<ObjectFormat>(self.session_id, object)
                .await?;

            let format = match format_response {
                Ok(format) => format.data.data,
                Err(e) => {
                    log::warn!("Failed to get object format, skipping: {e}");
                    continue;
                },
            };

            match parent {
                Some(parent) => match format {
                    ObjectFormatCode::Association => {
                        entries.push((
                            parent,
                            FolderEntry::Folder(Folder {
                                storage_id: self.storage_id,
                                id: object,
                                name: name.to_string(),
                                format,
                                protection_status: ProtectionStatus::ReadOnly,
                                date_created: None,
                                date_modified: None,
                                children: Vec::new(),
                            }),
                        ));
                    },
                    _ => {
                        let size_response = device
                            .get_object_prop_value::<ObjectSize>(self.session_id, object)
                            .await?;

                        let size = match size_response {
                            Ok(size) => size.data.data,
                            Err(e) => {
                                log::warn!("Failed to get object size, skipping: {e}");
                                continue;
                            },
                        };

                        entries.push((
                            parent,
                            FolderEntry::File(File {
                                storage_id: self.storage_id,
                                id: object,
                                parent,
                                name: name.to_string(),
                                size,
                                format,
                                protection_status: ProtectionStatus::ReadOnly,
                                date_created: None,
                                date_modified: None,
                            }),
                        ));
                    },
                },
                None => {
                    self.contents.push(Folder {
                        storage_id: self.storage_id,
                        id: object,
                        name: name.to_string(),
                        format,
                        protection_status: ProtectionStatus::ReadOnly,
                        date_created: None,
                        date_modified: None,
                        children: Vec::new(),
                    });
                },
            }
        }

        let root_ids = self.contents.iter().map(|dir| dir.id).collect::<Vec<_>>();
        for (index, root) in root_ids.iter().enumerate() {
            let children = collect_children(*root, &entries);
            self.contents[index].children = children;
        }

        Ok(())
    }
}

fn collect_children(
    root: ObjectHandle,
    all_entries: &[(ObjectHandle, FolderEntry)],
) -> Vec<Arc<FolderEntry>> {
    let mut entries = Vec::new();
    for (parent, entry) in all_entries.iter() {
        if *parent != root {
            continue;
        }

        match entry {
            FolderEntry::File(_) => {
                entries.push(Arc::new(entry.clone()));
            },
            FolderEntry::Folder(folder) => {
                let mut entry = folder.clone();
                entry.children = collect_children(folder.id, all_entries);
                entries.push(Arc::new(FolderEntry::Folder(entry)));
            },
        }
    }

    entries
}
