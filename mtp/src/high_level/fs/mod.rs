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

use mtp_spec::communication::operation::{BaseOperation, Operation};
use mtp_spec::device::session::MtpSession;
use mtp_spec::object::properties::ObjectPropertyCode;
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::str::FromStr;
use std::sync::{Arc, RwLock, Weak};
use tokio::sync::OnceCell;

/// Representation of a file on an MTP-compatible device
///
/// Note that it is **not** guaranteed that a device will support any or all of the operations available
/// on `File`.
#[derive(Debug)]
pub struct File {
    fs: Weak<FileSystem>,
    storage_id: StorageId,
    id: ObjectHandle,
    parent_id: ObjectHandle,
    name: String,
    size: u64,
    format: ObjectFormatCode,
    protection_status: ProtectionStatus,
    date_modified: Option<DateTime>,
    date_created: Option<DateTime>,

    spool: OnceCell<std::fs::File>,
}

// Getters
impl File {
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
    pub async fn open<D>(
        &self,
        session: &mut MtpSession<D>,
    ) -> Result<std::fs::File, crate::error::Error<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        let object = session.get_object(self.id).await?;
        let file_data = object.data.data;

        let mut tmp = tempfile::tempfile()?;
        tmp.write_all(&file_data)?;

        Ok(tmp)
    }

    /// Read a portion of the file
    ///
    /// # Errors
    ///
    /// * The device lies about supporting [`GetPartialObject`]
    /// * See also: [`File::open()`]
    ///
    /// [`GetPartialObject`]: crate::communication::operation::GetPartialObject
    pub async fn read_at<D>(
        &self,
        session: &mut MtpSession<D>,
        offset: u32,
        len: u32,
    ) -> Result<Vec<u8>, Error<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        let fs = self.fs.upgrade().expect("FS dropped");
        if fs.capabilities.supports_partial_read {
            let response = session.get_partial_object(self.id, offset, len).await?;
            return Ok(response.data.data);
        }

        let spool_file = self
            .spool
            .get_or_try_init(|| async {
                let object = session.get_object(self.id).await?;

                let mut tmp = tempfile::tempfile()?;

                tmp.write_all(&object.data.data)?;
                Ok::<std::fs::File, Error<<D as PtpIo>::TransportError>>(tmp)
            })
            .await?;

        let mut file_lock = spool_file.try_clone()?;
        file_lock.seek(SeekFrom::Start(offset as u64))?;

        let mut buffer = vec![0u8; len as usize];
        let bytes_read = file_lock.read(&mut buffer)?;
        buffer.truncate(bytes_read);

        Ok(buffer)
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
        session: &mut MtpSession<D>,
        name: N,
    ) -> Result<Arc<Self>, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
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
                date_modified: self.date_modified.clone(),
                date_created: self.date_created.clone(),
                spool: OnceCell::new(),
            }));
        }

        if !session
            .object_property_can_be_modified::<ObjectFileName>(self.format)
            .await?
        {
            return Err(MtpError::UnsupportedOperation.into());
        }

        let name_ptp = PtpString::from_str(name_str)
            .map_err(Into::<MtpError<<D as PtpIo>::TransportError>>::into)?;

        let _ = session
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
            date_modified: self.date_modified.clone(),
            date_created: self.date_created.clone(),
            spool: OnceCell::new(),
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
    pub async fn copy<D>(
        &self,
        session: &mut MtpSession<D>,
        parent: Option<ObjectHandle>,
    ) -> Result<ObjectHandle, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        let new_handle = session
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
    pub async fn move_<D>(
        &self,
        session: &mut MtpSession<D>,
        new_parent: &Folder,
    ) -> Result<Arc<Self>, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        session
            .move_object(self.id, self.storage_id, Some(new_parent.id))
            .await?;

        let fs = self.fs.upgrade().expect("FS dropped");

        let updated_file = Arc::new(Self {
            fs: self.fs.clone(),
            storage_id: self.storage_id,
            id: self.id,
            parent_id: new_parent.id,
            name: self.name.clone(),
            size: self.size,
            format: self.format,
            protection_status: self.protection_status,
            date_modified: self.date_modified.clone(),
            date_created: self.date_created.clone(),
            spool: OnceCell::new(),
        });

        // Sync FUSE maps
        let entry = FolderEntry::File(updated_file.clone());
        let inode_lock = fs.inode_map.read().unwrap();

        if let Some(FolderEntry::Folder(old_parent)) = inode_lock.get(&self.parent_id) {
            old_parent.children.write().unwrap().remove(&self.name);
        }
        if let Some(FolderEntry::Folder(new_parent_obj)) = inode_lock.get(&new_parent.id) {
            new_parent_obj
                .children
                .write()
                .unwrap()
                .insert(self.name.clone(), entry.clone());
        }

        drop(inode_lock);
        fs.inode_map.write().unwrap().insert(self.id, entry);

        Ok(updated_file)
    }

    pub fn object_info(&self) -> Result<ObjectInfo, <PtpString as FromStr>::Err> {
        let filename = PtpString::from_str(&self.name)?;

        Ok(ObjectInfo {
            storage_id: self.storage_id,
            object_format: self.format,
            protection_status: self.protection_status,
            compressed_size: self.size as u32,
            thumbnail: None,
            parent_object: Some(self.parent_id),
            association: None,
            sequence_number: 0,
            filename,
            date_created: self.date_modified.clone(), /* MTP handles often reuse modified for created */
            date_modified: self.date_modified.clone(),
            keywords: Default::default(),
        })
    }
}

