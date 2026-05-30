use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{Read, Seek, SeekFrom, Write};
use std::iter;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};

use fuser::{
    Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation, INodeNo,
    KernelConfig, LockOwner, OpenFlags, RenameFlags, ReplyAttr, ReplyData, ReplyDirectory,
    ReplyEmpty, ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, Request, WriteFlags,
};
use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use mtp::communication::response::SendObjectInfo;
use mtp::device::Device;
use mtp::device::extensions::android::AndroidDevice;
use mtp::device::session::MtpSession;
use mtp::error::MtpError;
use mtp::high_level::fs::{File, FileSystem, FileSystemError, Folder, FolderEntry, SessionFsExt};
use mtp::high_level::storages::Storage;
use mtp::object::{DateTime, ObjectHandle};
use mtp::usb::DeviceHandle;
use mtp::usb::error::{Error, UsbError};
use tokio::sync::Mutex;

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

struct InjectedEntry {
    ino: INodeNo,
    name: Arc<str>,
}

#[derive(Clone)]
enum Entry {
    Real {
        ino: INodeNo,
        entry: FolderEntry,
    },
    Injected(Arc<InjectedEntry>),
    /// This is a temporary state. It is not valid for an entry to remain in this state by the end of an operation.
    Empty,
}

impl Entry {
    fn name(&self) -> &str {
        match self {
            Self::Real { entry, .. } => entry.name(),
            Self::Injected(entry) => &entry.name,
            Self::Empty => unreachable!(),
        }
    }

    fn ino(&self) -> INodeNo {
        match self {
            Self::Real { ino, .. } => *ino,
            Self::Injected(entry) => entry.ino,
            Self::Empty => unreachable!(),
        }
    }

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
            Entry::Injected(entry) => match entry.ino {
                PLAYLISTS_NODE_ID => PLAYLISTS_ATTR,
                LOST_AND_FOUND_NODE_ID => LOST_AND_FOUND_ATTR,
                _ => ROOT_ATTR,
            },
            Entry::Empty => unreachable!(),
        }
    }
}

struct Dirty {
    playlists: bool,
    lost_and_found: bool,
    fs: bool,
}

impl Dirty {
    fn new() -> Self {
        // Everything needs an initial refresh
        Self {
            playlists: true,
            lost_and_found: true,
            fs: true,
        }
    }
}

#[derive(Default)]
struct FsInner {
    nodes: RwLock<HashMap<INodeNo, Entry>>,
}

/// The offset at which non-virtual inodes start
const MTP_INODE_OFFSET: u64 = 10;

impl FsInner {
    /// Get an [`Entry`] by its inode number
    fn get(&self, inode: INodeNo) -> Option<Entry> {
        self.nodes.read().unwrap().get(&inode).cloned()
    }

    /// Insert a new [`FolderEntry`]
    fn insert(&self, entry: FolderEntry) -> INodeNo {
        let handle = entry.handle();
        let ino = self.handle_to_ino(handle);
        self.nodes
            .write()
            .unwrap()
            .insert(ino, Entry::Real { entry, ino });
        ino
    }

    /// Remove a node from the map
    fn remove(&self, inode: INodeNo) {
        self.nodes.write().unwrap().remove(&inode);
    }

    /// Refresh from the current state of the [`FileSystem`]
    fn refresh(&mut self, fs: &FileSystem) {
        fn walk_and_insert(fs: &mut FsInner, entry: FolderEntry) {
            fs.insert(entry.clone());
            if let FolderEntry::Folder(folder) = entry {
                folder.with_children(|children| {
                    for child in children.values() {
                        walk_and_insert(fs, child.clone());
                    }
                });
            }
        }

        self.nodes
            .write()
            .unwrap()
            .retain(|&ino, _| ino.0 < MTP_INODE_OFFSET); // Keep virtual nodes
        fs.with_root(|root| {
            walk_and_insert(self, FolderEntry::Folder(root.clone()));
        });
    }

