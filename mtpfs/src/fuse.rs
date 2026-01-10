use std::ffi::OsStr;
use std::fmt::{Debug, Formatter};
use std::io::{Read, Seek, SeekFrom, Write};
use std::iter;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};

use fuser::{
    FUSE_ROOT_ID, FileAttr, FileType, Filesystem, KernelConfig, ReplyAttr, ReplyData,
    ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, Request,
};
use id_tree::{InsertBehavior, Node, NodeId, RemoveBehavior, Tree};
use indicatif::{ProgressBar, ProgressStyle};
use libc::{EINVAL, EIO, ENOENT, ENOTDIR, ENOTSUP, c_int};
use log::info;
use mtp::communication::SessionId;
use mtp::communication::response::SendObjectInfo;
use mtp::device::Device;
use mtp::device::extensions::android::AndroidDevice;
use mtp::device::session::MtpSession;
use mtp::error::MtpError;
use mtp::high_level::fs::{File, FileSystem, FolderEntry, SessionFsExt};
use mtp::high_level::storages::Storage;
use mtp::object::types::{DateTime, ObjectHandle};
use mtp::usb::DeviceHandle;
use mtp::usb::error::{Error, UsbError};
use tokio::sync::Mutex;

const TTL: Duration = Duration::from_secs(u64::MAX);
const BLOCK_SIZE: u32 = 1024;

const ROOT_ATTR: FileAttr = FileAttr {
    ino: FUSE_ROOT_ID,
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

const PLAYLISTS_NODE_ID: u64 = 2;
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

const LOST_AND_FOUND_NODE_ID: u64 = 3;
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
    name: Arc<str>,
}

#[derive(Clone)]
enum Entry {
    Real(FolderEntry),
    Injected(Arc<InjectedEntry>),
    /// This is a temporary state. It is not valid for an entry to remain in this state by the end of an operation.
    Empty,
}

impl Entry {
    fn name(&self) -> &str {
        match self {
            Self::Real(entry) => entry.name(),
            Self::Injected(entry) => &entry.name,
            Self::Empty => unreachable!(),
        }
    }
}

#[derive(Clone)]
struct INode {
    parent: u64,
    object_handle: ObjectHandle,
    attr: FileAttr,
    entry: Entry,
}

impl Debug for INode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("INode")
            .field("inode", &self.attr.ino)
            .field("kind", &self.attr.kind)
            .field("parent", &self.parent)
            .field("object_handle", &self.object_handle)
            .finish_non_exhaustive()
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
    node_ids: Vec<NodeId>,
    next_inode: AtomicU64,
    tree: Tree<INode>,
}

impl FsInner {
    fn get_node_id(&self, inode: u64) -> Option<&NodeId> {
        self.node_ids.get((inode - FUSE_ROOT_ID) as usize)
    }

    fn get(&self, inode: u64) -> Option<(&INode, &NodeId)> {
        let node_id = self.get_node_id(inode)?;
        self.tree
            .get(node_id)
            .ok()
            .map(|node| (node.data(), node_id))
    }

    fn insert(
        &mut self,
        parent_inode: u64,
        mut attr: FileAttr,
        object_handle: ObjectHandle,
        entry: Entry,
    ) -> Option<u64> {
        let parent = self.get_node_id(parent_inode).cloned()?;

        let inode = self.next_inode.fetch_add(1, Ordering::Relaxed);
        attr.ino = inode;

        let new_inode = INode {
            parent: parent_inode,
            object_handle,
            attr,
            entry,
        };

        let new_inode_id = self
            .tree
            .insert(Node::new(new_inode), InsertBehavior::UnderNode(&parent))
            .unwrap();

        self.node_ids.push(new_inode_id);
        Some(inode)
    }

    fn update_entry(&mut self, inode: u64, entry: Entry) {
        let node_id = self.get_node_id(inode).expect("invalid node ID").clone();
        self.tree
            .get_mut(&node_id)
            .expect("node should exist")
            .data_mut()
            .entry = entry;
    }