/// Representation of a folder on an MTP-compatible device
///
/// Note that it is **not** guaranteed that a device will support any or all of the operations available
/// on `Folder`.
#[derive(Debug)]
pub struct Folder {
    fs: Weak<FileSystem>,
    /// The device-specific ID of the storage where this folder lives
    storage_id: StorageId,
    id: ObjectHandle,
    parent_id: ObjectHandle,
    name: String,
    format: ObjectFormatCode,
    protection_status: ProtectionStatus,
    date_created: Option<DateTime>,
    date_modified: Option<DateTime>,
    children: RwLock<HashMap<String, FolderEntry>>,
}

// Getters
impl Folder {
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
    pub fn with_children<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&HashMap<String, FolderEntry>) -> T,
    {
        let children = self.children.read().unwrap();
        f(&*children)
    }
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
        session: &mut MtpSession<D>,
        name: N,
    ) -> Result<Arc<Self>, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        N: AsRef<str>,
    {
        let name_str = name.as_ref();
        let name_ptp = PtpString::from_str(name_str)?;

        session
            .set_object_prop_value::<ObjectFileName>(self.id, name_ptp)
            .await?;

        let fs = self.fs.upgrade().expect("FS dropped");

        // When reconstructing the folder, we carry over its existing children map
        let updated_folder = Arc::new(Self {
            fs: self.fs.clone(),
            storage_id: self.storage_id,
            id: self.id,
            parent_id: self.parent_id,
            name: name_str.to_string(),
            format: self.format,
            protection_status: self.protection_status,
            date_created: self.date_created.clone(),
            date_modified: self.date_modified.clone(),

            // FUSE nodes require inner mutability to persist state across Arc clones
            children: RwLock::new(self.children.read().unwrap().clone()),
        });

        let entry = FolderEntry::Folder(updated_folder.clone());
        fs.inode_map.write().unwrap().insert(self.id, entry.clone());

        if let Some(FolderEntry::Folder(parent)) = fs.inode_map.read().unwrap().get(&self.parent_id)
        {
            let mut children = parent.children.write().unwrap();
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
    pub async fn copy<D>(
        &self,
        session: &mut MtpSession<D>,
        parent: Option<ObjectHandle>,
    ) -> Result<ObjectHandle, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        let new_handle = session
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
    pub async fn move_<D>(
        &self,
        session: &mut MtpSession<D>,
        new_parent: &Folder,
    ) -> Result<Arc<Self>, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        session
            .move_object(self.id, self.storage_id, Some(new_parent.id))
            .await?;

        let fs = self.fs.upgrade().expect("FS dropped");

        let updated_folder = Arc::new(Self {
            fs: self.fs.clone(),
            storage_id: self.storage_id,
            id: self.id,
            parent_id: new_parent.id,
            name: self.name.clone(),
            format: self.format,
            protection_status: self.protection_status,
            date_created: self.date_created.clone(),
            date_modified: self.date_modified.clone(),
            children: RwLock::new(self.children.read().unwrap().clone()),
        });

        let entry = FolderEntry::Folder(updated_folder.clone());
        let inode_lock = fs.inode_map.read().unwrap();

        if let Some(FolderEntry::Folder(old_parent)) = inode_lock.get(&self.parent_id) {
            old_parent.children.write().unwrap().remove(&self.name);
        }

        if let Some(FolderEntry::Folder(new_parent)) = inode_lock.get(&new_parent.id) {
            new_parent
                .children
                .write()
                .unwrap()
                .insert(self.name.clone(), entry.clone());
        }

        drop(inode_lock);
        fs.inode_map.write().unwrap().insert(self.id, entry);

        Ok(updated_folder)
    }

    /// Create a new file within this `Folder`
    pub async fn create_file<D, N>(
        &self,
        session: &mut MtpSession<D>,
        name: N,
        format: ObjectFormatCode,
        data: Vec<u8>,
    ) -> Result<Arc<File>, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        N: AsRef<str> + Send,
    {
        let file = session
            .create(Some(self), name.as_ref(), format, data)
            .await?;
        let file = Arc::new(file);
        self.children
            .write()
            .unwrap()
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
    /// [`DeleteObject`]: crate::operations::object::DeleteObject
    pub async fn remove_child<D>(
        &self,
        session: &mut MtpSession<D>,
        name: &str,
    ) -> Result<(), FileSystemError<D>>
    where
        D: Device,
    {
        let mut children = self.children.write().unwrap();
        let Some(entry) = children.remove(name) else {
            return Err(FileSystemError::Io(std::io::Error::from(
                ErrorKind::NotFound,
            )));
        };

        match session.delete_object(entry.handle(), None).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // Assume the entry is still on the device?
                children.insert(name.to_string(), entry);
                Err(FileSystemError::Mtp(e))
            },
        }
    }

    /// Find an entry within this folder by name
    pub fn find(&self, name: &str) -> Option<FolderEntry> {
        self.children.read().unwrap().get(name).cloned()
    }
}

