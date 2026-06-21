//! Filesystem abstractions for MTP
//!
//! High-level utilities for interacting with MTP devices as if they were hierarchical filesystems

mod device_ext;

pub use device_ext::*;

use crate::device::storage::StorageId;
use crate::device::{Device, PtpIo};
use crate::error::{Error, MtpError};
use crate::object::properties::ObjectFileName;
use crate::object::{
    DateTime, ObjectFormatCode, ObjectHandle, ObjectInfo, ProtectionStatus, PtpString,
};

use futures::StreamExt;
use mtp_spec::communication::event::Event;
use mtp_spec::communication::operation::{BaseOperation, Operation};
use mtp_spec::device::session::MtpSession;
use mtp_spec::object::properties::{ObjectPropertyCode, ObjectSize};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::io::{ErrorKind, Write};
use std::str::FromStr;
use std::sync::{Arc, Weak};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{RwLock, RwLockWriteGuard};

/// Representation of a file on an MTP-compatible device
///
/// Note that it is **not** guaranteed that a device will support any or all of the operations available
/// on `File`.
pub struct File<D> {
    fs: Weak<FileSystem<D>>,
    storage_id: StorageId,
    id: ObjectHandle,
    parent_id: ObjectHandle,
    name: String,
    size: u64,
    format: ObjectFormatCode,
    protection_status: ProtectionStatus,
    date_modified: Option<DateTime>,
    date_created: Option<DateTime>,
}

impl<D> Debug for File<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File")
            .field("storage_id", &self.storage_id)
            .field("id", &self.id)
            .field("parent_id", &self.parent_id)
            .field("name", &self.name)
            .field("size", &self.size)
            .field("format", &self.format)
            .field("protection_status", &self.protection_status)
            .field("date_modified", &self.date_modified)
            .field("date_created", &self.date_created)
            .finish_non_exhaustive()
    }
}

// Getters
impl<D> File<D> {
    /// The device-specific ID of the storage where this file lives
    pub fn storage_id(&self) -> StorageId {
        self.storage_id
    }

    /// The device-specific ID of this file
    pub fn id(&self) -> ObjectHandle {
        self.id
    }

    /// The device-specific ID of the parent folder
    pub fn parent(&self) -> ObjectHandle {
        self.parent_id
    }

    /// The name of the file as it appears on the device
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// The size of the file in bytes
    pub fn size(&self) -> u64 {
        self.size
    }

    /// The format of the file, if it can be determined
    pub fn format(&self) -> ObjectFormatCode {
        self.format
    }

    /// The write-protection status of the file
    pub fn protection_status(&self) -> ProtectionStatus {
        self.protection_status
    }

    /// The date and time the file was last modified, if available
    ///
    /// NOTE: This is oftentimes *not* available
    pub fn date_modified(&self) -> Option<DateTime> {
        self.date_modified
    }

    /// The date and time the file was created, if available
    ///
    /// NOTE: This is oftentimes *not* available
    pub fn date_created(&self) -> Option<DateTime> {
        self.date_created
    }
}

