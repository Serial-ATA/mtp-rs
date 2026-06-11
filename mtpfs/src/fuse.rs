use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{Read, Seek, SeekFrom, Write};
use std::iter;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

use fuser::{
    BsdFileFlags, Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation,
    INodeNo, KernelConfig, LockOwner, OpenFlags, RenameFlags, ReplyAttr, ReplyCreate, ReplyData,
    ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, ReplyXattr,
    Request, TimeOrNow, WriteFlags,
};
use indicatif::{ProgressBar, ProgressStyle};
use mtp::device::Device;
use mtp::device::extensions::android::AndroidDevice;
use mtp::device::session::MtpSession;
use mtp::error::MtpError;
use mtp::high_level::fs::{
    File, FileSystem, FileSystemError, Folder, FolderEntry, FsEvent, SessionFsExt,
};
use mtp::high_level::storages::Storage;
use mtp::object::{DateTime, ObjectFormatCode, ObjectHandle};
use mtp::usb::DeviceHandle;
use mtp::usb::error::{Error, UsbError};
use tokio::runtime::Handle;
use tokio::sync::{Mutex, OnceCell, RwLock};
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

#[derive(Clone, Debug)]
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
    /// A pending entry that hasn't yet been committed to the device
    ///
    /// Every interaction with the entry will occur on the host tempfile until it's released
    /// and ultimately written to the device.
    Pending(PendingEntry),
}

#[derive(Clone, Debug)]
struct PendingEntry {
    ino: INodeNo,
    parent: INodeNo,
    name: Arc<str>,
    mtime: SystemTime,
    crtime: SystemTime,
    spool: Arc<Mutex<std::fs::File>>,
}

impl PendingEntry {
    fn new(ino: INodeNo, parent: INodeNo, name: Arc<str>) -> Result<Self, Errno> {
        let time = SystemTime::now();
        Ok(PendingEntry {
            ino,
            parent,
            name,
            mtime: time,
            crtime: time,
            spool: Arc::new(Mutex::new(tempfile::tempfile().map_err(|e| e.to_errno())?)),
        })
    }
}

impl Entry {
    /// Get the file name for this entry
    fn name(&self) -> &str {
        match self {
            Self::Real { entry, .. } => entry.name(),
            Self::Virtual { name, .. } => &name,
            Self::Pending(entry) => &entry.name,
        }
    }

    /// Get the virtual inode number for this entry
    fn ino(&self) -> INodeNo {
        match self {
            Self::Real { ino, .. } => *ino,
            Self::Virtual { ino, .. } => *ino,
            Self::Pending(entry) => entry.ino,
        }
    }

    /// Get the parent inode number for this entry
    fn parent(&self) -> INodeNo {
        match self {
            Self::Real { entry, .. } => handle_to_ino(entry.parent()),
            Self::Virtual { .. } => panic!("Virtual entries do not have parents"),
            Self::Pending(entry) => entry.parent,
        }
    }

    /// Create the [`FileAttr`] for this entry
    ///
    /// # Errors
    ///
    /// This can only fail for pending entries, as we're doing an actual fetch from the host FS.
    async fn attr(&self) -> Result<FileAttr, Errno> {
        match self {
            Entry::Real { entry, ino } => match entry {
                FolderEntry::File(file) => Ok(FileAttr {
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
                }),
                FolderEntry::Folder(folder) => Ok(FileAttr {
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
                }),
            },
            Entry::Virtual { ino, .. } => match *ino {
                PLAYLISTS_NODE_ID => Ok(PLAYLISTS_ATTR),
                LOST_AND_FOUND_NODE_ID => Ok(LOST_AND_FOUND_ATTR),
                _ => Ok(ROOT_ATTR),
            },
            Entry::Pending(PendingEntry {
                ino,
                mtime,
                crtime,
                spool,
                ..
            }) => {
                let size = spool
                    .lock()
                    .await
                    .metadata()
                    .map_err(|e| e.to_errno())?
                    .len();
                Ok(FileAttr {
                    ino: *ino,
                    size,
                    blocks: (size + u64::from(BLOCK_SIZE) - 1) / u64::from(BLOCK_SIZE),
                    atime: SystemTime::UNIX_EPOCH,
                    mtime: *mtime,
                    ctime: SystemTime::UNIX_EPOCH,
                    crtime: *crtime,
                    kind: FileType::RegularFile,
                    perm: 0o666,
                    nlink: 1,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    blksize: BLOCK_SIZE,
                    flags: 0,
                })
            },
        }
    }
}