/// An entry in a [`Folder`]
#[derive(Clone, Debug)]
pub enum FolderEntry {
    /// A [`File`] entry
    File(Arc<File>),
    /// A [`Folder`] entry
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
        session: &mut MtpSession<D>,
        name: N,
    ) -> Result<Self, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        N: AsRef<str>,
    {
        match self {
            FolderEntry::File(f) => Ok(FolderEntry::File(f.rename(session, name).await?)),
            FolderEntry::Folder(f) => Ok(FolderEntry::Folder(f.rename(session, name).await?)),
        }
    }

    /// Attempt to copy this entry to another directory
    ///
    /// # Errors
    ///
    /// See [`File::copy()`] and [`Folder::copy()`]
    pub async fn copy<D>(
        &self,
        session: &mut MtpSession<D>,
        parent: Option<ObjectHandle>,
    ) -> Result<ObjectHandle, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        match self {
            FolderEntry::File(f) => f.copy(session, parent).await,
            FolderEntry::Folder(f) => f.copy(session, parent).await,
        }
    }

    /// Attempt to copy this entry to another directory
    ///
    /// # Errors
    ///
    /// See [`File::move_()`] and [`Folder::move_()`]
    pub async fn move_<D>(
        &self,
        session: &mut MtpSession<D>,
        new_parent: &Folder,
    ) -> Result<FolderEntry, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        match self {
            FolderEntry::File(f) => Ok(FolderEntry::File(f.move_(session, new_parent).await?)),
            FolderEntry::Folder(f) => Ok(FolderEntry::Folder(f.move_(session, new_parent).await?)),
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
    /// Whether the device supports the [`GetObjectPropList`] operation
    ///
    /// [`GetObjectPropList`]: crate::communication::operation::GetObjectPropList
    pub supports_bulk_props: bool,
}

impl DeviceCapabilities {
    async fn query<D: Device>(
        session: &mut MtpSession<D>,
    ) -> Result<Self, MtpError<<D as PtpIo>::TransportError>> {
        let info = session.get_device_info().await?;
        let ops = info.data.data.operations_supported.as_slice();

        Ok(Self {
            supports_partial_read: ops.contains(&Operation::Base(BaseOperation::GetPartialObject)),
            supports_bulk_props: ops.contains(&Operation::Base(BaseOperation::GetObjectPropList)),
        })
    }
}

/// A filesystem abstraction for MTP devices
pub struct FileSystem {
    capabilities: DeviceCapabilities,
    storage_id: StorageId,
    root: RwLock<Option<Arc<Folder>>>,
    inode_map: RwLock<HashMap<ObjectHandle, FolderEntry>>,
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

impl FileSystem {
    /// Create a new [`File`] at the specified path
    ///
    /// # Errors
    ///
    /// * Not all parent folders exist
    /// * Not all parent path segments are [`Folder`]s
    /// * See [`Folder::create_file()`]
    pub async fn create<D>(
        &self,
        session: &mut MtpSession<D>,
        path: impl AsRef<str>,
        format: ObjectFormatCode,
        data: Vec<u8>,
    ) -> Result<Arc<File>, FileSystemError<D>>
    where
        D: Device,
    {
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
            .ok_or(std::io::Error::from(ErrorKind::NotFound))?;

        let FolderEntry::Folder(parent) = parent_entry else {
            return Err(std::io::Error::from(ErrorKind::NotADirectory).into());
        };

        parent
            .create_file(session, file_name, format, data)
            .await
            .map_err(FileSystemError::Mtp)
    }