impl<D> File<D>
where
    D: Device,
{
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
    /// use futures::stream::StreamExt;
    /// use mtp::high_level::fs::{FileSystem, FolderEntry};
    /// use mtp::high_level::storages::SessionStorageExt;
    /// use mtp::usb::device_list;
    /// use std::io::Read;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> mtp::usb::error::Result<()> {
    /// // Get the first MTP-eligible device
    /// let device = device_list().await?.next().await.expect("No devices");
    /// let mut session = device?.open().await?;
    ///
    /// // Get whatever the first storage happens to be
    /// let storages = session.storages().await?;
    /// let storage = storages.first().expect("no storages");
    ///
    /// // Load the storage and find the first file
    /// let fs = FileSystem::load(&mut session, storage.id).await?;
    /// for child in &fs.root.children {
    ///     let FolderEntry::File(file) = child else {
    ///         continue;
    ///     };
    ///
    ///     // Print out whatever the first file's contents happen to be
    ///     let mut open_file = file.open(&mut session).await?;
    ///
    ///     println!("Contents of: {}", file.name);
    ///
    ///     let mut contents = Vec::new();
    ///     open_file.read_to_end(&mut contents)?;
    ///
    ///     println!("{:X?}", contents);
    ///     break;
    /// }
    ///
    /// # Ok(()) }
    /// ```
    pub async fn open(
        &self,
    ) -> Result<std::fs::File, crate::error::Error<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        let object = self.fs().session.get_object(self.id).await?;
        let file_data = object.data.data;

        let mut tmp = tempfile::tempfile()?;
        tmp.write_all(&file_data)?;

        Ok(tmp)
    }

    /// Read a portion of the file
    ///
    /// # Errors
    ///
    /// * The device doesn't support [`GetPartialObject`]
    ///
    /// [`GetPartialObject`]: crate::communication::operation::GetPartialObject
    pub async fn read_at(
        &self,
        offset: u32,
        len: u32,
    ) -> Result<Vec<u8>, Error<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        let fs = self.fs();
        if fs.capabilities.supports_partial_read {
            let response = fs.session.get_partial_object(self.id, offset, len).await?;
            return Ok(response.data.data);
        }

        Err(MtpError::UnsupportedOperation.into())
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
    pub async fn rename<N>(
        &self,
        name: N,
    ) -> Result<Arc<Self>, MtpError<<D as PtpIo>::TransportError>>
    where
        N: AsRef<str>,
    {
        let name_str = name.as_ref();

        // Return a fresh copy of the state if the name is unchanged
        if name_str == self.name {
            return Ok(Arc::new(Self {
                fs: self.fs.clone(),
                storage_id: self.storage_id,
                id: self.id,
                parent_id: self.parent_id,
                name: self.name.clone(),
                size: self.size,
                format: self.format,
                protection_status: self.protection_status,
                date_modified: self.date_modified,
                date_created: self.date_created,
            }));
        }

        let fs = self.fs();
        if !fs
            .session
            .object_property_can_be_modified::<ObjectFileName>(self.format)
            .await?
        {
            return Err(MtpError::UnsupportedOperation);
        }

        let name_ptp = PtpString::from_str(name_str)
            .map_err(Into::<MtpError<<D as PtpIo>::TransportError>>::into)?;

        let _ = fs
            .session
            .set_object_prop_value::<ObjectFileName>(self.id, name_ptp)
            .await?;

        Ok(Arc::new(Self {
            fs: self.fs.clone(),
            storage_id: self.storage_id,
            id: self.id,
            parent_id: self.parent_id,
            name: name_str.to_string(),
            size: self.size,
            format: self.format,
            protection_status: self.protection_status,
            date_modified: self.date_modified,
            date_created: self.date_created,
        }))
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
    pub async fn copy(
        &self,
        parent: Option<ObjectHandle>,
    ) -> Result<ObjectHandle, MtpError<<D as PtpIo>::TransportError>> {
        let new_handle = self
            .fs()
            .session
            .copy_object(self.id, self.storage_id, parent)
            .await?
            .data
            .data;

        Ok(new_handle)
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
    pub async fn move_(
        &self,
        new_parent: &Folder<D>,
    ) -> Result<Arc<Self>, MtpError<<D as PtpIo>::TransportError>> {
        let fs = self.fs();
        fs.session
            .move_object(self.id, self.storage_id, Some(new_parent.id))
            .await?;

        let updated_file = Arc::new(Self {
            fs: self.fs.clone(),
            storage_id: self.storage_id,
            id: self.id,
            parent_id: new_parent.id,
            name: self.name.clone(),
            size: self.size,
            format: self.format,
            protection_status: self.protection_status,
            date_modified: self.date_modified,
            date_created: self.date_created,
        });

        let entry = FolderEntry::File(updated_file.clone());
        let inode_lock = fs.inode_map.read().await;

        if let Some(FolderEntry::Folder(old_parent)) = inode_lock.get(&self.parent_id) {
            old_parent.children.write().await.remove(&self.name);
        }
        if let Some(FolderEntry::Folder(new_parent_obj)) = inode_lock.get(&new_parent.id) {
            new_parent_obj
                .children
                .write()
                .await
                .insert(self.name.clone(), entry.clone());
        }

        drop(inode_lock);
        fs.inode_map.write().await.insert(self.id, entry);

        Ok(updated_file)
    }

    /// Attempt to create an [`ObjectInfo`] instance for this file
    ///
    /// # Errors
    ///
    /// * See [`PtpString::from_str()`]
    pub fn object_info(&self) -> Result<ObjectInfo, <PtpString as FromStr>::Err> {
        let filename = PtpString::from_str(&self.name)?;

        Ok(ObjectInfo {
            storage_id: self.storage_id,
            object_format: self.format,
            protection_status: self.protection_status,
            compressed_size: self.size as u32,
            thumbnail: None,
            image_details: None,
            parent_object: Some(self.parent_id),
            association: None,
            sequence_number: 0,
            filename,
            date_created: self.date_created,
            date_modified: self.date_modified,
            keywords: PtpString::default(),
        })
    }

    fn fs(&self) -> Arc<FileSystem<D>> {
        self.fs.upgrade().expect("FS dropped")
    }
}

/// Representation of a folder on an MTP-compatible device
///
/// Note that it is **not** guaranteed that a device will support any or all of the operations available
/// on `Folder`.
pub struct Folder<D> {
    fs: Weak<FileSystem<D>>,
    storage_id: StorageId,
    id: ObjectHandle,
    parent_id: ObjectHandle,
    name: String,
    format: ObjectFormatCode,
    protection_status: ProtectionStatus,
    date_created: Option<DateTime>,
    date_modified: Option<DateTime>,
    children: RwLock<HashMap<String, FolderEntry<D>>>,
}

impl<D> Debug for Folder<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Folder")
            .field("storage_id", &self.storage_id)
            .field("id", &self.id)
            .field("parent_id", &self.parent_id)
            .field("name", &self.name)
            .field("format", &self.format)
            .field("protection_status", &self.protection_status)
            .field("date_created", &self.date_created)
            .field("date_modified", &self.date_modified)
            .finish_non_exhaustive()
    }
}

