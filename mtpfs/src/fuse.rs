use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{Read, Seek, SeekFrom, Write};
use std::iter;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use fuser::{
    Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation, INodeNo,
    KernelConfig, LockOwner, OpenFlags, RenameFlags, ReplyAttr, ReplyData, ReplyDirectory,
    ReplyEmpty, ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, ReplyXattr, Request, WriteFlags,
};
use indicatif::{ProgressBar, ProgressStyle};
use mtp::communication::response::SendObjectInfo;
use mtp::device::Device;
use mtp::device::extensions::android::AndroidDevice;
use mtp::device::session::MtpSession;
use mtp::error::MtpError;
use mtp::high_level::fs::{
    File, FileSystem, FileSystemError, Folder, FolderEntry, FsEvent, SessionFsExt,
};
use mtp::high_level::storages::Storage;
use mtp::object::{DateTime, ObjectHandle};
use mtp::usb::DeviceHandle;
use mtp::usb::error::{Error, UsbError};
use tokio::runtime::Handle;
use tokio::sync::RwLock;
use tracing::info;

const TTL: Duration = Duration::from_secs(u64::MAX);
const BLOCK_SIZE: u32 = 1024;

const ROOT_ATTR: FileAttr = FileAttr {
    ino: INodeNo::ROOT,
    size: 0,
    blocks: 0,
    atime: SystemTime::UNIX_EPOCH,
    mtime: SystemTime::UNIX_EPOCH,
    ctime: SystemTime::UNIX_EPOCH,
    crtime: SystemTime::UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o777,
    nlink: 2,
    uid: 0,
    gid: 0,
    rdev: 0,
    blksize: 0,
    flags: 0,
};

const PLAYLISTS_NODE_ID: INodeNo = INodeNo(2);
const PLAYLISTS_ATTR: FileAttr = FileAttr {
    ino: PLAYLISTS_NODE_ID,
    size: 0,
    blocks: 0,
    atime: SystemTime::UNIX_EPOCH,
    mtime: SystemTime::UNIX_EPOCH,
    ctime: SystemTime::UNIX_EPOCH,
    crtime: SystemTime::UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o777,
    nlink: 1,
    uid: 0,
    gid: 0,
    rdev: 0,
    blksize: BLOCK_SIZE,
    flags: 0,
};

const LOST_AND_FOUND_NODE_ID: INodeNo = INodeNo(3);
const LOST_AND_FOUND_ATTR: FileAttr = FileAttr {
    ino: LOST_AND_FOUND_NODE_ID,
    size: 0,
    blocks: 0,
    atime: SystemTime::UNIX_EPOCH,
    mtime: SystemTime::UNIX_EPOCH,
    ctime: SystemTime::UNIX_EPOCH,
    crtime: SystemTime::UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o777,
    nlink: 1,
    uid: 0,
    gid: 0,
    rdev: 0,
    blksize: BLOCK_SIZE,
    flags: 0,
};

#[derive(Clone)]
enum Entry {
    /// An entry on the device
    Real {
        /// A virtual inode number derived from the host [`ObjectHandle`].
        ///
        /// See [`FsState::handle_to_ino()`]
        ino: INodeNo,
        entry: FolderEntry<DeviceHandle>,
    },
    /// A virtual entry that doesn't actually exist on the device
    Virtual { ino: INodeNo, name: Arc<str> },
}

impl Entry {
    /// Get the file name for this entry
    fn name(&self) -> &str {
        match self {
            Self::Real { entry, .. } => entry.name(),
            Self::Virtual { name, .. } => &name,
        }
    }

    /// Get the virtual inode number for this entry
    fn ino(&self) -> INodeNo {
        match self {
            Self::Real { ino, .. } => *ino,
            Self::Virtual { ino, .. } => *ino,
        }
    }