    /// Find an entry by its path
    pub fn find(&self, path: impl AsRef<str>) -> Option<FolderEntry> {
        let path = path.as_ref();

        // Nothing to do with non-absolute paths
        if !path.starts_with('/') {
            return None;
        }

        let root = self.root.read().unwrap().clone()?;
        let mut current = FolderEntry::Folder(root);

        for component in path.split('/').filter(|c| !c.is_empty()) {
            if let FolderEntry::Folder(folder) = current {
                current = folder.find(component)?;
            } else {
                return None;
            }
        }

        Some(current)
    }

    /// Call the function `f` with immutable access to the [`FileSystem`] root
    pub fn with_root<F, T>(&self, f: F) -> T
    where
        F: FnOnce(Arc<Folder>) -> T,
    {
        let guard = self.root.read().unwrap();
        f((&*guard).as_ref().cloned().expect("root should exist"))
    }

    /// Load a `FileSystem` from the given `storage_id`
    ///
    /// Note that the speed of this depends entirely on the device and the numbers of files on the storage.
    /// See [Performance Considerations].
    ///
    /// [Performance Considerations]: https://docs.rs/mtp/latest/mtp/#performance-considerations
    pub async fn load<D>(
        session: &mut MtpSession<D>,
        storage_id: StorageId,
    ) -> Result<Arc<Self>, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
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
    pub async fn load_with_callback<D, F>(
        session: &mut MtpSession<D>,
        storage_id: StorageId,
        mut callback: F,
    ) -> Result<Arc<Self>, MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
        F: FnMut(ObjectHandle),
    {
        let capabilities = DeviceCapabilities::query(session).await?;

        let fs = Arc::new(Self {
            capabilities,
            storage_id,
            root: RwLock::new(None),
            inode_map: RwLock::new(HashMap::new()),
        });

        let fs_weak = Arc::downgrade(&fs);

        let handles_res = session.get_object_handles(storage_id, None, None).await?;
        let handles = handles_res.data.data.as_slice();

        let mut flat_map: HashMap<ObjectHandle, FolderEntry> =
            HashMap::with_capacity(handles.len());

        if fs.capabilities.supports_bulk_props {
            Self::load_bulk_fast_path(session, fs_weak.clone(), handles, storage_id, &mut flat_map)
                .await?;
        } else {
            Self::load_iterative_slow_path(
                session,
                fs_weak.clone(),
                handles,
                storage_id,
                &mut flat_map,
            )
            .await?;
        }

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

        let entries: Vec<FolderEntry> = flat_map.values().cloned().collect();
        for entry in entries {
            let parent_id = match &entry {
                FolderEntry::File(f) => f.parent_id,
                FolderEntry::Folder(f) => f.parent_id,
            };

            if entry.handle() == ObjectHandle::NONE {
                continue;
            }

            if let Some(FolderEntry::Folder(parent)) = flat_map.get(&parent_id) {
                parent
                    .children
                    .write()
                    .unwrap()
                    .insert(entry.name().to_string(), entry);
            } else {
                root.children
                    .write()
                    .unwrap()
                    .insert(entry.name().to_string(), entry);
            }
        }

        *fs.inode_map.write().unwrap() = flat_map;
        *fs.root.write().unwrap() = Some(root);

        Ok(fs)
    }

    async fn load_bulk_fast_path<D>(
        session: &mut MtpSession<D>,
        fs: Weak<FileSystem>,
        handles: &[ObjectHandle],
        storage_id: StorageId,
        flat_map: &mut HashMap<ObjectHandle, FolderEntry>,
    ) -> Result<(), MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        let prop_list = session
            .get_object_prop_list(
                ObjectHandle::ALL,
                None,
                Some(ObjectPropertyCode::All),
                None,
                Some(u32::MAX),
            )
            .await?;

        // Since `ObjectHandle::ALL` returns data for EVERY storage volume on the device,
        // we must intersect the results with the handles we know belong to this `storage_id`.
        let storage_handles: std::collections::HashSet<ObjectHandle> =
            handles.iter().copied().collect();

        #[derive(Default)]
        struct PartialInfo {
            parent_id: Option<ObjectHandle>,
            name: Option<String>,
            size: Option<u64>,
            format: Option<ObjectFormatCode>,
            protection: Option<ProtectionStatus>,
            modified: Option<DateTime>,
            created: Option<DateTime>,
        }