// Getters
impl<D> Folder<D> {
    /// The device-specific ID of the storage where this folder lives
    pub fn storage_id(&self) -> StorageId {
        self.storage_id
    }

    /// The device-specific ID of this folder
    pub fn id(&self) -> ObjectHandle {
        self.id
    }

    /// The device-specific ID of the parent folder
    pub fn parent(&self) -> ObjectHandle {
        self.parent_id
    }

    /// The name of the folder as it appears on the device
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// The format of the folder, if it can be determined
    ///
    /// This should always be `Association`
    pub fn format(&self) -> ObjectFormatCode {
        self.format
    }

    /// The write-protection status of the folder
    pub fn protection_status(&self) -> ProtectionStatus {
        self.protection_status
    }

    /// The date and time the folder was last modified, if available
    ///
    /// NOTE: This is oftentimes *not* available
    pub fn date_modified(&self) -> Option<DateTime> {
        self.date_modified
    }

    /// The date and time the folder was created, if available
    ///
    /// NOTE: This is oftentimes *not* available
    pub fn date_created(&self) -> Option<DateTime> {
        self.date_created
    }

    /// Call the function `f` with immutable access to this folder's children
    pub async fn with_children<F, Fut>(&self, f: F) -> Fut::Output
    where
        F: FnOnce(&HashMap<String, FolderEntry<D>>) -> Fut,
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let children = self.children.read().await;
        f(&*children).await
    }
}

impl<D> Folder<D>
where
    D: Device,
{
    /// Attempt to rename this folder on the device
    ///
    /// # Errors
    ///
    /// This can fail for a variety of reasons, namely:
    ///
    /// * The device doesn't support renaming
    /// * The object no longer exists on the device
    /// * Any errors from the transport backend
    pub async fn rename<N>(
        &self,
        name: N,
    ) -> Result<Arc<Self>, MtpError<<D as PtpIo>::TransportError>>
    where
        N: AsRef<str>,
    {
        let name_str = name.as_ref();
        let name_ptp = PtpString::from_str(name_str)?;

        let fs = self.fs();
        fs.session
            .set_object_prop_value::<ObjectFileName>(self.id, name_ptp)
            .await?;

        let updated_folder = Arc::new(Self {
            fs: self.fs.clone(),
            storage_id: self.storage_id,
            id: self.id,
            parent_id: self.parent_id,
            name: name_str.to_string(),
            format: self.format,
            protection_status: self.protection_status,
            date_created: self.date_created,
            date_modified: self.date_modified,
            children: RwLock::new(self.children.read().await.clone()),
        });

        let entry = FolderEntry::Folder(updated_folder.clone());
        fs.inode_map.write().await.insert(self.id, entry.clone());

        if let Some(FolderEntry::Folder(parent)) = fs.inode_map.read().await.get(&self.parent_id) {
            let mut children = parent.children.write().await;
            children.remove(&self.name);
            children.insert(name_str.to_string(), entry);
        }

        Ok(updated_folder)
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
    pub async fn copy(
        &self,
        parent: Option<ObjectHandle>,
    ) -> Result<ObjectHandle, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        let new_handle = self
            .fs()
            .session
            .copy_object(self.id, self.storage_id, parent)
            .await?
            .data
            .data;

        Ok(new_handle)
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
    pub async fn move_(
        &self,
        new_parent: &Folder<D>,
    ) -> Result<Arc<Self>, MtpError<<D as PtpIo>::TransportError>> {
        let fs = self.fs();
        fs.session
            .move_object(self.id, self.storage_id, Some(new_parent.id))
            .await?;

        let updated_folder = Arc::new(Self {
            fs: self.fs.clone(),
            storage_id: self.storage_id,
            id: self.id,
            parent_id: new_parent.id,
            name: self.name.clone(),
            format: self.format,
            protection_status: self.protection_status,
            date_created: self.date_created,
            date_modified: self.date_modified,
            children: RwLock::new(self.children.read().await.clone()),
        });

        let entry = FolderEntry::Folder(updated_folder.clone());
        let inode_lock = fs.inode_map.read().await;

        if let Some(FolderEntry::Folder(old_parent)) = inode_lock.get(&self.parent_id) {
            old_parent.children.write().await.remove(&self.name);
        }

        if let Some(FolderEntry::Folder(new_parent)) = inode_lock.get(&new_parent.id) {
            new_parent
                .children
                .write()
                .await
                .insert(self.name.clone(), entry.clone());
        }

        drop(inode_lock);
        fs.inode_map.write().await.insert(self.id, entry);

        Ok(updated_folder)
    }

    /// Create a new file within this `Folder`
    ///
    /// # Errors
    ///
    /// See [`SessionFsExt::create()`]
    pub async fn create_file<N>(
        &self,
        name: N,
        format: ObjectFormatCode,
        data: Vec<u8>,
    ) -> Result<Arc<File<D>>, MtpError<<D as PtpIo>::TransportError>>
    where
        N: AsRef<str> + Send,
    {
        let file = self
            .fs()
            .session
            .create(Some(self), name.as_ref(), format, data)
            .await?;
        let file = Arc::new(file);
        self.children
            .write()
            .await
            .insert(name.as_ref().to_string(), FolderEntry::File(file.clone()));

        Ok(file)
    }

    /// Remove a child entry from this folder by name
    ///
    /// This will remove the child both from the [`FileSystem`] and the device.
    ///
    /// # Errors
    ///
    /// * No child exists with the given name
    /// * The [`DeleteObject`] operation failed
    ///   * Note that in this case, it is assumed the entry still exists on the device
    ///
    /// [`DeleteObject`]: crate::communication::operation::DeleteObject
    pub async fn remove_child(&self, name: &str) -> Result<(), FileSystemError<D>> {
        let mut children = self.children.write().await;
        let Some(entry) = children.remove(name) else {
            return Err(FileSystemError::Io(std::io::Error::from(
                ErrorKind::NotFound,
            )));
        };

        match self.fs().session.delete_object(entry.handle(), None).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Assume the entry is still on the device?
                children.insert(name.to_string(), entry);
                Err(FileSystemError::Mtp(e))
            },
        }
    }

    /// Find an entry within this folder by name
    pub async fn find(&self, name: &str) -> Option<FolderEntry<D>> {
        self.children.read().await.get(name).cloned()
    }

    fn fs(&self) -> Arc<FileSystem<D>> {
        self.fs.upgrade().expect("FS dropped")
    }
}