    /// Convert an MTP ObjectHandle to a FUSE inode
    fn handle_to_ino(&self, handle: ObjectHandle) -> INodeNo {
        if handle == ObjectHandle::NONE {
            ROOT_ATTR.ino
        } else {
            INodeNo(u64::from(Into::<u32>::into(handle)) + MTP_INODE_OFFSET)
        }
    }

    /// Convert a FUSE inode back to an MTP ObjectHandle
    fn ino_to_handle(&self, ino: INodeNo) -> Option<ObjectHandle> {
        if ino == ROOT_ATTR.ino {
            return Some(ObjectHandle::NONE);
        }
        if ino.0 < MTP_INODE_OFFSET {
            // Virtual inodes don't actually map to anything on the device
            return None;
        }
        Some(ObjectHandle::from((ino.0 - MTP_INODE_OFFSET) as u32))
    }
}

pub struct MtpFuse {
    session: Arc<Mutex<MtpSession<DeviceHandle>>>,
    storage: Storage,
    is_android: bool,
    dirty: Dirty,

    fs: Option<Arc<FileSystem>>,
    inner: FsInner,
}

impl MtpFuse {
    pub async fn new(
        session: Arc<Mutex<MtpSession<DeviceHandle>>>,
        storage: Storage,
    ) -> mtp::usb::error::Result<Self> {
        let is_android = session.lock().await.is_android().await?;
        if !is_android {
            log::warn!(
                "The selected device doesn't support the Android MTP extensions. Writes will \
                 likely have bad performance."
            )
        }

        let mut nodes = HashMap::new();
        for virtual_dir in [
            Entry::Injected(Arc::new(InjectedEntry {
                ino: ROOT_ATTR.ino,
                name: "/".into(),
            })),
            Entry::Injected(Arc::new(InjectedEntry {
                ino: PLAYLISTS_NODE_ID,
                name: "Playlists".into(),
            })),
            Entry::Injected(Arc::new(InjectedEntry {
                ino: LOST_AND_FOUND_NODE_ID,
                name: "lost+found".into(),
            })),
        ] {
            nodes.insert(virtual_dir.ino(), virtual_dir);
        }

        Ok(MtpFuse {
            session,
            storage,
            is_android,
            dirty: Dirty::new(),
            fs: None,
            inner: FsInner {
                nodes: RwLock::new(nodes),
            },
        })
    }

    fn stat(&self, inode: INodeNo, _file_handle: Option<FileHandle>) -> Option<FileAttr> {
        if inode == INodeNo(0) {
            return None;
        }

        if inode == ROOT_ATTR.ino {
            return Some(ROOT_ATTR);
        }

        if inode == PLAYLISTS_NODE_ID {
            return Some(PLAYLISTS_ATTR);
        }

        if inode == LOST_AND_FOUND_NODE_ID {
            return Some(LOST_AND_FOUND_ATTR);
        }

        self.inner.get(inode).as_ref().map(Entry::attr)
    }