struct FsState {
    nodes: RwLock<HashMap<INodeNo, Entry>>,
    fs: OnceCell<Arc<FileSystem<DeviceHandle>>>,
    next_pending_ino: AtomicU64,
}

impl Default for FsState {
    fn default() -> Self {
        Self {
            nodes: Default::default(),
            fs: Default::default(),
            next_pending_ino: AtomicU64::new(PENDING_INODE_OFFSET),
        }
    }
}

/// The offset at which non-virtual inodes start
const MTP_INODE_OFFSET: u64 = 10;

/// Convert an MTP [`ObjectHandle`] to a FUSE inode
fn handle_to_ino(handle: ObjectHandle) -> INodeNo {
    if handle == ObjectHandle::NONE {
        ROOT_ATTR.ino
    } else {
        INodeNo(u64::from(Into::<u32>::into(handle)) + MTP_INODE_OFFSET)
    }
}

/// The offset at which pending entries start
///
/// Since MTP object handles are 32-bit, we can just use the upper 32 bits
const PENDING_INODE_OFFSET: u64 = u32::MAX as u64 + 1;

impl FsState {
    /// Get an [`Entry`] by its inode number
    async fn get(&self, inode: INodeNo) -> Option<Entry> {
        self.nodes.read().await.get(&inode).cloned()
    }