    fn insert_entry(&mut self, parent_inode: u64, entry: FolderEntry) -> Option<u64> {
        match entry {
            FolderEntry::File(file) => self.insert(
                parent_inode,
                FileAttr {
                    ino: 0,
                    size: file.size,
                    blocks: 0,
                    atime: SystemTime::UNIX_EPOCH,
                    mtime: file
                        .date_modified
                        .and_then(DateTime::as_systemtime)
                        .unwrap_or(SystemTime::UNIX_EPOCH),
                    ctime: SystemTime::UNIX_EPOCH,
                    crtime: file
                        .date_created
                        .and_then(DateTime::as_systemtime)
                        .unwrap_or(SystemTime::UNIX_EPOCH),
                    kind: FileType::RegularFile,
                    perm: 0o777,
                    nlink: 1,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    blksize: 0,
                    flags: 0,
                },
                file.id,
                Entry::Real(FolderEntry::File(file)),
            ),
            FolderEntry::Folder(folder) => {
                let ino = self.insert(
                    parent_inode,
                    FileAttr {
                        ino: 0,
                        size: 0,
                        blocks: 0,
                        atime: SystemTime::UNIX_EPOCH,
                        mtime: folder
                            .date_modified
                            .and_then(DateTime::as_systemtime)
                            .unwrap_or(SystemTime::UNIX_EPOCH),
                        ctime: SystemTime::UNIX_EPOCH,
                        crtime: folder
                            .date_created
                            .and_then(DateTime::as_systemtime)
                            .unwrap_or(SystemTime::UNIX_EPOCH),
                        kind: FileType::Directory,
                        perm: 0o777,
                        nlink: 1,
                        uid: 0,
                        gid: 0,
                        rdev: 0,
                        blksize: 0,
                        flags: 0,
                    },
                    folder.id,
                    Entry::Empty,
                )?;

                for child in folder.children.iter().cloned() {
                    self.insert_entry(ino, child);
                }

                self.update_entry(ino, Entry::Real(FolderEntry::Folder(folder)));
                Some(ino)
            },
        }
    }

    fn root_node_id(&self) -> &NodeId {
        &self.node_ids[0]
    }

    fn reset(&mut self) {
        // Only retain /, Playlists, and lost+found
        let nodes_to_remove = self
            .tree
            .children_ids(self.root_node_id())
            .unwrap()
            .skip(2)
            .cloned()
            .collect::<Vec<_>>();
        for node_id in nodes_to_remove {
            self.tree
                .remove_node(node_id, RemoveBehavior::DropChildren)
                .unwrap();
        }

        self.next_inode.store(FUSE_ROOT_ID + 3, Ordering::Relaxed);
    }
}

pub struct MtpFuse {
    session: Arc<Mutex<MtpSession<DeviceHandle>>>,
    storage: Storage,
    is_android: bool,
    dirty: Dirty,

    fs: Option<FileSystem>,
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

        let root = INode {
            parent: FUSE_ROOT_ID,
            object_handle: ObjectHandle::NONE,
            attr: ROOT_ATTR,
            entry: Entry::Injected(Arc::new(InjectedEntry { name: "/".into() })),
        };

        let mut fs = Tree::new();

        let root_node_id = fs.insert(Node::new(root), InsertBehavior::AsRoot).unwrap();
        let node_ids = vec![root_node_id.clone()];

        let mut ret = MtpFuse {
            session,
            storage,
            is_android,
            dirty: Dirty::new(),
            fs: None,
            inner: FsInner {
                node_ids,
                next_inode: AtomicU64::new(FUSE_ROOT_ID + 1),
                tree: fs,
            },
        };