        let mut parsed_data: HashMap<ObjectHandle, PartialInfo> =
            HashMap::with_capacity(handles.len());

        for prop in prop_list.data.data {
            if !storage_handles.contains(&prop.object()) {
                continue;
            }

            let entry = parsed_data.entry(prop.object()).or_default();
            let value = prop.value();
            match ObjectPropertyCode::from(prop.code()) {
                ObjectPropertyCode::ParentObject => {
                    entry.parent_id = value.as_u32().map(ObjectHandle::from)
                },
                ObjectPropertyCode::ObjectFormat => {
                    entry.format = value.as_u16().map(ObjectFormatCode::from)
                },
                ObjectPropertyCode::ObjectFileName => {
                    entry.name = value.as_string().map(|s| s.to_string())
                },
                ObjectPropertyCode::ObjectSize => entry.size = value.as_u64(),
                ObjectPropertyCode::ProtectionStatus => {
                    entry.protection = value.as_u16().map(ProtectionStatus::from)
                },
                ObjectPropertyCode::DateModified => {
                    entry.modified = value.as_string().and_then(|s| DateTime::try_from(s).ok())
                },
                ObjectPropertyCode::DateCreated => {
                    entry.created = value.as_string().and_then(|s| DateTime::try_from(s).ok())
                },
                _ => {},
            }
        }

        for (handle, info) in parsed_data {
            let name = info
                .name
                .unwrap_or_else(|| format!("UNKNOWN_{}", Into::<u32>::into(handle)));
            let format = info.format.unwrap_or(ObjectFormatCode::Undefined);
            let parent_id = info.parent_id.unwrap_or(ObjectHandle::NONE);
            let protection_status = info.protection.unwrap_or(ProtectionStatus::ReadOnly);

            if format == ObjectFormatCode::Association {
                flat_map.insert(
                    handle,
                    FolderEntry::Folder(Arc::new(Folder {
                        fs: fs.clone(),
                        id: handle,
                        storage_id,
                        parent_id,
                        name,
                        format,
                        protection_status,
                        date_created: None,
                        date_modified: info.modified,
                        children: RwLock::new(HashMap::new()),
                    })),
                );
            } else {
                flat_map.insert(
                    handle,
                    FolderEntry::File(Arc::new(File {
                        fs: fs.clone(),
                        id: handle,
                        storage_id,
                        parent_id,
                        name,
                        size: info.size.unwrap_or(0),
                        format,
                        protection_status,
                        date_modified: info.modified,
                        date_created: info.created,
                        spool: OnceCell::new(),
                    })),
                );
            }
        }

        Ok(())
    }

    async fn load_iterative_slow_path<D>(
        session: &mut MtpSession<D>,
        fs: Weak<FileSystem>,
        handles: &[ObjectHandle],
        storage_id: StorageId,
        flat_map: &mut HashMap<ObjectHandle, FolderEntry>,
    ) -> Result<(), MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        for &handle in handles {
            let info = session.get_object_info(handle).await?.data.data;
            let name = info.filename.to_string();
            let parent_id = info.parent_object.unwrap_or(ObjectHandle::NONE);

            if info.object_format == ObjectFormatCode::Association {
                flat_map.insert(
                    handle,
                    FolderEntry::Folder(Arc::new(Folder {
                        fs: fs.clone(),
                        id: handle,
                        storage_id,
                        parent_id,
                        name,
                        format: info.object_format,
                        protection_status: info.protection_status,
                        date_created: None,
                        date_modified: info.date_modified,
                        children: RwLock::new(HashMap::new()),
                    })),
                );
            } else {
                flat_map.insert(
                    handle,
                    FolderEntry::File(Arc::new(File {
                        fs: fs.clone(),
                        id: handle,
                        storage_id,
                        parent_id,
                        name,
                        size: info.compressed_size as u64,
                        format: info.object_format,
                        protection_status: info.protection_status,
                        date_modified: info.date_modified,
                        date_created: info.date_created,
                        spool: OnceCell::new(),
                    })),
                );
            }
        }
        Ok(())
    }

    /// Refresh the `FileSystem` to match the new state of the device
    ///
    /// Note that the speed of this depends entirely on the device and the numbers of files on the storage.
    /// See [Performance Considerations].
    ///
    /// [Performance Considerations]: https://docs.rs/mtp/latest/mtp/#performance-considerations
    pub async fn refresh<D>(
        &mut self,
        session: &mut MtpSession<D>,
    ) -> Result<(), MtpError<<D as PtpIo>::TransportError>>
    where
        D: Device,
    {
        self.refresh_with_callback(session, |_| {}).await
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
        session: &mut MtpSession<D>,
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