    fn file(&self, ino: INodeNo) -> Result<Arc<File>, Errno> {
        match self.inner.get(ino) {
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

    fn dir(&self, ino: INodeNo) -> Result<Arc<Folder>, Errno> {
        match self.inner.get(ino) {
            Some(Entry::Real {
                entry: FolderEntry::Folder(folder),
                ..
            }) => Ok(folder),
            Some(_) => Err(Errno::ENOTDIR),
            None => Err(Errno::ENOENT),
        }
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

        let fs_result = futures::executor::block_on(async {
            let mut session = self.session.lock().await;

            let spinner = ProgressBar::new_spinner()
                .with_message("Loading all MTP directories into memory")
                .with_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());
            let fs = FileSystem::load_with_callback(&mut *session, self.storage.id, |_| {
                spinner.tick();
            })
            .await;

            spinner.finish_and_clear();
            fs
        });

        match fs_result {
            Ok(fs) => {
                self.inner.refresh(&fs);

                let storage_name = self
                    .storage
                    .description
                    .as_deref()
                    .unwrap_or(self.storage.volume_identifier.as_str());

                let load_time = start.elapsed();
                let load_time_seconds = load_time.as_secs() % 60;

                info!(
                    "Finished loading filesystem for storage `{storage_name}` in {:02}:{:02}",
                    (load_time.as_secs() - load_time_seconds) / 60,
                    load_time_seconds
                );

                self.fs = Some(fs);
                self.dirty.fs = false;

                Ok(())
            },
            Err(e) => {
                log::error!("Failed to initialize MTP FUSE mount: {e}");
                Err(std::io::Error::from_raw_os_error(e.to_errno().code()))
            },
        }
    }

    fn destroy(&mut self) {
        log::info!("Closing filesystem");
        self.inner = FsInner::default();
        let _ = self.fs.take();
    }

    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let Some(name_str) = name.to_str() else {
            return reply.error(Errno::EINVAL);
        };

        let Some(parent_entry) = self.inner.get(parent) else {
            return reply.error(Errno::ENOENT);
        };

        let Entry::Real {
            entry: FolderEntry::Folder(folder),
            ..
        } = parent_entry
        else {
            return reply.error(Errno::ENOTDIR);
        };

        let Some(child) = folder.find(name_str) else {
            return reply.error(Errno::ENOENT);
        };

        let ino = self.inner.insert(child.clone());
        reply.entry(
            &TTL,
            &Entry::Real { entry: child, ino }.attr(),
            Generation(0),
        );
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        if ino == INodeNo(0) {
            return reply.error(Errno::ENOENT);
        }

        let Some(entry) = self.inner.get(ino) else {
            return reply.error(Errno::ENOENT);
        };

        reply.attr(&TTL, &entry.attr())
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
        let Some(name_str) = name.to_str() else {
            return reply.error(Errno::EINVAL);
        };

        let Some(Entry::Real {
            entry: FolderEntry::Folder(parent_dir),
            ..
        }) = self.inner.get(parent)
        else {
            return reply.error(Errno::ENOTDIR);
        };

        let result = futures::executor::block_on(async {
            let mut session = self.session.lock().await;
            session
                .mkdir(Some(parent_dir.as_ref()), name_str.to_string())
                .await
        });

        match result {
            Ok(folder) => {
                let entry = FolderEntry::Folder(Arc::new(folder));
                let ino = self.inner.insert(entry.clone());
                reply.entry(&TTL, &Entry::Real { entry, ino }.attr(), Generation(0));
            },
            Err(e) => reply.error(e.to_errno()),
        }
    }

    // TODO: Figure out why the lookups still resolve in rmdir and unlink even after destroy() is called
    fn unlink(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let Some(name_str) = name.to_str() else {
            return reply.error(Errno::EINVAL);
        };

        let parent = match self.dir(parent) {
            Ok(parent) => parent,
            Err(e) => return reply.error(e),
        };

        let Some(child_entry) = parent.find(name_str) else {
            return reply.error(Errno::ENOENT);
        };

        let FolderEntry::File(child) = child_entry else {
            return reply.error(Errno::EISDIR);
        };

        let result = futures::executor::block_on(async {
            let mut session = self.session.lock().await;
            parent.remove_child(&mut session, name_str).await
        });

        match result {
            Ok(_) => {
                self.inner.remove(self.inner.handle_to_ino(child.id()));
                reply.ok();
            },
            Err(e) => match e {
                FileSystemError::Mtp(e) => reply.error(e.to_errno()),
                FileSystemError::Io(e) => reply.error(e.to_errno()),
            },
        }
    }

