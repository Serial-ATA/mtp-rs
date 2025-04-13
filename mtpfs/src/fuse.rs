use std::ffi::OsStr;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::iter;
use std::mem::ManuallyDrop;
use std::os::fd::{FromRawFd, IntoRawFd};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use fuser::{
    FUSE_ROOT_ID, FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, Request,
};
use id_tree::{InsertBehavior, Node, NodeId, RemoveBehavior, Tree};
use indicatif::{ProgressBar, ProgressStyle};
use libc::{EINVAL, EIO, ENOENT, ENOTDIR};
use mtp::communication::SessionId;
use mtp::error::Error;
use mtp::high_level::DateTimeExt;
use mtp::high_level::fs::{DeviceFsExt, FileSystem, FolderEntry};
use mtp::high_level::storages::Storage;
use mtp::object::types::{DateTime, ObjectHandle};
use mtp::usb::DeviceHandle;
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
    Real(Arc<FolderEntry>),
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

    fn insert_entry(&mut self, parent_inode: u64, entry: Arc<FolderEntry>) -> Option<u64> {
        match &*entry {
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
                Entry::Real(entry),
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

                self.update_entry(ino, Entry::Real(entry));
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
    device: Arc<Mutex<DeviceHandle>>,
    session_id: SessionId,
    storage: Storage,
    dirty: Dirty,

    fs: Option<FileSystem>,
    inner: FsInner,
}

impl MtpFuse {
    pub fn new(device: Arc<Mutex<DeviceHandle>>, session_id: SessionId, storage: Storage) -> Self {
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
            device,
            session_id,
            storage,
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
        ret
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
        {
            let mut device = self.device.lock().await;

            let spinner = ProgressBar::new_spinner()
                .with_message("Loading all directories")
                .with_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());

            fs = FileSystem::load_with_callback(
                &mut *device,
                self.session_id,
                self.storage.id,
                |_| spinner.tick(),
            )
            .await?;
            spinner.finish_and_clear();
        }

        let mut root_inodes = Vec::new();
        for entry in fs.contents.iter().cloned() {
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
                entry.id,
                Entry::Real(Arc::new(FolderEntry::Folder(entry))),
            ) else {
                continue;
            };

            root_inodes.push(ino);
        }

        for (root, inode) in fs.contents.iter().zip(root_inodes.into_iter()) {
            for child in root.children.iter().cloned() {
                self.inner.insert_entry(inode, child);
            }
        }

        self.fs = Some(fs);
        Ok(())
    }

    fn children_of(&self, inode: u64) -> Option<impl Iterator<Item = &INode>> {
        let (_, node_id) = self.inner.get(inode)?;

        let children = self.inner.tree.children(node_id).unwrap();
        Some(children.map(Node::data))
    }

    fn _lookup(&self, parent: u64, name: &OsStr) -> Result<&INode, i32> {
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

    async fn rename(
        &mut self,
        parent: u64,
        name: &OsStr,
        new_parent: u64,
        new_name: &OsStr,
    ) -> Result<(), i32> {
        let mut device = self.device.lock().await;

        let inode = self._lookup(parent, name)?;

        let Entry::Real(entry) = &inode.entry else {
            return Err(EINVAL);
        };

        let Some(name_str) = new_name.to_str() else {
            return Err(EINVAL);
        };

        if new_parent != parent {
            todo!();
        }

        let ino = inode.attr.ino;
        let new_entry;
        {
            new_entry = entry
                .rename(&mut *device, self.session_id, name_str)
                .await
                .map_err(|_| EIO)?;
        }

        self.inner
            .update_entry(ino, Entry::Real(Arc::new(new_entry)));

        Ok(())
    }
}

impl Filesystem for MtpFuse {
    fn destroy(&mut self) {
        log::info!("Closing filesystem");
    }

    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let result = futures::executor::block_on(async move {
            self.update_files_if_needed().await.map_err(|_| EIO)?;
            self._lookup(parent, name)
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

            let FolderEntry::Folder(dir) = &**entry else {
                reply.error(EINVAL);
                return;
            };

            parent_dir = Some(dir)
        }

        let result = futures::executor::block_on(async {
            let mut device = self.device.lock().await;
            device.mkdir(self.session_id, parent_dir, name).await
        });

        match result {
            Ok(folder) => {
                let Some(inode) = self
                    .inner
                    .insert_entry(parent, Arc::new(FolderEntry::Folder(folder)))
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
            Err(_) => {
                reply.error(EIO);
                return;
            },
        }
    }

    fn unlink(&mut self, _req: &Request<'_>, _parent: u64, _name: &OsStr, _reply: ReplyEmpty) {
        todo!()
    }

    fn rmdir(&mut self, _req: &Request<'_>, _parent: u64, _name: &OsStr, _reply: ReplyEmpty) {
        todo!()
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
        _reply: ReplyEntry,
    ) {
        todo!()
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, flags: i32, reply: ReplyOpen) {
        let Some((inode, _)) = self.inner.get(ino) else {
            reply.error(ENOENT);
            return;
        };

        let Entry::Real(entry) = &inode.entry else {
            reply.error(EINVAL);
            return;
        };

        let FolderEntry::File(file) = &**entry else {
            reply.error(EINVAL);
            return;
        };

        futures::executor::block_on(async {
            let mut device = self.device.lock().await;
            match file.open(&mut *device, self.session_id).await {
                Ok(fd) => reply.opened(fd.into_raw_fd() as u64, flags as u32),
                Err(e) => {
                    let Error::Io(err) = e else {
                        reply.error(EIO);
                        return;
                    };

                    if let Some(errno) = err.raw_os_error() {
                        reply.error(errno);
                        return;
                    }

                    reply.error(EIO);
                },
            }
        });
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let mut file = ManuallyDrop::new(unsafe { File::from_raw_fd(fh as _) });
        if file.seek(SeekFrom::Start(offset as _)).is_err() {
            reply.error(EIO);
            return;
        }

        let mut buf = vec![0; size as usize];
        if let Err(e) = file.read_exact(&mut buf) {
            match e.raw_os_error() {
                Some(errno) => reply.error(errno),
                None => reply.error(EIO),
            }

            return;
        }

        reply.data(&buf);
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        _reply: ReplyWrite,
    ) {
        todo!()
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

    fn release(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        unsafe { File::from_raw_fd(fh as _) };
        reply.ok();
    }

    fn fsync(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        _reply: ReplyEmpty,
    ) {
        todo!()
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

    fn releasedir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        reply: ReplyEmpty,
    ) {
        reply.ok()
    }

    fn fsyncdir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        reply: ReplyEmpty,
    ) {
        reply.ok()
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