        ret.inner.insert(
            FUSE_ROOT_ID,
            PLAYLISTS_ATTR,
            ObjectHandle::NONE,
            Entry::Injected(Arc::new(InjectedEntry {
                name: "Playlists".into(),
            })),
        );
        ret.inner.insert(
            FUSE_ROOT_ID,
            LOST_AND_FOUND_ATTR,
            ObjectHandle::NONE,
            Entry::Injected(Arc::new(InjectedEntry {
                name: "lost+found".into(),
            })),
        );
        Ok(ret)
    }

    fn stat(&self, inode: u64, _file_handle: Option<u64>) -> Option<FileAttr> {
        if inode == 0 {
            return None;
        }

        if inode == FUSE_ROOT_ID {
            return Some(ROOT_ATTR);
        }

        if inode == PLAYLISTS_NODE_ID {
            return Some(PLAYLISTS_ATTR);
        }

        if inode == LOST_AND_FOUND_NODE_ID {
            return Some(LOST_AND_FOUND_ATTR);
        }

        self.inner.get(inode).map(|(inode, _)| inode.attr)
    }

    fn playlists(&mut self) -> impl Iterator<Item = (String, FileAttr)> {
        if self.dirty.playlists {
            self.dirty.playlists = false;
        }

        iter::empty()
    }

    fn lost_and_found(&mut self) -> impl Iterator<Item = (String, FileAttr)> {
        if self.dirty.lost_and_found {
            self.dirty.lost_and_found = false;
        }

        iter::empty()
    }

    async fn update_files_if_needed(&mut self) -> Result<(), Error> {
        if !self.dirty.fs {
            return Ok(()); // Already up to date
        }

        self.dirty.fs = false;
        self.inner.reset();

        let fs;
        let start = Instant::now();
        {
            let mut session = self.session.lock().await;

            let spinner = ProgressBar::new_spinner()
                .with_message("Loading all directories")
                .with_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());

            fs = FileSystem::load_with_callback(&mut *session, self.storage.id, |_| spinner.tick())
                .await?;
            spinner.finish_and_clear();
        }

        let mut root_inodes = Vec::new();
        for entry in fs.root.children.iter().cloned() {
            let Some(ino) = self.inner.insert(
                FUSE_ROOT_ID,
                FileAttr {
                    ino: 0,
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
                    blksize: 0,
                    flags: 0,
                },
                entry.handle(),
                Entry::Real(entry),
            ) else {
                continue;
            };

            root_inodes.push(ino);
        }

        for (entry, inode) in fs.root.children.iter().zip(root_inodes.into_iter()) {
            self.inner.insert_entry(inode, entry.clone());
        }

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
        Ok(())
    }

    fn children_of(&self, inode: u64) -> Option<impl Iterator<Item = &INode>> {
        let (_, node_id) = self.inner.get(inode)?;

        let children = self.inner.tree.children(node_id).unwrap();
        Some(children.map(Node::data))
    }

    fn lookup_(&self, parent: u64, name: &OsStr) -> Result<&INode, i32> {
        let Some(children) = self.children_of(parent) else {
            return Err(ENOENT);
        };

        for child in children {
            if name.to_str() == Some(child.entry.name()) {
                return Ok(child);
            }
        }

        Err(ENOENT)
    }

    fn file(&self, ino: u64) -> Result<&File, c_int> {
        let Some((inode, _)) = self.inner.get(ino) else {
            return Err(ENOENT);
        };

        let Entry::Real(entry) = &inode.entry else {
            return Err(EINVAL);
        };

        let FolderEntry::File(file) = entry else {
            return Err(EINVAL);
        };

        Ok(&**file)
    }

    async fn rename(
        &mut self,
        parent: u64,
        name: &OsStr,
        new_parent: u64,
        new_name: &OsStr,
    ) -> Result<(), i32> {
        let mut session = self.session.lock().await;

        let inode = self.lookup_(parent, name)?;

        let Entry::Real(entry) = &inode.entry else {
            return Err(EINVAL);
        };

        let Some(name_str) = new_name.to_str() else {
            return Err(EINVAL);
        };

        let Some((parent_inode, _)) = self.inner.get(new_parent) else {
            return Err(ENOENT);
        };

        let Entry::Real(parent) = &parent_inode.entry else {
            return Err(EINVAL);
        };

        let FolderEntry::Folder(parent) = &*parent else {
            return Err(EINVAL);
        };

        let ino = inode.attr.ino;
        let new_entry;
        match entry.rename(&mut *session, name_str).await {
            Ok(entry) => new_entry = entry,
            // Fallback to copy + delete
            Err(MtpError::UnsupportedOperation) => {
                let ret;
                match entry {
                    FolderEntry::Folder(_entry) => {
                        let folder = session
                            .mkdir(Some(parent), name_str)
                            .await
                            .map_err(|_| EIO)?;
                        ret = FolderEntry::Folder(Arc::new(folder));
                    },
                    FolderEntry::File(entry) => {
                        let mut f = entry.open(&mut *session).await.map_err(|_| EIO)?;

                        let mut data = Vec::new();
                        f.read_to_end(&mut data)
                            .map_err(|e| e.raw_os_error().unwrap_or(EIO))?;

                        let file = session
                            .create(Some(parent), name_str, entry.format, data)
                            .await
                            .map_err(|_| EIO)?;

                        ret = FolderEntry::File(Arc::new(file));
                    },
                }

                session
                    .delete_object(entry.handle(), Some(entry.format()))
                    .await
                    .map_err(|_| EIO)?;

                new_entry = ret;
            },
            Err(_) => return Err(EIO),
        }

        self.inner.update_entry(ino, Entry::Real(new_entry));

        Ok(())
    }
}

