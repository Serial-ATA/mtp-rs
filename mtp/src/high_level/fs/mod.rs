mod device_ext;

pub use device_ext::*;
use std::collections::HashMap;

use crate::communication::SessionId;
use crate::device::storage::id::StorageId;
use crate::device::{Device, PtpIo};
use crate::error::{Error, MtpError};
use crate::object::info::{ObjectInfo, ProtectionStatus};
use crate::object::types::properties::{ObjectFileName, ObjectFormat, ObjectSize, ParentObject};
use crate::object::types::{DateTime, ObjectFormatCode, ObjectHandle, PtpString};

use mtp_spec::communication::response::errors::{InvalidObjectHandle, OperationError};
use std::io::Write;
use std::str::FromStr;
use std::sync::{Arc, Weak};

/// Representation of a file on an MTP-compatible device
///
/// Note that it is **not** guaranteed that a device will support any or all of the operations available
/// on `File`.
#[derive(Clone, Debug)]
pub struct File {
    /// The device-specific ID of the storage where this file lives
    pub storage_id: StorageId,
    pub id: ObjectHandle,
    pub parent: Option<Weak<Folder>>,
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
    /// # async fn main() -> mtp::usb::error::Result<()> {
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
    ) -> Result<std::fs::File, crate::error::Error<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        let object = device.get_object(session_id, self.id).await?;
        let file_data = object.data.data;

        let mut tmp = tempfile::tempfile()?;
        tmp.write_all(&file_data)?;

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
    ) -> Result<Self, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        N: AsRef<str>,
    {
        let name_str = name.as_ref();
        if name_str == self.name {
            return Ok(self.clone());
        }

        if !device
            .object_property_can_be_modified::<ObjectFileName>(session_id, self.format)
            .await?
        {
            return Err(MtpError::UnsupportedOperation.into());
        }

        let name_ptp = PtpString::from_str(name_str)
            .map_err(Into::<MtpError<<D as PtpIo>::TransportError>>::into)?;

        let id = self.id;
        let parent = self.parent.clone();
        let storage_id = self.storage_id;
        let _ = device
            .set_object_prop_value::<ObjectFileName>(session_id, self.id, name_ptp)
            .await?;

        Ok(Self {
            storage_id,
            parent,
            id,
            name: name_str.to_string(),
            size: self.size,
            format: self.format,
            protection_status: self.protection_status,
            date_created: self.date_created,
            date_modified: self.date_modified,
        })
    }

    /// Attempt to copy this file to another directory
    ///
    /// # Errors
    ///
    /// This can fail for a variety of reasons, namely:
    ///
    /// * The device doesn't support copying
    /// * The object no longer exists on the device
    /// * The parent no longer exists on the device
    /// * Any errors from the transport backend
    pub async fn copy<D>(
        &self,
        device: &mut D,
        session_id: SessionId,
        parent: Option<ObjectHandle>,
    ) -> Result<(), MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        <D as PtpIo>::Error: From<Error<<D as PtpIo>::Error>>,
    {
        device
            .copy_object(session_id, self.id, self.storage_id, parent)
            .await?;

        Ok(())
    }

    /// Attempt to move this file
    ///
    /// # Errors
    ///
    /// This can fail for a variety of reasons, namely:
    ///
    /// * The device doesn't support moving
    /// * The object no longer exists on the device
    /// * The parent no longer exists on the device
    /// * Any errors from the transport backend
    pub async fn move_<D>(
        &self,
        device: &mut D,
        session_id: SessionId,
        destination: Arc<Folder>,
    ) -> Result<(), MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        device
            .move_object(session_id, self.id, self.storage_id, Some(destination.id))
            .await?;
        Ok(())
    }

    pub fn object_info(&self) -> Result<ObjectInfo, <PtpString as FromStr>::Err> {
        let filename = self.name.parse()?;
        let Some(parent_object) = self.parent.as_ref().and_then(Weak::upgrade).map(|p| p.id) else {
            todo!()
        };
        Ok(ObjectInfo {
            storage_id: self.storage_id,
            object_format: self.format,
            protection_status: self.protection_status,
            compressed_size: 0,
            thumbnail: None,
            parent_object: Some(parent_object),
            association: None,
            sequence_number: 0,
            filename,
            date_created: self.date_created,
            date_modified: self.date_modified,
            keywords: Default::default(),
        })
    }
}