    fn rmdir(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let Some(name_str) = name.to_str() else {
            return reply.error(Errno::EINVAL);
        };

        let parent_dir = match self.dir(parent) {
            Ok(parent) => parent,
            Err(e) => return reply.error(e),
        };

        let Some(FolderEntry::Folder(child)) = parent_dir.find(name_str) else {
            return reply.error(Errno::ENOENT);
        };

        let result = futures::executor::block_on(async {
            let mut session = self.session.lock().await;
            parent_dir.remove_child(&mut session, name_str).await
        });

        match result {
            Ok(_) => {
                self.inner.remove(self.inner.handle_to_ino(child.id()));
                reply.ok();
            },
            Err(e) => match e {
                FileSystemError::Mtp(e) => reply.error(e.to_errno()),
                FileSystemError::Io(e) => reply.error(e.to_errno()),
            },
        }
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
        let Some(name_str) = name.to_str() else {
            return reply.error(Errno::EINVAL);
        };
        let Some(new_name_str) = newname.to_str() else {
            return reply.error(Errno::EINVAL);
        };

        let original_parent = match self.dir(parent) {
            Ok(folder) => folder,
            Err(e) => return reply.error(e),
        };

        let new_parent = match self.dir(newparent) {
            Ok(folder) => folder,
            Err(e) => return reply.error(e),
        };

        let Some(child) = original_parent.find(name_str) else {
            return reply.error(Errno::ENOENT);
        };

        let result = futures::executor::block_on(async {
            let mut session = self.session.lock().await;

            let mut current_child = child.clone();
            if parent != newparent {
                current_child = current_child
                    .move_(&mut *session, new_parent.as_ref())
                    .await?;
            }

            if name_str != new_name_str {
                current_child = current_child.rename(&mut *session, new_name_str).await?;
            }

            Ok::<FolderEntry, MtpError<_>>(current_child)
        });

        match result {
            Ok(updated_entry) => {
                self.inner.remove(self.inner.handle_to_ino(child.handle()));
                self.inner.insert(updated_entry);
                reply.ok();
            },
            Err(e) => reply.error(e.to_errno()),
        }
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
        let file = match self.file(ino) {
            Ok(file) => file,
            Err(e) => {
                reply.error(e);
                return;
            },
        };

        futures::executor::block_on(async {
            let mut session = self.session.lock().await;
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
        let file = match self.file(ino) {
            Ok(file) => file,
            Err(e) => {
                reply.error(e);
                return;
            },
        };

        futures::executor::block_on(async {
            let mut session = self.session.lock().await;
            match file.read_at(&mut *session, offset as u32, size).await {
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
        let file = match self.file(ino) {
            Ok(file) => file,
            Err(e) => {
                reply.error(e);
                return;
            },
        };

        futures::executor::block_on(async {
            let mut session = self.session.lock().await;

            // Android has a fast path, where we can actually edit files in-place
            if self.is_android {
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

            let mut temp = match file.open(&mut *session).await {
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

            if let Err(e) = temp.write_all(data) {
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

            let storage;
            let parent;
            let id;
            match session.send_object_info(object_info).await {
                Ok(response) => {
                    SendObjectInfo {
                        storage_id: storage,
                        parent,
                        reserved_handle: id,
                    } = response.data;
                },
                Err(e) => {
                    reply.error(e.to_errno());
                    return;
                },
            }

            match session.send_object(object_data).await {
                Ok(_) => {
                    reply.written(data.len() as u32);
                    return;
                },
                Err(e) => {
                    reply.error(e.to_errno());
                    return;
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
        let Some(entry) = self.inner.get(ino) else {
            reply.error(Errno::ENOENT);
            return;
        };

        if entry.attr().kind != FileType::Directory {
            reply.error(Errno::ENOTDIR);
            return;
        }

        reply.opened(FileHandle(ino.0), FopenFlags::empty());
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

        let Some(parent_entry) = self.inner.get(ino) else {
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
            let parent_ino = self.inner.handle_to_ino(folder.parent());
            let _ = reply.add(parent_ino, 2, FileType::Directory, "..");
        }

        let child_offset = if offset < 2 { 0 } else { (offset - 2) as usize };

        folder.with_children(|children| {
            let mut keys: Vec<&String> = children.keys().collect();
            keys.sort();

            for (i, key) in keys.into_iter().skip(child_offset).enumerate() {
                let child = &children[key];

                let child_ino = self.inner.insert(child.clone());
                let kind = match child {
                    FolderEntry::Folder(_) => FileType::Directory,
                    FolderEntry::File(_) => FileType::RegularFile,
                };

                let reply_offset = child_offset as u64 + i as u64 + 3;
                if reply.add(child_ino, reply_offset, kind, child.name()) {
                    break;
                }
            }
        });

        reply.ok();
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