    /// Demote an `Entry::Real` to `Entry::Pending`.
    async fn demote_to_pending(&self, ino: INodeNo) -> Result<Entry, Errno> {
        let mut map = self.nodes.write().await;
        match map.get_mut(&ino) {
            Some(entry @ Entry::Real { .. }) => {
                let parent = entry.parent();
                let _dir = self.dir(parent).await?; // Sanity

                let name = entry.name().into();
                *entry = Entry::Pending(PendingEntry::new(ino, parent, name)?);
                Ok(entry.clone())
            },
            Some(Entry::Virtual { .. }) => Err(Errno::EINVAL),
            Some(entry @ Entry::Pending(_)) => Ok(entry.clone()),
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

    /// Create a new [`Entry::Pending`]
    ///
    /// # Errors
    ///
    /// Unable to create the backing tempfile
    async fn new_pending(&self, parent: INodeNo, name: Arc<str>) -> Result<Entry, Errno> {
        let ino = INodeNo(self.next_pending_ino.fetch_add(1, Ordering::Relaxed));
        let entry = Entry::Pending(PendingEntry::new(ino, parent, name)?);
        self.nodes.write().await.insert(ino, entry.clone());
        Ok(entry)
    }

    async fn find_pending_entry_in(&self, parent: INodeNo, name: &str) -> Option<PendingEntry> {
        let nodes = self.nodes.read().await;
        nodes.iter().find_map(|(&ino, entry)| {
            if let Entry::Pending(p) = entry {
                if p.parent == parent && &*p.name == name {
                    return Some(p.clone());
                }
            }
            None
        })
    }

    /// Insert a new [`FolderEntry`]
    async fn insert(&self, entry: FolderEntry<DeviceHandle>) -> INodeNo {
        let handle = entry.handle();
        let ino = handle_to_ino(handle);
        self.nodes
            .write()
            .await
            .insert(ino, Entry::Real { entry, ino });
        ino
    }

    async fn update(&self, entry: FolderEntry<DeviceHandle>) {
        let handle = entry.handle();
        let ino = handle_to_ino(handle);
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
    async fn refresh(state: Arc<Self>, fs: Arc<FileSystem<DeviceHandle>>) {
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

        assert!(state.fs.set(fs.clone()).is_ok(), "should be set once");

        state
            .nodes
            .write()
            .await
            .retain(|&ino, _| ino.0 < MTP_INODE_OFFSET); // Keep virtual nodes
        fs.with_root(|root| walk_and_insert(state, FolderEntry::Folder(root.clone())))
            .await;
    }
}

pub struct MtpFuse {
    session: MtpSession<DeviceHandle>,
    storage: Storage,
    is_android: bool,

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

        let state = Arc::new(FsState::default());

        Ok(MtpFuse {
            session,
            storage,
            is_android,
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
                        FsState::refresh(state.clone(), fs).await;

                        tokio::task::spawn(async move {
                            while let Some(event) = events.recv().await {
                                match event {
                                    FsEvent::Added(entry) => {
                                        state.insert(entry).await;
                                    },
                                    FsEvent::Removed(entry) => {
                                        state.remove(handle_to_ino(entry.handle())).await;
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
    }

    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let Some(name_str) = name.to_str().map(String::from) else {
            return reply.error(Errno::EINVAL);
        };

        let state = self.state.clone();
        self.runtime.spawn(async move {
            let parent_dir = match state.dir(parent).await {
                Ok(parent_entry) => parent_entry,
                Err(e) => {
                    return reply.error(e);
                },
            };

            if let Some(pending) = state.find_pending_entry_in(parent, &name_str).await {
                match Entry::Pending(pending).attr().await {
                    Ok(attr) => {
                        return reply.entry(&TTL, &attr, Generation(0));
                    },
                    Err(e) => {
                        return reply.error(e);
                    },
                }
            }

            let Some(child) = parent_dir.find(&name_str).await else {
                return reply.error(Errno::ENOENT);
            };

            let ino = state.insert(child.clone()).await;
            let attr = Entry::Real { entry: child, ino }
                .attr()
                .await
                .expect("attr should not fail for real entries");
            reply.entry(&TTL, &attr, Generation(0));
        });
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        let state = self.state.clone();
        self.runtime.spawn(async move {
            let Some(entry) = state.get(ino).await else {
                return reply.error(Errno::ENOENT);
            };

            match entry.attr().await {
                Ok(attr) => reply.attr(&TTL, &attr),
                Err(e) => reply.error(e),
            }
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
                    let attr = Entry::Real { entry, ino }
                        .attr()
                        .await
                        .expect("attr should not fail for real entries");
                    reply.entry(&TTL, &attr, Generation(0));
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
            let parent_dir = match state.dir(parent).await {
                Ok(parent) => parent,
                Err(e) => return reply.error(e),
            };

            // Just drop the entry from memory if it's pending
            if let Some(pending) = state.find_pending_entry_in(parent, &name_str).await {
                state.remove(pending.ino).await;
                return reply.ok();
            }

            // Otherwise, physically remove it from the device
            let Some(child_entry) = parent_dir.find(&name_str).await else {
                return reply.error(Errno::ENOENT);
            };

            let FolderEntry::File(child) = child_entry else {
                return reply.error(Errno::EISDIR);
            };

            let result = parent_dir.remove_child(&name_str).await;

            match result {
                Ok(_) => {
                    state.remove(handle_to_ino(child.id())).await;
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
                    state.remove(handle_to_ino(child.id())).await;
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

            {
                if let Some(pending) = state.find_pending_entry_in(parent, &name_str).await {
                    let now = SystemTime::now();
                    let mut nodes = state.nodes.write().await;
                    return match nodes.get_mut(&pending.ino) {
                        Some(Entry::Pending(PendingEntry {
                            parent,
                            name,
                            mtime,
                            ..
                        })) => {
                            *parent = newparent;
                            *name = new_name_str.into();
                            *mtime = now;
                            reply.ok()
                        },
                        Some(Entry::Real {
                            entry: FolderEntry::Folder(_),
                            ..
                        }) => reply.error(Errno::EISDIR),
                        Some(_) => reply.error(Errno::EINVAL),
                        None => reply.error(Errno::ENOENT),
                    };
                }
            }

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

            state.remove(handle_to_ino(child.handle())).await;
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
            match state.get(ino).await {
                Some(Entry::Pending(_)) => {
                    reply.opened(
                        FileHandle(0),
                        FopenFlags::from_bits_truncate(flags.0 as u32),
                    );
                },
                Some(Entry::Real {
                    entry: FolderEntry::File(file),
                    ..
                }) => match session.get_object_info(file.id()).await {
                    Ok(_fd) => reply.opened(
                        FileHandle(0),
                        FopenFlags::from_bits_truncate(flags.0 as u32),
                    ),
                    Err(e) => reply.error(e.to_errno()),
                },
                Some(Entry::Real {
                    entry: FolderEntry::Folder(_),
                    ..
                }) => reply.error(Errno::EISDIR),
                Some(_) => reply.error(Errno::EINVAL),
                None => reply.error(Errno::ENOENT),
            }
        });
    }

    fn create(
        &self,
        _req: &Request,
        parent: INodeNo,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        let Some(name_str) = name.to_str().map(String::from) else {
            return reply.error(Errno::EINVAL);
        };

        let state = self.state.clone();
        self.runtime.spawn(async move {
            if let Err(e) = state.dir(parent).await {
                return reply.error(e);
            };

            match state.new_pending(parent, name_str.into()).await {
                Ok(entry) => match entry.attr().await {
                    Ok(attr) => {
                        reply.created(
                            &TTL,
                            &attr,
                            Generation(0),
                            FileHandle(entry.ino().0),
                            FopenFlags::empty(),
                        );
                    },
                    Err(e) => return reply.error(e),
                },
                Err(e) => {
                    return reply.error(e);
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
            match state.get(ino).await {
                Some(Entry::Pending(p)) => {
                    let mut spool = p.spool.lock().await;
                    let mut buffer = vec![0; size as usize];
                    match spool.seek(SeekFrom::Start(offset)) {
                        Ok(_) => match spool.read(&mut buffer) {
                            Ok(bytes_read) => reply.data(&buffer[..bytes_read]),
                            Err(e) => reply.error(e.to_errno()),
                        },
                        Err(e) => reply.error(e.to_errno()),
                    }
                },
                Some(Entry::Real {
                    entry: FolderEntry::File(file),
                    ..
                }) => match file.read_at(offset as u32, size).await {
                    Ok(data) => reply.data(&data),
                    Err(Error::Core(MtpError::UnsupportedOperation)) => {
                        tracing::warn!("TODO: support non-GetPartialObject reads");
                        reply.error(Errno::ENOTSUP);
                    },
                    Err(e) => reply.error(e.to_errno()),
                },
                Some(Entry::Real {
                    entry: FolderEntry::Folder(_),
                    ..
                }) => reply.error(Errno::EISDIR),
                Some(_) => reply.error(Errno::EINVAL),
                None => reply.error(Errno::ENOENT),
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
        let state = self.state.clone();
        let is_android = self.is_android;
        let data = data.to_vec();
        self.runtime.spawn(async move {
            match state.get(ino).await {
                Some(Entry::Pending(PendingEntry { spool, name, .. })) => {
                    {
                        let mut spool = spool.lock().await;
                        if let Err(e) = spool.seek(SeekFrom::Start(offset)) {
                            return reply.error(e.to_errno());
                        }
                        if let Err(e) = spool.write_all(&data) {
                            return reply.error(e.to_errno());
                        }
                    }

                    let now = SystemTime::now();
                    if let Some(Entry::Pending(PendingEntry { mtime, .. })) =
                        state.nodes.write().await.get_mut(&ino)
                    {
                        *mtime = now;
                    }

                    return reply.written(data.len() as u32);
                },
                Some(Entry::Real {
                    entry: FolderEntry::File(file),
                    ..
                }) => {
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

                    // Otherwise, demote to a pending file and accumulate writes till `release()`

                    match dbg!(state.demote_to_pending(ino).await) {
                        Ok(_) => {
                            reply.written(data.len() as u32);
                            return;
                        },
                        Err(e) => {
                            reply.error(e);
                            return;
                        },
                    }
                },
                Some(Entry::Real {
                    entry: FolderEntry::Folder(_),
                    ..
                }) => reply.error(Errno::EISDIR),
                Some(Entry::Virtual { .. }) => reply.error(Errno::EINVAL),
                None => reply.error(Errno::ENOENT),
            }
        });
    }

    fn setattr(
        &self,
        _req: &Request,
        ino: INodeNo,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        _fh: Option<FileHandle>,
        crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<BsdFileFlags>,
        reply: ReplyAttr,
    ) {
        let state = self.state.clone();
        self.runtime.spawn(async move {
            let mut entry = match state.get(ino).await {
                Some(e) => e,
                None => return reply.error(Errno::ENOENT),
            };

            if let Some(size) = size {
                match &mut entry {
                    Entry::Pending(pending) => {
                        if let Err(e) = pending.spool.lock().await.set_len(size) {
                            return reply.error(e.to_errno());
                        }
                    },
                    Entry::Real {
                        entry: FolderEntry::File(_),
                        ..
                    } => {
                        tracing::warn!("TODO: set size on existing files");
                        return reply.error(Errno::ENOTSUP);
                    },
                    Entry::Real {
                        entry: FolderEntry::Folder(_),
                        ..
                    } => return reply.error(Errno::EISDIR),
                    Entry::Virtual { .. } => return reply.error(Errno::EINVAL),
                }
            }

            if let Some(time) = mtime {
                if let Entry::Pending(_) = &mut entry {
                    let mut nodes = state.nodes.write().await;
                    if let Some(Entry::Pending(p_mut)) = nodes.get_mut(&ino) {
                        p_mut.mtime = match time {
                            TimeOrNow::SpecificTime(time) => time,
                            TimeOrNow::Now => SystemTime::now(),
                        };
                        entry = Entry::Pending(p_mut.clone());
                    }
                }
            }

            if let Some(time) = crtime {
                if let Entry::Pending(_) = &mut entry {
                    let mut nodes = state.nodes.write().await;
                    if let Some(Entry::Pending(p_mut)) = nodes.get_mut(&ino) {
                        p_mut.crtime = time;
                        entry = Entry::Pending(p_mut.clone());
                    }
                }
            }

            match entry.attr().await {
                Ok(attr) => reply.attr(&TTL, &attr),
                Err(e) => reply.error(e),
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

    fn release(
        &self,
        _req: &Request,
        ino: INodeNo,
        _fh: FileHandle,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        let state = self.state.clone();
        self.runtime.spawn(async move {
            let (ino, parent, name, spool) = match state.get(ino).await {
                Some(Entry::Pending(PendingEntry {
                    ino,
                    parent,
                    name,
                    mtime: _,
                    crtime: _,
                    spool,
                })) => (ino, parent, name, spool),
                // Nothing to do
                Some(_) => return reply.ok(),
                None => return reply.error(Errno::ENOENT),
            };

            let parent = match state.dir(parent).await {
                Ok(parent) => parent,
                Err(e) => {
                    reply.error(e);
                    return;
                },
            };

            let mut object_data = Vec::new();
            {
                let mut spool = spool.lock().await;

                // Don't commit empty files to the device. Android, at the very least, gets
                // tripped up and hangs forever waiting for data. Sad...
                let size = spool.metadata().map(|m| m.len()).unwrap_or(0);
                if size == 0 {
                    return reply.ok();
                }

                if let Err(e) = spool.seek(SeekFrom::Start(0)) {
                    reply.error(e.to_errno());
                    return;
                }

                if let Err(e) = spool.read_to_end(&mut object_data) {
                    reply.error(e.to_errno());
                    return;
                }
            }

            let mut format_code = ObjectFormatCode::Undefined;
            if let Some(ext) = name.rsplit('.').next() {
                format_code = ObjectFormatCode::from_extension(ext);
            }

            tracing::error!(
                "CREATING FILE: {name} (size: {}, format: {format_code:?})",
                object_data.len()
            );
            match parent
                .create_file(name.as_ref(), format_code, object_data)
                .await
            {
                Ok(file) => {
                    state.remove(ino).await;
                    state.insert(FolderEntry::File(file)).await;
                    reply.ok();
                },
                Err(e) => {
                    // Will also keep the temp file around, just in case
                    reply.error(dbg!(e).to_errno());
                },
            }
        });
    }

    fn opendir(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        let state = self.state.clone();
        self.runtime.spawn(async move {
            let Some(entry) = state.get(ino).await else {
                reply.error(Errno::ENOENT);
                return;
            };

            let attr = match entry.attr().await {
                Ok(attr) => attr,
                Err(e) => {
                    return reply.error(e);
                },
            };

            if attr.kind != FileType::Directory {
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

            let parent_ino = handle_to_ino(folder.parent());
            let mut entries = vec![
                (ino, FileType::Directory, String::from(".")),
                (parent_ino, FileType::Directory, String::from("..")),
            ];

            let mut pending_entries = state
                .nodes
                .read()
                .await
                .values()
                .filter_map(|entry| match entry {
                    Entry::Pending(pending) if pending.parent == ino => {
                        Some((pending.ino, FileType::RegularFile, pending.name.to_string()))
                    },
                    _ => None,
                })
                .collect::<Vec<_>>();
            pending_entries.sort_by(|a, b| a.2.cmp(&b.2));
            entries.extend(pending_entries);

            folder
                .with_children(|children| {
                    let mut children: Vec<FolderEntry<DeviceHandle>> =
                        children.values().cloned().collect();
                    children.sort_by_cached_key(|e| e.name().to_string());

                    async move {
                        for child in children {
                            let child_ino = state.insert(child.clone()).await;
                            let kind = match child {
                                FolderEntry::Folder(_) => FileType::Directory,
                                FolderEntry::File(_) => FileType::RegularFile,
                            };
                            entries.push((child_ino, kind, child.name().to_string()));
                        }

                        for (index, (child_ino, kind, name)) in
                            entries.into_iter().enumerate().skip(offset as usize)
                        {
                            if reply.add(child_ino, (index + 1) as u64, kind, name) {
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