/// An entry in a [`Folder`]
pub enum FolderEntry<D> {
    /// A [`File`] entry
    File(Arc<File<D>>),
    /// A [`Folder`] entry
    Folder(Arc<Folder<D>>),
}

impl<D> Debug for FolderEntry<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FolderEntry::File(file) => f.debug_tuple("File").field(&file).finish(),
            FolderEntry::Folder(folder) => f.debug_tuple("Folder").field(&folder).finish(),
        }
    }
}

impl<D> Clone for FolderEntry<D> {
    fn clone(&self) -> Self {
        match self {
            FolderEntry::File(file) => FolderEntry::File(Arc::clone(file)),
            FolderEntry::Folder(folder) => FolderEntry::Folder(Arc::clone(folder)),
        }
    }
}

impl<D> FolderEntry<D>
where
    D: Device,
{
    fn new(
        fs: Weak<FileSystem<D>>,
        handle: ObjectHandle,
        storage_id: StorageId,
        info: &ObjectInfo,
    ) -> Self {
        let name = info.filename.to_string();
        let parent_id = info.parent_object.unwrap_or(ObjectHandle::NONE);
        if info.object_format == ObjectFormatCode::Association {
            FolderEntry::Folder(Arc::new(Folder {
                fs,
                id: handle,
                storage_id,
                parent_id,
                name,
                format: info.object_format,
                protection_status: info.protection_status,
                date_created: info.date_created,
                date_modified: info.date_modified,
                children: RwLock::new(HashMap::new()),
            }))
        } else {
            FolderEntry::File(Arc::new(File {
                fs,
                id: handle,
                storage_id,
                parent_id,
                name,
                size: u64::from(info.compressed_size),
                format: info.object_format,
                protection_status: info.protection_status,
                date_created: info.date_created,
                date_modified: info.date_modified,
            }))
        }
    }

    fn fs(&self) -> Arc<FileSystem<D>> {
        match self {
            FolderEntry::File(f) => f.fs.upgrade().expect("FS dropped"),
            FolderEntry::Folder(f) => f.fs.upgrade().expect("FS dropped"),
        }
    }

    /// Get the virtual absolute path of the entry
    ///
    /// The path will be constructed from the device-provided object names at the depth
    /// it chooses to provide. The result may not be the absolute path on the device.
    pub async fn path(&self) -> String {
        let fs = self.fs();
        let map = fs.inode_map.read().await;
        self.path_inner(&map)
    }

    fn path_inner(&self, map: &INodeMap<D>) -> String {
        let mut components = Vec::new();
        let mut cur = Some(self.clone());
        while let Some(entry) = cur {
            components.push(entry.name().to_string());
            let parent = entry.parent();
            if parent == ObjectHandle::NONE {
                break;
            }

            cur = map.get(&parent).cloned();
        }
        components.reverse();
        components.join("/")
    }

    /// Get the name of this entry
    pub fn name(&self) -> &str {
        match self {
            FolderEntry::File(f) => &f.name,
            FolderEntry::Folder(f) => &f.name,
        }
    }

    fn set_name(&mut self, name: String) {
        match self {
            FolderEntry::File(f) => Arc::get_mut(f).unwrap().name = name,
            FolderEntry::Folder(f) => Arc::get_mut(f).unwrap().name = name,
        }
    }

    /// Get the object handle of the parent
    pub fn parent(&self) -> ObjectHandle {
        match self {
            FolderEntry::File(f) => f.parent_id,
            FolderEntry::Folder(f) => f.parent_id,
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

    fn set_format(&mut self, format: ObjectFormatCode) {
        match self {
            FolderEntry::File(f) => Arc::get_mut(f).unwrap().format = format,
            FolderEntry::Folder(f) => Arc::get_mut(f).unwrap().format = format,
        }
    }

    /// Get the write-protection status of this entry
    pub fn protection_status(&self) -> ProtectionStatus {
        match self {
            FolderEntry::File(f) => f.protection_status,
            FolderEntry::Folder(f) => f.protection_status,
        }
    }

    fn set_protection_status(&mut self, protection_status: ProtectionStatus) {
        match self {
            FolderEntry::File(f) => Arc::get_mut(f).unwrap().protection_status = protection_status,
            FolderEntry::Folder(f) => {
                Arc::get_mut(f).unwrap().protection_status = protection_status
            },
        }
    }

    fn set_size(&mut self, size: u64) {
        if let FolderEntry::File(f) = self {
            Arc::get_mut(f).unwrap().size = size;
        }
    }

    fn set_date_created(&mut self, date: Option<DateTime>) {
        match self {
            FolderEntry::File(f) => Arc::get_mut(f).unwrap().date_created = date,
            FolderEntry::Folder(f) => Arc::get_mut(f).unwrap().date_created = date,
        }
    }

    fn set_date_modified(&mut self, date: Option<DateTime>) {
        match self {
            FolderEntry::File(f) => Arc::get_mut(f).unwrap().date_modified = date,
            FolderEntry::Folder(f) => Arc::get_mut(f).unwrap().date_modified = date,
        }
    }

    /// Attempt to rename this entry on the device
    ///
    /// # Errors
    ///
    /// See [`File::rename()`] and [`Folder::rename()`]
    pub async fn rename<N>(&self, name: N) -> Result<Self, MtpError<<D as PtpIo>::TransportError>>
    where
        N: AsRef<str>,
    {
        match self {
            FolderEntry::File(f) => Ok(FolderEntry::File(f.rename(name).await?)),
            FolderEntry::Folder(f) => Ok(FolderEntry::Folder(f.rename(name).await?)),
        }
    }

    /// Attempt to copy this entry to another directory
    ///
    /// # Errors
    ///
    /// See [`File::copy()`] and [`Folder::copy()`]
    pub async fn copy(
        &self,
        parent: Option<ObjectHandle>,
    ) -> Result<ObjectHandle, MtpError<<D as PtpIo>::TransportError>> {
        match self {
            FolderEntry::File(f) => f.copy(parent).await,
            FolderEntry::Folder(f) => f.copy(parent).await,
        }
    }

    /// Attempt to copy this entry to another directory
    ///
    /// # Errors
    ///
    /// See [`File::move_()`] and [`Folder::move_()`]
    pub async fn move_(
        &self,
        new_parent: &Folder<D>,
    ) -> Result<FolderEntry<D>, MtpError<<D as PtpIo>::TransportError>> {
        match self {
            FolderEntry::File(f) => Ok(FolderEntry::File(f.move_(new_parent).await?)),
            FolderEntry::Folder(f) => Ok(FolderEntry::Folder(f.move_(new_parent).await?)),
        }
    }
}

/// The advanced capabilities of a device
#[derive(Clone, Debug)]
struct DeviceCapabilities {
    /// Whether the device supports the [`GetPartialObject`] operation
    ///
    /// [`GetPartialObject`]: crate::communication::operation::GetPartialObject
    pub supports_partial_read: bool,
}

impl DeviceCapabilities {
    async fn query<D: Device>(
        session: &MtpSession<D>,
    ) -> Result<Self, MtpError<<D as PtpIo>::TransportError>> {
        let info = session.get_device_info().await?;
        let ops = info.data.data.operations_supported.as_slice();

        Ok(Self {
            supports_partial_read: ops.contains(&Operation::Base(BaseOperation::GetPartialObject)),
        })
    }
}

/// Events emitted by the reactive FileSystem
#[derive(Clone, Debug)]
pub enum FsEvent<D> {
    /// An entry was added
    Added(FolderEntry<D>),
    /// An entry was removed
    Removed(FolderEntry<D>),
    /// A property of the entry changed
    Refreshed(FolderEntry<D>),
}

type INodeMap<D> = HashMap<ObjectHandle, FolderEntry<D>>;
type INodeMapGuard<'a, D> = RwLockWriteGuard<'a, INodeMap<D>>;

/// A filesystem abstraction for MTP devices
pub struct FileSystem<D> {
    capabilities: DeviceCapabilities,
    storage_id: StorageId,
    session: MtpSession<D>,
    root: RwLock<Option<Arc<Folder<D>>>>,
    inode_map: RwLock<INodeMap<D>>,
}

/// Errors that can occur while interating with a [`FileSystem`]
pub enum FileSystemError<D>
where
    D: Device,
{
    /// An error occurred within the MTP protocol
    Mtp(MtpError<<D as PtpIo>::TransportError>),
    /// An I/O error occurred within the filesystem
    Io(std::io::Error),
}

impl<D> From<std::io::Error> for FileSystemError<D>
where
    D: Device,
{
    fn from(err: std::io::Error) -> Self {
        FileSystemError::Io(err)
    }
}

impl<D> Debug for FileSystemError<D>
where
    D: Device,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileSystemError::Mtp(err) => Debug::fmt(err, f),
            FileSystemError::Io(err) => Debug::fmt(err, f),
        }
    }
}