    /// Create the [`FileAttr`] for this entry
    fn attr(&self) -> FileAttr {
        match self {
            Entry::Real { entry, ino } => match entry {
                FolderEntry::File(file) => FileAttr {
                    ino: *ino,
                    size: file.size(),
                    blocks: (file.size() + u64::from(BLOCK_SIZE) - 1) / u64::from(BLOCK_SIZE),
                    atime: SystemTime::UNIX_EPOCH,
                    mtime: file
                        .date_modified()
                        .and_then(DateTime::as_systemtime)
                        .unwrap_or(SystemTime::UNIX_EPOCH),
                    ctime: SystemTime::UNIX_EPOCH,
                    crtime: file
                        .date_created()
                        .and_then(DateTime::as_systemtime)
                        .unwrap_or(SystemTime::UNIX_EPOCH),
                    kind: FileType::RegularFile,
                    perm: 0o777,
                    nlink: 1,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    blksize: BLOCK_SIZE,
                    flags: 0,
                },
                FolderEntry::Folder(folder) => FileAttr {
                    ino: *ino,
                    size: 0,
                    blocks: 0,
                    atime: SystemTime::UNIX_EPOCH,
                    mtime: folder
                        .date_modified()
                        .and_then(DateTime::as_systemtime)
                        .unwrap_or(SystemTime::UNIX_EPOCH),
                    ctime: SystemTime::UNIX_EPOCH,
                    crtime: folder
                        .date_created()
                        .and_then(DateTime::as_systemtime)
                        .unwrap_or(SystemTime::UNIX_EPOCH),
                    kind: FileType::Directory,
                    perm: 0o777,
                    nlink: 2,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    blksize: BLOCK_SIZE,
                    flags: 0,
                },
            },
            Entry::Virtual { ino, .. } => match *ino {
                PLAYLISTS_NODE_ID => PLAYLISTS_ATTR,
                LOST_AND_FOUND_NODE_ID => LOST_AND_FOUND_ATTR,
                _ => ROOT_ATTR,
            },
        }
    }
}

#[derive(Default)]
struct FsState {
    nodes: RwLock<HashMap<INodeNo, Entry>>,
}

/// The offset at which non-virtual inodes start
const MTP_INODE_OFFSET: u64 = 10;

impl FsState {
    /// Get an [`Entry`] by its inode number
    async fn get(&self, inode: INodeNo) -> Option<Entry> {
        self.nodes.read().await.get(&inode).cloned()
    }

    async fn file(&self, ino: INodeNo) -> Result<Arc<File<DeviceHandle>>, Errno> {
        match self.get(ino).await {
            Some(Entry::Real {
                entry: FolderEntry::File(file),
                ..
            }) => Ok(file),
            Some(Entry::Real {
                entry: FolderEntry::Folder(_),
                ..
            }) => Err(Errno::EISDIR),
            Some(_) => Err(Errno::EINVAL),
            None => Err(Errno::ENOENT),
        }
    }

    async fn dir(&self, ino: INodeNo) -> Result<Arc<Folder<DeviceHandle>>, Errno> {
        match self.get(ino).await {
            Some(Entry::Real {
                entry: FolderEntry::Folder(folder),
                ..
            }) => Ok(folder),
            Some(_) => Err(Errno::ENOTDIR),
            None => Err(Errno::ENOENT),
        }
    }

    /// Insert a new [`FolderEntry`]
    async fn insert(&self, entry: FolderEntry<DeviceHandle>) -> INodeNo {
        let handle = entry.handle();
        let ino = self.handle_to_ino(handle);
        self.nodes
            .write()
            .await
            .insert(ino, Entry::Real { entry, ino });
        ino
    }

    async fn update(&self, entry: FolderEntry<DeviceHandle>) {
        let handle = entry.handle();
        let ino = self.handle_to_ino(handle);
        self.nodes
            .write()
            .await
            .entry(ino)
            .insert_entry(Entry::Real { entry, ino });
    }

    /// Remove a node from the map
    async fn remove(&self, inode: INodeNo) {
        self.nodes.write().await.remove(&inode);
    }

    /// Refresh from the current state of the [`FileSystem`]
    async fn refresh(state: Arc<Self>, fs: &FileSystem<DeviceHandle>) {
        async fn walk_and_insert(fs: Arc<FsState>, root: FolderEntry<DeviceHandle>) {
            let mut stack = vec![root];

            while let Some(entry) = stack.pop() {
                fs.insert(entry.clone()).await;

                if let FolderEntry::Folder(folder) = entry {
                    let children = folder
                        .with_children(|children| {
                            let children = children.values().cloned().collect::<Vec<_>>();
                            async move { children }
                        })
                        .await;

                    stack.extend(children);
                }
            }
        }

        state
            .nodes
            .write()
            .await
            .retain(|&ino, _| ino.0 < MTP_INODE_OFFSET); // Keep virtual nodes
        fs.with_root(|root| walk_and_insert(state, FolderEntry::Folder(root.clone())))
            .await;
    }