/// Representation of a folder on an MTP-compatible device
///
/// Note that it is **not** guaranteed that a device will support any or all of the operations available
/// on `Folder`.
#[derive(Clone, Debug)]
pub struct Folder {
    /// The device-specific ID of the storage where this folder lives
    pub id: ObjectHandle,
    pub storage_id: StorageId,
    pub parent: Option<Weak<Folder>>,
    pub name: String,
    pub format: ObjectFormatCode,
    pub protection_status: ProtectionStatus,
    pub date_created: Option<DateTime>,
    pub date_modified: Option<DateTime>,
    pub children: Vec<FolderEntry>,
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
    ) -> Result<Self, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        N: AsRef<str>,
    {
        let name_str = name.as_ref();
        let name = PtpString::from_str(name_str)?;
        let _ = device
            .set_object_prop_value::<ObjectFileName>(session_id, self.id, name)
            .await?;

        Ok(Self {
            storage_id: self.storage_id,
            id: self.id,
            name: name_str.to_string(),
            format: self.format,
            protection_status: self.protection_status,
            date_created: self.date_created,
            date_modified: self.date_modified,
            children: self.children.clone(),
            parent: self.parent.clone(),
        })
    }

    /// Attempt to copy this file to another directory
    ///
    /// # Errors
    ///
    /// This can fail for a variety of reasons, namely:
    ///
    /// * The device doesn't support copying
    /// * The object no longer exists on the device
    /// * The parent no longer exists on the device
    /// * Any errors from the transport backend
    pub async fn copy<D>(
        &self,
        device: &mut D,
        session_id: SessionId,
        parent: Option<ObjectHandle>,
    ) -> Result<(), MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        <D as PtpIo>::Error: From<Error<<D as PtpIo>::Error>>,
    {
        device
            .copy_object(session_id, self.id, self.storage_id, parent)
            .await?;

        Ok(())
    }

    /// Attempt to move this directory
    ///
    /// # Errors
    ///
    /// This can fail for a variety of reasons, namely:
    ///
    /// * The device doesn't support moving
    /// * The object no longer exists on the device
    /// * The parent no longer exists on the device
    /// * Any errors from the transport backend
    pub async fn move_<D>(
        &self,
        device: &mut D,
        session_id: SessionId,
        parent: Option<Arc<Self>>,
    ) -> Result<(), MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        <D as PtpIo>::Error: From<Error<<D as PtpIo>::Error>>,
    {
        device
            .move_object(session_id, self.id, self.storage_id, parent.map(|p| p.id))
            .await?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum FolderEntry {
    File(Arc<File>),
    Folder(Arc<Folder>),
}

impl FolderEntry {
    /// Get the name of this entry
    pub fn name(&self) -> &str {
        match self {
            FolderEntry::File(f) => &f.name,
            FolderEntry::Folder(f) => &f.name,
        }
    }

    /// Get the object handle of this entry
    pub fn handle(&self) -> ObjectHandle {
        match self {
            FolderEntry::File(f) => f.id,
            FolderEntry::Folder(f) => f.id,
        }
    }

    /// Get the storage ID of this entry
    pub fn storage_id(&self) -> StorageId {
        match self {
            FolderEntry::File(f) => f.storage_id,
            FolderEntry::Folder(f) => f.storage_id,
        }
    }

    /// Get the object format of this entry
    pub fn format(&self) -> ObjectFormatCode {
        match self {
            FolderEntry::File(f) => f.format,
            FolderEntry::Folder(f) => f.format,
        }
    }

    /// Get the write-protection status of this entry
    pub fn protection_status(&self) -> ProtectionStatus {
        match self {
            FolderEntry::File(f) => f.protection_status,
            FolderEntry::Folder(f) => f.protection_status,
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
    ) -> Result<Self, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        N: AsRef<str>,
    {
        match self {
            FolderEntry::File(f) => Ok(FolderEntry::File(Arc::new(
                f.rename(device, session_id, name).await?,
            ))),
            FolderEntry::Folder(f) => Ok(FolderEntry::Folder(Arc::new(
                f.rename(device, session_id, name).await?,
            ))),
        }
    }

    /// Attempt to copy this entry to another directory
    ///
    /// # Errors
    ///
    /// See [`File::copy()`] and [`Folder::copy()`]
    pub async fn copy<D>(
        &self,
        device: &mut D,
        session_id: SessionId,
        parent: Option<ObjectHandle>,
    ) -> Result<(), MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        <D as PtpIo>::Error: From<Error<<D as PtpIo>::Error>>,
    {
        match self {
            FolderEntry::File(f) => f.copy(device, session_id, parent).await,
            FolderEntry::Folder(f) => f.copy(device, session_id, parent).await,
        }
    }

    /// Attempt to copy this entry to another directory
    ///
    /// # Errors
    ///
    /// See [`File::move_()`] and [`Folder::move_()`]
    pub async fn move_<D>(
        &self,
        device: &mut D,
        session_id: SessionId,
        parent: Option<Arc<Folder>>,
    ) -> Result<(), MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        <D as PtpIo>::Error: From<Error<<D as PtpIo>::Error>>,
    {
        match self {
            FolderEntry::File(f) => {
                let Some(parent) = parent else {
                    return Err(MtpError::Protocol(OperationError::InvalidObjectHandle(
                        InvalidObjectHandle {},
                    )));
                };

                f.move_(device, session_id, parent).await
            },
            FolderEntry::Folder(f) => f.move_(device, session_id, parent).await,
        }
    }
}

pub struct FileSystem {
    session_id: SessionId,
    storage_id: StorageId,
    pub root: Arc<Folder>,
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
    ) -> Result<Self, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
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
        mut callback: F,
    ) -> Result<Self, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        F: FnMut(ObjectHandle),
    {
        let handles_res = device
            .get_object_handles(session_id, storage_id, None, None)
            .await?;
        let handles = handles_res.data.data;

        let mut raw_data: HashMap<ObjectHandle, (ObjectHandle, FolderEntry)> = HashMap::new();

        for handle in handles {
            callback(handle);

            let parent_id = device
                .get_object_prop_value::<ParentObject>(session_id, handle)
                .await?
                .data
                .data;
            let name_ptp = device
                .get_object_prop_value::<ObjectFileName>(session_id, handle)
                .await?
                .data
                .data;
            let format = device
                .get_object_prop_value::<ObjectFormat>(session_id, handle)
                .await?
                .data
                .data;

            let name = name_ptp.to_string();
            if format == ObjectFormatCode::Association {
                let folder = Folder {
                    id: handle,
                    storage_id,
                    parent: None,
                    name,
                    format,
                    protection_status: ProtectionStatus::ReadOnly,
                    date_created: None,
                    date_modified: None,
                    children: Vec::new(),
                };
                raw_data.insert(handle, (parent_id, FolderEntry::Folder(Arc::new(folder))));
            } else {
                let file = File {
                    id: handle,
                    storage_id,
                    parent: None,
                    name,
                    size: device
                        .get_object_prop_value::<ObjectSize>(session_id, handle)
                        .await?
                        .data
                        .data,
                    format,
                    protection_status: ProtectionStatus::ReadOnly,
                    date_created: None,
                    date_modified: None,
                };
                raw_data.insert(handle, (parent_id, FolderEntry::File(Arc::new(file))));
            }
        }

        let root = Arc::new_cyclic(|weak_root| {
            let mut children = Vec::new();
            Self::assemble_recursive(
                ObjectHandle::NONE,
                weak_root.clone(),
                &mut raw_data,
                &mut children,
            );

            Folder {
                id: ObjectHandle::NONE,
                storage_id,
                parent: None,
                name: "/".to_string(),
                format: ObjectFormatCode::Association,
                protection_status: ProtectionStatus::ReadOnly,
                date_created: None,
                date_modified: None,
                children,
            }
        });

        Ok(Self {
            session_id,
            storage_id,
            root,
        })
    }

    /// Internal recursive helper to build child nodes and link them to parents
    fn assemble_recursive(
        parent_id: ObjectHandle,
        parent_weak: Weak<Folder>,
        raw_data: &mut HashMap<ObjectHandle, (ObjectHandle, FolderEntry)>,
        out_children: &mut Vec<FolderEntry>,
    ) {
        let child_handles: Vec<ObjectHandle> = raw_data
            .iter()
            .filter(|(_, (p, _))| *p == parent_id)
            .map(|(h, _)| *h)
            .collect();

        for h in child_handles {
            let (_, entry) = raw_data.remove(&h).expect("Exists");

            match entry {
                FolderEntry::File(mut file) => {
                    {
                        let file = Arc::get_mut(&mut file).expect("should be valid");
                        file.parent = Some(parent_weak.clone());
                    }

                    out_children.push(FolderEntry::File(file));
                },
                FolderEntry::Folder(folder_arc) => {
                    let assembled_folder = Arc::new_cyclic(|me_weak| {
                        let mut children = Vec::new();
                        Self::assemble_recursive(h, me_weak.clone(), raw_data, &mut children);

                        Folder {
                            id: folder_arc.id,
                            storage_id: folder_arc.storage_id,
                            parent: Some(parent_weak.clone()),
                            name: folder_arc.name.clone(),
                            format: folder_arc.format,
                            protection_status: folder_arc.protection_status,
                            date_created: folder_arc.date_created.clone(),
                            date_modified: folder_arc.date_modified.clone(),
                            children,
                        }
                    });
                    out_children.push(FolderEntry::Folder(assembled_folder));
                },
            }
        }
    }

    /// Refresh the `FileSystem` to match the new state of the device
    ///
    /// Note that the speed of this depends entirely on the device and the numbers of files on the storage.
    /// See [Performance Considerations].
    ///
    /// [Performance Considerations]: https://docs.rs/mtp/latest/mtp/#performance-considerations
    pub async fn refresh<D>(
        &mut self,
        device: &mut D,
    ) -> Result<(), MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
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
    ) -> Result<(), MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        todo!()
        // let objects_response = device
        //     .get_object_handles(self.session_id, self.storage_id, None, None)
        //     .await?;
        // let objects = objects_response.data.data;
        //
        // let mut entries = Vec::with_capacity(objects.len());
        // for object in objects.iter().copied() {
        //     callback(object);
        //
        //     let parent = match device
        //         .get_object_prop_value::<ParentObject>(self.session_id, object)
        //         .await
        //     {
        //         Ok(parent) => {
        //             if parent.data.data == ObjectHandle::NONE {
        //                 None
        //             } else {
        //                 Some(parent.data.data)
        //             }
        //         },
        //         Err(e) => {
        //             log::warn!("Failed to get parent object, skipping: {e}");
        //             continue;
        //         },
        //     };
        //
        //     let name = match device
        //         .get_object_prop_value::<ObjectFileName>(self.session_id, object)
        //         .await
        //     {
        //         Ok(name) => name.data.data,
        //         Err(e) => {
        //             log::warn!("Failed to get object name, skipping: {e}");
        //             continue;
        //         },
        //     };
        //
        //     let format = match device
        //         .get_object_prop_value::<ObjectFormat>(self.session_id, object)
        //         .await
        //     {
        //         Ok(format) => format.data.data,
        //         Err(e) => {
        //             log::warn!("Failed to get object format, skipping: {e}");
        //             continue;
        //         },
        //     };
        //
        //     match parent {
        //         Some(parent) => match format {
        //             ObjectFormatCode::Association => {
        //                 entries.push((
        //                     parent,
        //                     FolderEntry::Folder(Folder {
        //                         storage_id: self.storage_id,
        //                         id: object,
        //                         name: name.to_string(),
        //                         format,
        //                         protection_status: ProtectionStatus::ReadOnly,
        //                         date_created: None,
        //                         date_modified: None,
        //                         children: Vec::new(),
        //                     }),
        //                 ));
        //             },
        //             _ => {
        //                 let size = match device
        //                     .get_object_prop_value::<ObjectSize>(self.session_id, object)
        //                     .await
        //                 {
        //                     Ok(size) => size.data.data,
        //                     Err(e) => {
        //                         log::warn!("Failed to get object size, skipping: {e}");
        //                         continue;
        //                     },
        //                 };
        //
        //                 entries.push((
        //                     parent,
        //                     FolderEntry::File(File {
        //                         storage_id: self.storage_id,
        //                         id: object,
        //                         parent,
        //                         name: name.to_string(),
        //                         size,
        //                         format,
        //                         protection_status: ProtectionStatus::ReadOnly,
        //                         date_created: None,
        //                         date_modified: None,
        //                     }),
        //                 ));
        //             },
        //         },
        //         None => {
        //             self.contents.push(Folder {
        //                 storage_id: self.storage_id,
        //                 id: object,
        //                 name: name.to_string(),
        //                 format,
        //                 protection_status: ProtectionStatus::ReadOnly,
        //                 date_created: None,
        //                 date_modified: None,
        //                 children: Vec::new(),
        //             });
        //         },
        //     }
        // }
        //
        // let root_ids = self.contents.iter().map(|dir| dir.id).collect::<Vec<_>>();
        // for (index, root) in root_ids.iter().enumerate() {
        //     let children = collect_children(*root, &entries);
        //     self.contents[index].children = children;
        // }
        //
        // Ok(())
    }
}

// fn collect_children(
//     root: ObjectHandle,
//     all_entries: &[(ObjectHandle, FolderEntry)],
// ) -> Vec<Arc<FolderEntry>> {
//     let mut entries = Vec::new();
//     for (parent, entry) in all_entries.iter() {
//         if *parent != root {
//             continue;
//         }
//
//         match entry {
//             FolderEntry::File(_) => {
//                 entries.push(Arc::new(entry.clone()));
//             },
//             FolderEntry::Folder(folder) => {
//                 let mut entry = folder.clone();
//                 entry.children = collect_children(folder.id, all_entries);
//                 entries.push(Arc::new(FolderEntry::Folder(entry)));
//             },
//         }
//     }
//
//     entries
// }