impl Filesystem for MtpFuse {
    fn init(&mut self, _req: &Request<'_>, _config: &mut KernelConfig) -> Result<(), c_int> {
        futures::executor::block_on(
            async move { self.update_files_if_needed().await.map_err(|_| EIO) },
        )
    }

    fn destroy(&mut self) {
        log::info!("Closing filesystem");
        self.inner = FsInner::default();
        let _ = self.fs.take();
    }

    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let result = futures::executor::block_on(async move {
            self.update_files_if_needed().await.map_err(|_| EIO)?;
            self.lookup_(parent, name)
        });
        match result {
            Ok(entry) => {
                reply.entry(&TTL, &entry.attr, 0);
            },
            Err(e) => {
                reply.error(e);
            },
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, fh: Option<u64>, reply: ReplyAttr) {
        match self.stat(ino, fh) {
            Some(attr) => reply.attr(&TTL, &attr),
            _ => reply.error(ENOENT),
        }
    }

    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        let Some((inode, _)) = self.inner.get(parent) else {
            reply.error(ENOENT);
            return;
        };

        let Some(name) = name.to_str() else {
            reply.error(EINVAL);
            return;
        };

        let parent_dir;
        if inode.attr.ino == FUSE_ROOT_ID {
            parent_dir = None;
        } else {
            let Entry::Real(entry) = &inode.entry else {
                reply.error(EINVAL);
                return;
            };

            let FolderEntry::Folder(dir) = entry else {
                reply.error(EINVAL);
                return;
            };

            parent_dir = Some(dir)
        }

        let result = futures::executor::block_on(async {
            let mut session = self.session.lock().await;
            session
                .mkdir(parent_dir.map(Arc::as_ref), name.to_string())
                .await
        });