    /// Convert an MTP [`ObjectHandle`] to a FUSE inode
    fn handle_to_ino(&self, handle: ObjectHandle) -> INodeNo {
        if handle == ObjectHandle::NONE {
            ROOT_ATTR.ino
        } else {
            INodeNo(u64::from(Into::<u32>::into(handle)) + MTP_INODE_OFFSET)
        }
    }
}

pub struct MtpFuse {
    session: MtpSession<DeviceHandle>,
    storage: Storage,
    is_android: bool,

    fs: Option<Arc<FileSystem<DeviceHandle>>>,
    state: Arc<FsState>,
    runtime: Handle,
}

impl MtpFuse {
    pub async fn new(
        session: MtpSession<DeviceHandle>,
        storage: Storage,
    ) -> mtp::usb::error::Result<Self> {
        let is_android = session.is_android().await?;
        if !is_android {
            tracing::warn!(
                "The selected device doesn't support the Android MTP extensions. Writes will \
                 likely have bad performance."
            )
        }

        let mut nodes = HashMap::new();
        for virtual_dir in [
            Entry::Virtual {
                ino: ROOT_ATTR.ino,
                name: "/".into(),
            },
            Entry::Virtual {
                ino: PLAYLISTS_NODE_ID,
                name: "Playlists".into(),
            },
            Entry::Virtual {
                ino: LOST_AND_FOUND_NODE_ID,
                name: "lost+found".into(),
            },
        ] {
            nodes.insert(virtual_dir.ino(), virtual_dir);
        }

        let state = Arc::new(FsState {
            nodes: RwLock::new(nodes),
        });

        Ok(MtpFuse {
            session,
            storage,
            is_android,
            fs: None,
            state,
            runtime: Handle::current(),
        })
    }

    // TODO
    fn playlists(&self) -> impl Iterator<Item = (String, FileAttr)> {
        iter::empty()
    }

    // TODO
    fn lost_and_found(&self) -> impl Iterator<Item = (String, FileAttr)> {
        iter::empty()
    }
}