impl<D> Display for FileSystemError<D>
where
    D: Device,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileSystemError::Mtp(err) => Display::fmt(err, f),
            FileSystemError::Io(err) => Display::fmt(err, f),
        }
    }
}

impl<D> std::error::Error for FileSystemError<D> where D: Device {}

impl<D> FileSystem<D>
where
    D: Device,
{
    /// Create a new [`File`] at the specified path
    ///
    /// # Errors
    ///
    /// * Not all parent folders exist
    /// * Not all parent path segments are [`Folder`]s
    /// * See [`Folder::create_file()`]
    pub async fn create(
        &self,
        path: impl AsRef<str>,
        format: ObjectFormatCode,
        data: Vec<u8>,
    ) -> Result<Arc<File<D>>, FileSystemError<D>> {
        let path = path.as_ref();
        let (parent_path, file_name) = path
            .rsplit_once('/')
            .ok_or(std::io::Error::from(ErrorKind::InvalidInput))?;

        let lookup_path = if parent_path.is_empty() {
            "/"
        } else {
            parent_path
        };
        let parent_entry = self
            .find(lookup_path)
            .await
            .ok_or(std::io::Error::from(ErrorKind::NotFound))?;

        let FolderEntry::Folder(parent) = parent_entry else {
            return Err(std::io::Error::from(ErrorKind::NotADirectory).into());
        };

        parent
            .create_file(file_name, format, data)
            .await
            .map_err(FileSystemError::Mtp)
    }

    /// Find an entry by its path
    pub async fn find(&self, path: impl AsRef<str>) -> Option<FolderEntry<D>> {
        let path = path.as_ref();

        // Nothing to do with non-absolute paths
        if !path.starts_with('/') {
            return None;
        }

        let root = self.root.read().await.clone()?;
        let mut current = FolderEntry::Folder(root);

        for component in path.split('/').filter(|c| !c.is_empty()) {
            if let FolderEntry::Folder(folder) = current {
                current = folder.find(component).await?;
            } else {
                return None;
            }
        }

        Some(current)
    }

    /// Attempt to remove an entry by its handle
    ///
    /// NOTE: This will only remove it from the `FileSystem`, the device will not be modified.
    pub async fn remove(&self, handle: ObjectHandle) -> bool {
        self.remove_entry(handle).await.is_some()
    }

    /// Attempt to add an entry by its handle
    ///
    /// Returns the [`FolderEntry`] if it was created, otherwise the `FileSystem`
    /// is unchanged.
    ///
    /// # Errors
    ///
    /// * [`MtpSession::get_object_info()`]
    pub async fn add(
        self: &Arc<Self>,
        handle: ObjectHandle,
    ) -> Result<Option<FolderEntry<D>>, MtpError<<D as PtpIo>::TransportError>> {
        Self::object_added(self.clone(), &self.session, handle).await
    }

    /// Call the function `f` with immutable access to the [`FileSystem`] root
    #[allow(clippy::missing_panics_doc)]
    pub async fn with_root<F, Fut>(&self, f: F) -> Fut::Output
    where
        F: FnOnce(Arc<Folder<D>>) -> Fut,
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        let guard = self.root.read().await;
        f((*guard).clone().expect("root should exist")).await
    }

    /// Load a `FileSystem` from the given `storage_id`
    ///
    /// Note that the speed of this depends entirely on the device and the numbers of files on the storage.
    /// See [Performance Considerations].
    ///
    /// [Performance Considerations]: https://docs.rs/mtp/latest/mtp/#performance-considerations
    ///
    /// This will also spawn a background process to update the filesystem based on the notifications
    /// the device sends (if any).
    ///
    /// # Errors
    ///
    /// See [`Self::load_with_callback()`]
    pub async fn load(
        session: MtpSession<D>,
        storage_id: StorageId,
    ) -> Result<
        (Arc<Self>, tokio::sync::mpsc::UnboundedReceiver<FsEvent<D>>),
        MtpError<<D as PtpIo>::TransportError>,
    >
    where
        D: Device + 'static,
    {
        Self::load_with_callback(session, storage_id, |_| {}).await
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
    ///
    /// This will also spawn a background process to update the filesystem based on the notifications
    /// the device sends (if any).
    ///
    /// # Errors
    ///
    /// This will fail if, at any point, the device fails any of the following operations:
    ///
    /// * [`GetDeviceInfo`]
    /// * [`GetObjectHandles`]
    /// * [`GetObjectInfo`]
    ///
    /// [`GetDeviceInfo`]: crate::communication::operation::GetDeviceInfo
    /// [`GetObjectHandles`]: crate::communication::operation::GetObjectHandles
    /// [`GetObjectInfo`]: crate::communication::operation::GetObjectInfo
    pub async fn load_with_callback<F>(
        session: MtpSession<D>,
        storage_id: StorageId,
        callback: F,
    ) -> Result<
        (Arc<Self>, tokio::sync::mpsc::UnboundedReceiver<FsEvent<D>>),
        MtpError<<D as PtpIo>::TransportError>,
    >
    where
        D: Device + 'static,
        F: FnMut(ObjectHandle),
    {
        let capabilities = DeviceCapabilities::query(&session).await?;

        let fs = Arc::new(Self {
            session: session.clone(),
            capabilities,
            storage_id,
            root: RwLock::new(None),
            inode_map: RwLock::new(HashMap::new()),
        });

        let fs_weak = Arc::downgrade(&fs);

        let handles_res = session.get_object_handles(storage_id, None, None).await?;
        let handles = handles_res.data.data.as_slice();

        let mut flat_map: HashMap<ObjectHandle, FolderEntry<D>> =
            HashMap::with_capacity(handles.len());

        Self::load_iterative(
            &session,
            fs_weak.clone(),
            handles,
            storage_id,
            &mut flat_map,
            callback,
        )
        .await?;

        let root = Arc::new(Folder {
            fs: fs_weak,
            storage_id,
            id: ObjectHandle::NONE,
            parent_id: ObjectHandle::NONE,
            name: "/".to_string(),
            format: ObjectFormatCode::Association,
            protection_status: ProtectionStatus::ReadOnly,
            date_created: None,
            date_modified: None,
            children: RwLock::new(HashMap::new()),
        });

        flat_map.insert(ObjectHandle::NONE, FolderEntry::Folder(root.clone()));

        let entries: Vec<FolderEntry<D>> = flat_map.values().cloned().collect();
        for entry in entries {
            if entry.handle() == ObjectHandle::NONE {
                continue;
            }

            if let Some(FolderEntry::Folder(parent)) = flat_map.get(&entry.parent()) {
                parent
                    .children
                    .write()
                    .await
                    .insert(entry.name().to_string(), entry);
            } else {
                root.children
                    .write()
                    .await
                    .insert(entry.name().to_string(), entry);
            }
        }

        *fs.inode_map.write().await = flat_map;
        *fs.root.write().await = Some(root);

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let session = session.clone();
        let fs_weak = Arc::downgrade(&fs);
        tokio::task::spawn(async move {
            Self::event_loop(session, fs_weak, tx).await;
        });

        Ok((fs, rx))
    }

    async fn load_iterative<F>(
        session: &MtpSession<D>,
        fs: Weak<FileSystem<D>>,
        handles: &[ObjectHandle],
        storage_id: StorageId,
        flat_map: &mut HashMap<ObjectHandle, FolderEntry<D>>,
        mut callback: F,
    ) -> Result<(), MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        F: FnMut(ObjectHandle),
    {
        for &handle in handles {
            callback(handle);

            let info = session.get_object_info(handle).await?.data.data;
            let entry = FolderEntry::new(fs.clone(), handle, storage_id, &info);
            flat_map.insert(handle, entry);
        }
        Ok(())
    }
}