        match result {
            Ok(folder) => {
                let Some(inode) = self
                    .inner
                    .insert_entry(parent, FolderEntry::Folder(Arc::new(folder)))
                else {
                    reply.error(ENOENT);
                    return;
                };

                let Some((inode, _)) = self.inner.get(inode) else {
                    reply.error(ENOENT);
                    return;
                };

                reply.entry(&TTL, &inode.attr, 0);
            },
            Err(e) => {
                reply.error(e.to_errno());
                return;
            },
        }
    }

    // TODO: Figure out why the lookups still resolve in rmdir and unlink even after destroy() is called
    fn unlink(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        match self.lookup_(parent, name) {
            Ok(entry) => {
                if entry.attr.kind != FileType::RegularFile {
                    reply.error(EINVAL);
                    return;
                }

                reply.ok();
            },
            Err(e) => reply.error(e),
        }
    }

    fn rmdir(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        match self.lookup_(parent, name) {
            Ok(entry) => {
                if entry.attr.ino == PLAYLISTS_NODE_ID {
                    reply.ok();
                    return;
                }

                if entry.attr.ino == LOST_AND_FOUND_NODE_ID {
                    reply.ok();
                    return;
                }

                reply.ok();
            },
            Err(e) => reply.error(e),
        }
    }

    fn symlink(
        &mut self,
        _req: &Request<'_>,
        _parent: u64,
        _link_name: &OsStr,
        _target: &Path,
        reply: ReplyEntry,
    ) {
        reply.error(ENOTSUP);
    }

    fn rename(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        _flags: u32,
        reply: ReplyEmpty,
    ) {
        let result = futures::executor::block_on(async move {
            self.update_files_if_needed().await.map_err(|_| EIO)?;
            self.rename(parent, name, newparent, newname).await
        });
        match result {
            Ok(()) => {
                reply.ok();
            },
            Err(e) => {
                reply.error(e);
            },
        }
    }

    fn link(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _newparent: u64,
        _newname: &OsStr,
        reply: ReplyEntry,
    ) {
        reply.error(ENOTSUP);
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, flags: i32, reply: ReplyOpen) {
        let file = match self.file(ino) {
            Ok(file) => file,
            Err(e) => {
                reply.error(e);
                return;
            },
        };

        futures::executor::block_on(async {
            let mut session = self.session.lock().await;
            match session.get_object_info(file.id).await {
                Ok(_fd) => reply.opened(0, flags as u32),
                Err(e) => {
                    reply.error(e.to_errno());
                },
            }
        });
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
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
            match session
                .get_partial_object(file.id, offset as u32, size)
                .await
            {
                Ok(response) => reply.data(&response.data.data),
                Err(e) => {
                    reply.error(e.to_errno());
                },
            }
        });
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
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
                    .send_partial_object(file.id, offset as u64, data.len() as u32, data)
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

            if let Err(e) = temp.seek(SeekFrom::Start(offset as u64)) {
                reply.error(e.raw_os_error().unwrap_or(EIO));
                return;
            }

            if let Err(e) = temp.write_all(data) {
                reply.error(e.raw_os_error().unwrap_or(EIO));
                return;
            }

            let mut object_data = Vec::new();
            if let Err(e) = temp.read_to_end(&mut object_data) {
                reply.error(e.raw_os_error().unwrap_or(EIO));
                return;
            }

            let Ok(object_info) = file.object_info() else {
                reply.error(EINVAL);
                return;
            };

            if let Err(e) = session.delete_object(file.id, None).await {
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
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _lock_owner: u64,
        reply: ReplyEmpty,
    ) {
        reply.ok()
    }

    fn opendir(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        let Some((inode, _)) = self.inner.get(ino) else {
            reply.error(ENOENT);
            return;
        };

        if inode.attr.kind != FileType::Directory {
            reply.error(ENOTDIR);
            return;
        }

        reply.opened(ino, 0);
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino == PLAYLISTS_NODE_ID {
            for (index, (name, attr)) in self.playlists().enumerate() {
                let _ = reply.add(attr.ino, (index + 2) as i64, attr.kind, name);
            }

            reply.ok();
            return;
        }

        if ino == LOST_AND_FOUND_NODE_ID {
            for (index, (name, attr)) in self.lost_and_found().enumerate() {
                let _ = reply.add(attr.ino, (index + 2) as i64, attr.kind, name);
            }

            reply.ok();
            return;
        }

        let Some(children) = self.children_of(ino) else {
            reply.error(ENOENT);
            return;
        };

        for (inode, offset) in children.skip(offset as usize).zip(offset..) {
            if reply.add(
                inode.attr.ino,
                offset + 1,
                inode.attr.kind,
                inode.entry.name(),
            ) {
                reply.ok();
                return;
            }
        }

        reply.ok();
    }

    fn statfs(&mut self, _req: &Request<'_>, _ino: u64, reply: ReplyStatfs) {
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
    fn to_errno(&self) -> c_int;
}

impl ToErrno for MtpError<Arc<UsbError>> {
    fn to_errno(&self) -> c_int {
        if let MtpError::Transport(e) = self {
            if let UsbError::Native(e) = &**e {
                return e.os_error().map(|e| e as c_int).unwrap_or(EIO);
            }
        }

        EIO
    }
}

impl ToErrno for Error {
    fn to_errno(&self) -> c_int {
        match self {
            Error::Io(e) => e.raw_os_error().unwrap_or(EIO),
            Error::Core(e) => e.to_errno(),
            Error::EventStream(_) | Error::Generic(_) => EIO,
        }
    }
}