impl Filesystem for MtpFuse {
    fn init(&mut self, _req: &Request, _config: &mut KernelConfig) -> std::io::Result<()> {
        let start = Instant::now();

        let session = self.session.clone();
        let state = self.state.clone();
        let storage_id = self.storage.id;
        let runtime = self.runtime.clone();
        tokio::task::block_in_place(move || {
            runtime.block_on(async {
                let spinner = ProgressBar::new_spinner()
                    .with_message("Loading all MTP directories into memory")
                    .with_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());
                let fs = FileSystem::load_with_callback(session, storage_id, |_| {
                    spinner.tick();
                })
                .await;
                spinner.finish_and_clear();

                match fs {
                    Ok((fs, mut events)) => {
                        FsState::refresh(state.clone(), &fs).await;

                        tokio::task::spawn(async move {
                            while let Some(event) = events.recv().await {
                                match event {
                                    FsEvent::Added(entry) => {
                                        state.insert(entry).await;
                                    },
                                    FsEvent::Removed(entry) => {
                                        state.remove(state.handle_to_ino(entry.handle())).await;
                                    },
                                    FsEvent::Refreshed(entry) => {
                                        state.update(entry).await;
                                    },
                                }
                            }
                        });

                        let storage_name = self
                            .storage
                            .description
                            .as_deref()
                            .unwrap_or(self.storage.volume_identifier.as_str());

                        let load_time = start.elapsed();
                        let load_time_seconds = load_time.as_secs() % 60;

                        info!(
                            "Finished loading filesystem for storage `{storage_name}` in \
                             {:02}:{:02}",
                            (load_time.as_secs() - load_time_seconds) / 60,
                            load_time_seconds
                        );

                        self.fs = Some(fs);

                        Ok(())
                    },
                    Err(e) => {
                        tracing::error!("Failed to initialize MTP FUSE mount: {e}");
                        Err(std::io::Error::from_raw_os_error(e.to_errno().code()))
                    },
                }
            })
        })
    }

    fn destroy(&mut self) {
        tracing::info!("Closing filesystem");
        self.state = Arc::default();
        let _ = self.fs.take();
    }

    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let Some(name_str) = name.to_str().map(String::from) else {
            return reply.error(Errno::EINVAL);
        };

        let state = self.state.clone();
        self.runtime.spawn(async move {
            let parent = match state.dir(parent).await {
                Ok(parent_entry) => parent_entry,
                Err(e) => {
                    return reply.error(e);
                },
            };

            let Some(child) = parent.find(&name_str).await else {
                return reply.error(Errno::ENOENT);
            };

            let ino = state.insert(child.clone()).await;
            reply.entry(
                &TTL,
                &Entry::Real { entry: child, ino }.attr(),
                Generation(0),
            );
        });
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        let state = self.state.clone();
        self.runtime.spawn(async move {
            let Some(entry) = state.get(ino).await else {
                return reply.error(Errno::ENOENT);
            };

            reply.attr(&TTL, &entry.attr());
        });
    }

    fn mkdir(
        &self,
        _req: &Request,
        parent: INodeNo,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        let Some(name_str) = name.to_str().map(String::from) else {
            return reply.error(Errno::EINVAL);
        };

        let session = self.session.clone();
        let state = self.state.clone();
        self.runtime.spawn(async move {
            let Some(Entry::Real {
                entry: FolderEntry::Folder(parent_dir),
                ..
            }) = state.get(parent).await
            else {
                return reply.error(Errno::ENOTDIR);
            };

            let result = session.mkdir(Some(parent_dir.as_ref()), name_str).await;
            match result {
                Ok(folder) => {
                    let entry = FolderEntry::Folder(Arc::new(folder));
                    let ino = state.insert(entry.clone()).await;
                    reply.entry(&TTL, &Entry::Real { entry, ino }.attr(), Generation(0));
                },
                Err(e) => reply.error(e.to_errno()),
            }
        });
    }

    // TODO: Figure out why the lookups still resolve in rmdir and unlink even after destroy() is called
    fn unlink(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let Some(name_str) = name.to_str().map(String::from) else {
            return reply.error(Errno::EINVAL);
        };

        let state = self.state.clone();
        self.runtime.spawn(async move {
            let parent = match state.dir(parent).await {
                Ok(parent) => parent,
                Err(e) => return reply.error(e),
            };

            let Some(child_entry) = parent.find(&name_str).await else {
                return reply.error(Errno::ENOENT);
            };

            let FolderEntry::File(child) = child_entry else {
                return reply.error(Errno::EISDIR);
            };

            let result = parent.remove_child(&name_str).await;

            match result {
                Ok(_) => {
                    state.remove(state.handle_to_ino(child.id())).await;
                    reply.ok();
                },
                Err(e) => match e {
                    FileSystemError::Mtp(e) => reply.error(e.to_errno()),
                    FileSystemError::Io(e) => reply.error(e.to_errno()),
                },
            }
        });
    }

    fn rmdir(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let Some(name_str) = name.to_str().map(String::from) else {
            return reply.error(Errno::EINVAL);
        };

        let state = self.state.clone();
        self.runtime.spawn(async move {
            let parent_dir = match state.dir(parent).await {
                Ok(parent) => parent,
                Err(e) => return reply.error(e),
            };

            let Some(FolderEntry::Folder(child)) = parent_dir.find(&name_str).await else {
                return reply.error(Errno::ENOENT);
            };

            match parent_dir.remove_child(&name_str).await {
                Ok(_) => {
                    state.remove(state.handle_to_ino(child.id())).await;
                    reply.ok();
                },
                Err(e) => match e {
                    FileSystemError::Mtp(e) => reply.error(e.to_errno()),
                    FileSystemError::Io(e) => reply.error(e.to_errno()),
                },
            }
        });
    }

    fn symlink(
        &self,
        _req: &Request,
        _parent: INodeNo,
        _link_name: &OsStr,
        _target: &Path,
        reply: ReplyEntry,
    ) {
        reply.error(Errno::ENOTSUP);
    }

    fn rename(
        &self,
        _req: &Request,
        parent: INodeNo,
        name: &OsStr,
        newparent: INodeNo,
        newname: &OsStr,
        _flags: RenameFlags,
        reply: ReplyEmpty,
    ) {
        let Some(name_str) = name.to_str().map(String::from) else {
            return reply.error(Errno::EINVAL);
        };
        let Some(new_name_str) = newname.to_str().map(String::from) else {
            return reply.error(Errno::EINVAL);
        };

        let state = self.state.clone();
        self.runtime.spawn(async move {
            let original_parent = match state.dir(parent).await {
                Ok(folder) => folder,
                Err(e) => return reply.error(e),
            };

            let new_parent = match state.dir(newparent).await {
                Ok(folder) => folder,
                Err(e) => return reply.error(e),
            };

            let Some(child) = original_parent.find(&name_str).await else {
                return reply.error(Errno::ENOENT);
            };

            let mut current_child = child.clone();
            if parent != newparent {
                current_child = match current_child.move_(new_parent.as_ref()).await {
                    Ok(c) => c,
                    Err(e) => {
                        reply.error(e.to_errno());
                        return;
                    },
                };
            }

            if name_str != new_name_str {
                current_child = match current_child.rename(new_name_str).await {
                    Ok(c) => c,
                    Err(e) => {
                        reply.error(e.to_errno());
                        return;
                    },
                };
            }

            state.remove(state.handle_to_ino(child.handle())).await;
            state.insert(current_child).await;
            reply.ok();
        });
    }

    fn link(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _newparent: INodeNo,
        _newname: &OsStr,
        reply: ReplyEntry,
    ) {
        reply.error(Errno::ENOTSUP);
    }

    fn open(&self, _req: &Request, ino: INodeNo, flags: OpenFlags, reply: ReplyOpen) {
        let session = self.session.clone();
        let state = self.state.clone();
        self.runtime.spawn(async move {
            let file = match state.file(ino).await {
                Ok(file) => file,
                Err(e) => {
                    reply.error(e);
                    return;
                },
            };

            match session.get_object_info(file.id()).await {
                Ok(_fd) => reply.opened(
                    FileHandle(0),
                    FopenFlags::from_bits_truncate(flags.0 as u32),
                ),
                Err(e) => {
                    reply.error(e.to_errno());
                },
            }
        });
    }

    fn read(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        let state = self.state.clone();
        self.runtime.spawn(async move {
            let file = match state.file(ino).await {
                Ok(file) => file,
                Err(e) => {
                    reply.error(e);
                    return;
                },
            };

            match file.read_at(offset as u32, size).await {
                Ok(data) => reply.data(&data),
                Err(e) => reply.error(e.to_errno()),
            }
        });
    }

    fn write(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        data: &[u8],
        _write_flags: WriteFlags,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyWrite,
    ) {
        let session = self.session.clone();
        let fs = self.fs.clone().expect("fs should be set");
        let target_storage_id = self.storage.id;
        let state = self.state.clone();
        let is_android = self.is_android;
        let data = data.to_vec();
        self.runtime.spawn(async move {
            let file = match state.file(ino).await {
                Ok(file) => file,
                Err(e) => {
                    reply.error(e);
                    return;
                },
            };

            // Android has a fast path, where we can actually edit files in-place
            if is_android {
                match session
                    .send_partial_object(file.id(), offset, data.len() as u32, data)
                    .await
                {
                    Ok(response) => {
                        reply.written(response.data.length);
                        return;
                    },
                    Err(e) => {
                        reply.error(e.to_errno());
                        return;
                    },
                }
            }

            // Otherwise, we need to:
            // - Create a temp file and perform the write
            // - Delete the old object on the device
            // - Send a new copy of the object to the device

            let mut temp = match file.open().await {
                Ok(file) => file,
                Err(e) => {
                    reply.error(e.to_errno());
                    return;
                },
            };

            if let Err(e) = temp.seek(SeekFrom::Start(offset)) {
                reply.error(e.to_errno());
                return;
            }

            if let Err(e) = temp.write_all(&data) {
                reply.error(e.to_errno());
                return;
            }

            let mut object_data = Vec::new();
            if let Err(e) = temp.read_to_end(&mut object_data) {
                reply.error(e.to_errno());
                return;
            }

            let Ok(object_info) = file.object_info() else {
                reply.error(Errno::EINVAL);
                return;
            };

            if let Err(e) = session.delete_object(file.id(), None).await {
                reply.error(e.to_errno());
                return;
            }

            let id;
            match session.send_object_info(object_info).await {
                Ok(response) => {
                    let SendObjectInfo {
                        storage_id,
                        parent: _,
                        reserved_handle,
                    } = response.data;

                    if storage_id != target_storage_id {
                        // Well... the device decided not to honor the request
                        reply.error(Errno::ENOTRECOVERABLE);
                        return;
                    }

                    id = reserved_handle;
                },
                Err(e) => {
                    reply.error(e.to_errno());
                    return;
                },
            }

            if let Err(e) = session.send_object(object_data).await {
                reply.error(e.to_errno());
                return;
            }

            match fs.add(id).await {
                Ok(Some(entry)) => {
                    state.insert(entry).await;
                    reply.written(data.len() as u32);
                },
                Ok(None) => {
                    // Maybe the device put it on a different storage?
                    // Nothing we can do
                    return reply.error(Errno::ENOTRECOVERABLE);
                },
                Err(e) => {
                    return reply.error(e.to_errno());
                },
            }
        });
    }

    fn flush(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _fh: FileHandle,
        _lock_owner: LockOwner,
        reply: ReplyEmpty,
    ) {
        reply.ok()
    }

    fn opendir(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        let state = self.state.clone();
        self.runtime.spawn(async move {
            let Some(entry) = state.get(ino).await else {
                reply.error(Errno::ENOENT);
                return;
            };

            if entry.attr().kind != FileType::Directory {
                reply.error(Errno::ENOTDIR);
                return;
            }

            reply.opened(FileHandle(ino.0), FopenFlags::empty());
        });
    }

    fn getxattr(
        &self,
        _req: &Request,
        _ino: INodeNo,
        _name: &OsStr,
        _size: u32,
        reply: ReplyXattr,
    ) {
        reply.error(Errno::ENOTSUP);
    }

    fn readdir(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        offset: u64,
        mut reply: ReplyDirectory,
    ) {
        if ino == PLAYLISTS_NODE_ID {
            for (index, (name, attr)) in self.playlists().enumerate() {
                let _ = reply.add(attr.ino, (index + 2) as u64, attr.kind, name);
            }

            reply.ok();
            return;
        }

        if ino == LOST_AND_FOUND_NODE_ID {
            for (index, (name, attr)) in self.lost_and_found().enumerate() {
                let _ = reply.add(attr.ino, (index + 2) as u64, attr.kind, name);
            }

            reply.ok();
            return;
        }

        let state = self.state.clone();
        self.runtime.spawn(async move {
            let Some(parent_entry) = state.get(ino).await else {
                return reply.error(Errno::ENOENT);
            };

            let Entry::Real {
                entry: FolderEntry::Folder(folder),
                ..
            } = parent_entry
            else {
                return reply.error(Errno::ENOTDIR);
            };

            if offset == 0 {
                let _ = reply.add(ino, 1, FileType::Directory, ".");
            }
            if offset <= 1 {
                let parent_ino = state.handle_to_ino(folder.parent());
                let _ = reply.add(parent_ino, 2, FileType::Directory, "..");
            }

            let child_offset = if offset < 2 { 0 } else { (offset - 2) as usize };

            folder
                .with_children(|children| {
                    let mut children: Vec<FolderEntry<DeviceHandle>> =
                        children.values().cloned().collect();
                    children.sort_by_cached_key(|e| e.name().to_string());

                    async move {
                        for (i, child) in children.into_iter().skip(child_offset).enumerate() {
                            let child_ino = state.insert(child.clone()).await;
                            let kind = match child {
                                FolderEntry::Folder(_) => FileType::Directory,
                                FolderEntry::File(_) => FileType::RegularFile,
                            };

                            let reply_offset = child_offset as u64 + i as u64 + 3;
                            if reply.add(child_ino, reply_offset, kind, child.name()) {
                                break;
                            }
                        }

                        reply.ok();
                    }
                })
                .await;
        });
    }

    fn statfs(&self, _req: &Request, _ino: INodeNo, reply: ReplyStatfs) {
        let blocks = self.storage.max_capacity / u64::from(BLOCK_SIZE);
        let blocks_free = self.storage.free_space / u64::from(BLOCK_SIZE);
        let blocks_available = blocks_free;
        let files_free = self.storage.free_space_in_objects / BLOCK_SIZE;
        reply.statfs(
            blocks,
            blocks_free,
            blocks_available,
            0,
            u64::from(files_free),
            BLOCK_SIZE,
            0,
            0,
        );
    }
}

trait ToErrno {
    fn to_errno(&self) -> Errno;
}

impl ToErrno for MtpError<Arc<UsbError>> {
    fn to_errno(&self) -> Errno {
        if let MtpError::Transport(e) = self {
            if let UsbError::Native(e) = &**e {
                return e
                    .os_error()
                    .map(|e| Errno::from_i32(e as i32))
                    .unwrap_or(Errno::EIO);
            }
        }

        Errno::EIO
    }
}

impl ToErrno for std::io::Error {
    fn to_errno(&self) -> Errno {
        self.raw_os_error()
            .map(|e| Errno::from_i32(e))
            .unwrap_or(Errno::EIO)
    }
}

impl ToErrno for Error {
    fn to_errno(&self) -> Errno {
        match self {
            Error::Io(e) => e.to_errno(),
            Error::Core(e) => e.to_errno(),
            Error::EventStream(_) | Error::Generic(_) => Errno::EIO,
        }
    }
}