// Event monitor stuff
impl<D> FileSystem<D>
where
    D: Device,
{
    /// Event monitor loop for devices that support it
    ///
    /// This lets us keep the FS up to date without doing full rescans.
    async fn event_loop(
        session: MtpSession<D>,
        fs_weak: Weak<Self>,
        tx: UnboundedSender<FsEvent<D>>,
    ) {
        let mut events = session.event_stream();
        while let Some(event_res) = events.next().await {
            let Some(fs) = fs_weak.upgrade() else { break };

            // It seems that, more often than not, devices will simply `ObjectRemoved` + `ObjectAdded`
            // for any property modifications.
            match event_res {
                Ok(Event::ObjectAdded { object_handle }) => {
                    match Self::object_added(fs, &session, object_handle).await {
                        Ok(Some(entry)) => {
                            let _ = tx.send(FsEvent::Added(entry));
                        },
                        Ok(None) => {},
                        Err(e) => tracing::error!("Failed to add new object: {e}"),
                    }
                },
                Ok(Event::ObjectRemoved { object_handle }) => {
                    let Some(entry) = fs.remove_entry(object_handle).await else {
                        // Entry doesn't exist in our fs, maybe it's on another storage?
                        continue;
                    };
                    let _ = tx.send(FsEvent::Removed(entry));
                },
                Ok(Event::ObjectPropChanged { object, prop_code }) => {
                    Self::object_prop_changed(&session, fs, &tx, object, prop_code).await
                },
                Ok(Event::ObjectInfoChanged { object: _ }) => {
                    tracing::warn!("TODO: Handle object info change event")
                },
                Ok(_) => {},
                Err(e) => tracing::error!("MTP Event stream error: {e}"),
            }
        }

        tracing::info!("MTP FileSystem event loop exited.");
    }

    async fn object_prop_changed(
        session: &MtpSession<D>,
        fs: Arc<Self>,
        tx: &UnboundedSender<FsEvent<D>>,
        handle: ObjectHandle,
        prop: ObjectPropertyCode,
    ) {
        {
            let map = fs.inode_map.read().await;
            if !map.contains_key(&handle) {
                return;
            }
        }

        let info = match session.get_object_info(handle).await {
            Ok(info_res) => info_res.data.data,
            Err(e) => {
                tracing::error!("Failed to fetch info on prop change: {e}");
                return;
            },
        };

        let mut map = fs.inode_map.write().await;
        if info.storage_id != fs.storage_id || prop == ObjectPropertyCode::StorageId {
            drop(map);

            // No longer ours to manage
            if let Some(entry) = fs.remove_entry(handle).await {
                let _ = tx.send(FsEvent::Removed(entry));
            }

            return;
        }

        let Some(entry) = map.get_mut(&handle) else {
            // It's on our storage, but we don't know about it?
            return;
        };

        let entry_clone = entry.clone();
        match prop {
            ObjectPropertyCode::ObjectFileName => {
                let _ = entry;
                fs.rename_entry(&mut map, handle, info.filename.to_string())
                    .await;
            },
            ObjectPropertyCode::ObjectFormat => entry.set_format(info.object_format),
            ObjectPropertyCode::ProtectionStatus => {
                entry.set_protection_status(info.protection_status)
            },
            ObjectPropertyCode::DateCreated => entry.set_date_created(info.date_created),
            ObjectPropertyCode::DateModified => entry.set_date_modified(info.date_modified),
            ObjectPropertyCode::ObjectSize => {
                match session.get_object_prop_value::<ObjectSize>(handle).await {
                    Ok(size) => entry.set_size(size.data.data),
                    Err(e) => {
                        tracing::warn!("Failed to get object size: {e}");
                        return;
                    },
                }
            },
            _ => {},
        }

        tracing::debug!("Object property changed: {handle:?}, prop_code: {prop:?}");
        let _ = tx.send(FsEvent::Refreshed(entry_clone));
    }

    async fn object_added(
        fs: Arc<Self>,
        session: &MtpSession<D>,
        handle: ObjectHandle,
    ) -> Result<Option<FolderEntry<D>>, MtpError<<D as PtpIo>::TransportError>> {
        let info = session.get_object_info(handle).await?.data.data;
        if info.storage_id != fs.storage_id {
            // Doesn't apply to us..
            return Ok(None);
        }

        let entry = FolderEntry::new(Arc::downgrade(&fs), handle, fs.storage_id, &info);

        let mut map = fs.inode_map.write().await;

        tracing::debug!(
            "Adding entry `{}` (handle: {:#X})",
            entry.path_inner(&map),
            Into::<u32>::into(entry.handle())
        );

        let parent_id = entry.parent();
        if let Some(FolderEntry::Folder(parent)) = map.get(&parent_id) {
            parent
                .children
                .write()
                .await
                .insert(entry.name().to_string(), entry.clone());
            map.insert(entry.handle(), entry.clone());
            return Ok(Some(entry));
        } else if parent_id == ObjectHandle::NONE
            && let Some(root) = fs.root.read().await.as_ref()
        {
            root.children
                .write()
                .await
                .insert(entry.name().to_string(), entry.clone());
            map.insert(entry.handle(), entry.clone());
            return Ok(Some(entry));
        }

        Ok(None)
    }

    async fn remove_entry(&self, handle: ObjectHandle) -> Option<FolderEntry<D>> {
        let mut map = self.inode_map.write().await;
        let entry = map.remove(&handle)?;

        tracing::debug!(
            "Removing entry `{}` (handle: {:#X})",
            entry.path_inner(&map),
            Into::<u32>::into(entry.handle())
        );

        let parent = entry.parent();
        if let Some(FolderEntry::Folder(parent)) = map.get(&parent) {
            parent.children.write().await.remove(entry.name());
        } else if parent == ObjectHandle::NONE
            && let Some(root) = self.root.read().await.as_ref()
        {
            root.children.write().await.remove(entry.name());
        }

        Some(entry)
    }

    async fn rename_entry(
        &self,
        map: &mut INodeMapGuard<'_, D>,
        handle: ObjectHandle,
        new_name: String,
    ) -> Option<()> {
        let entry = map.get(&handle)?;

        // Nothing actually changed?
        if entry.name() == new_name {
            return Some(());
        }

        let original_name = entry.name().to_string();
        let parent = entry.parent();

        if let Some(entry) = map.get_mut(&handle) {
            entry.set_name(new_name.clone());
        }

        let parent_opt = if parent == ObjectHandle::NONE {
            self.root.read().await.clone().map(FolderEntry::Folder)
        } else {
            map.get(&parent).cloned()
        };

        if let Some(FolderEntry::Folder(parent)) = parent_opt {
            let mut children = parent.children.write().await;
            if let Some(child_entry) = children.remove(&original_name) {
                children.insert(new_name, child_entry);
            }
        }

        Some(())
    }
}
