use fuser::{
    FUSE_ROOT_ID, FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, Request,
};
use id_tree::{InsertBehavior, Node, NodeId, RemoveBehavior, Tree};
use indicatif::{ProgressBar, ProgressStyle};
use libc::{ENOENT, ENOTDIR};
use mtp::communication::SessionId;
use mtp::error::Error;
use mtp::high_level::fs::{FileSystem, FolderEntry};
use mtp::high_level::storages::Storage;
use mtp::object::types::ObjectHandle;
use mtp::usb::DeviceHandle;
use std::ffi::OsStr;
use std::fmt::{Debug, Formatter};
use std::iter;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime};
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

#[derive(Clone)]
struct INode {
    parent: u64,
    object_handle: ObjectHandle,
    attr: FileAttr,
    name: Arc<str>,
}

impl Debug for INode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("INode")
            .field("inode", &self.attr.ino)
            .field("kind", &self.attr.kind)
            .field("parent", &self.parent)
            .field("object_handle", &self.object_handle)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

#[derive(Default)]
struct Dirty {
    playlists: bool,
    lost_and_found: bool,
}

struct FsProps {
    node_ids: Vec<NodeId>,
    files_changed: AtomicBool,
    next_inode: AtomicU64,
}

pub struct MtpFuse {
    device: Arc<Mutex<DeviceHandle>>,
    session_id: SessionId,
    storage: Storage,
    dirty: Dirty,

    fs: Tree<INode>,
    props: FsProps,
}

impl MtpFuse {
    pub fn new(
        device: Arc<Mutex<DeviceHandle>>,
        session_id: SessionId,
        storage: Storage,
    ) -> MtpFuse {
        let root = INode {
            parent: FUSE_ROOT_ID,
            object_handle: ObjectHandle::NONE,
            attr: ROOT_ATTR,
            name: String::from("/").into(),
        };

        let mut fs = Tree::new();

        let root_node_id = fs.insert(Node::new(root), InsertBehavior::AsRoot).unwrap();
        let node_ids = vec![root_node_id.clone()];

        let mut ret = MtpFuse {
            device,
            session_id,
            storage,
            dirty: Dirty::default(),
            fs,
            props: FsProps {
                node_ids,
                files_changed: AtomicBool::new(true),
                next_inode: AtomicU64::new(FUSE_ROOT_ID + 1),
            },
        };

        ret.insert(
            FUSE_ROOT_ID,
            PLAYLISTS_ATTR,
            ObjectHandle::NONE,
            String::from("Playlists"),
        );
        ret.insert(
            FUSE_ROOT_ID,
            LOST_AND_FOUND_ATTR,
            ObjectHandle::NONE,
            String::from("lost+found"),
        );
        ret
    }

    fn get_node_id(&self, inode: u64) -> Option<&NodeId> {
        self.props.node_ids.get((inode - FUSE_ROOT_ID) as usize)
    }

    fn get(&self, inode: u64) -> Option<(&INode, &NodeId)> {
        let node_id = self.get_node_id(inode)?;
        self.fs.get(node_id).ok().map(|node| (node.data(), node_id))
    }

    fn insert(
        &mut self,
        parent_inode: u64,
        mut attr: FileAttr,
        object_handle: ObjectHandle,
        name: String,
    ) -> Option<u64> {
        let parent = self.get_node_id(parent_inode).cloned()?;

        let inode = self.props.next_inode.fetch_add(1, Ordering::Relaxed);
        attr.ino = inode;

        let new_inode = INode {
            parent: parent_inode,
            object_handle,
            attr,
            name: name.into(),
        };

        let new_inode_id = self
            .fs
            .insert(Node::new(new_inode), InsertBehavior::UnderNode(&parent))
            .unwrap();

        self.props.node_ids.push(new_inode_id);
        Some(inode)
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

        self.get(inode).map(|(inode, _)| inode.attr.clone())
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

    fn root_node_id(&self) -> &NodeId {
        &self.props.node_ids[0]
    }

    fn reset_fs(&mut self) {
        // Only retain /, Playlists, and lost+found
        let nodes_to_remove = self
            .fs
            .children_ids(self.root_node_id())
            .unwrap()
            .skip(2)
            .cloned()
            .collect::<Vec<_>>();
        for node_id in nodes_to_remove {
            self.fs
                .remove_node(node_id, RemoveBehavior::DropChildren)
                .unwrap();
        }

        self.props
            .next_inode
            .store(FUSE_ROOT_ID + 3, Ordering::Relaxed);
    }

    async fn update_files_if_needed(&mut self) -> Result<(), Error> {
        if self
            .props
            .files_changed
            .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            return Ok(()); // Already up to date
        }

        self.reset_fs();

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
        for entry in &fs.contents {
            let Some(ino) = self.insert(
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
                entry.name.clone(),
            ) else {
                continue;
            };

            root_inodes.push(ino);
        }

        for (root, inode) in fs.contents.iter().zip(root_inodes.into_iter()) {
            for child in &root.children {
                self.insert_entry(inode, child);
            }
        }

        Ok(())
    }

    fn insert_entry(&mut self, parent_inode: u64, entry: &FolderEntry) {
        match entry {
            FolderEntry::File(file) => {
                self.insert(
                    parent_inode,
                    FileAttr {
                        ino: 0,
                        size: 0,
                        blocks: 0,
                        atime: SystemTime::UNIX_EPOCH,
                        mtime: SystemTime::UNIX_EPOCH,
                        ctime: SystemTime::UNIX_EPOCH,
                        crtime: SystemTime::UNIX_EPOCH,
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
                    file.name.clone(),
                );
            },
            FolderEntry::Folder(folder) => {
                let Some(ino) = self.insert(
                    parent_inode,
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
                    folder.id,
                    folder.name.clone(),
                ) else {
                    return;
                };

                for child in &folder.children {
                    self.insert_entry(ino, child);
                }
            },
        }
    }

    async fn children_of(
        &mut self,
        inode: u64,
    ) -> Result<Option<impl Iterator<Item = (&FileAttr, Arc<str>)>>, Error> {
        self.update_files_if_needed().await?;

        let Some((inode_entry, node_id)) = self.get(inode) else {
            return Ok(None);
        };

        let parent_inode;
        if inode == FUSE_ROOT_ID {
            parent_inode = FUSE_ROOT_ID;
        } else {
            parent_inode = inode_entry.parent;
        }

        let children = self
            .fs
            .children(node_id)
            .unwrap()
            .map(|node| (&node.data().attr, node.data().name.clone()));

        let (parent, _) = self.get(parent_inode).expect("parent should exist");
        let pseudo_entries = [(&inode_entry.attr, ".".into()), (&parent.attr, "..".into())];

        Ok(Some(pseudo_entries.into_iter().chain(children)))
    }
}

impl Filesystem for MtpFuse {
    fn destroy(&mut self) {
        log::info!("Closing filesystem");
    }

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let result = futures::executor::block_on(async move { self.children_of(parent).await });

        let children;
        match result {
            Ok(Some(c)) => children = c,
            Ok(None) => {
                reply.error(ENOENT);
                return;
            },
            Err(e) => todo!(),
        }

        for (attr, child_name) in children {
            if name.to_str() == Some(&*child_name) {
                reply.entry(&TTL, attr, 0);
                return;
            }
        }

        reply.error(ENOENT);
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
        _parent: u64,
        _name: &OsStr,
        _mode: u32,
        _umask: u32,
        _reply: ReplyEntry,
    ) {
        todo!()
    }

    fn unlink(&mut self, _req: &Request, _parent: u64, _name: &OsStr, _reply: ReplyEmpty) {
        todo!()
    }

    fn rmdir(&mut self, _req: &Request, _parent: u64, _name: &OsStr, _reply: ReplyEmpty) {
        todo!()
    }

    fn rename(
        &mut self,
        _req: &Request<'_>,
        _parent: u64,
        _name: &OsStr,
        _newparent: u64,
        _newname: &OsStr,
        _flags: u32,
        _reply: ReplyEmpty,
    ) {
        todo!()
    }

    fn link(
        &mut self,
        _req: &Request,
        _ino: u64,
        _newparent: u64,
        _newname: &OsStr,
        _reply: ReplyEntry,
    ) {
        todo!()
    }

    fn open(&mut self, _req: &Request, _ino: u64, _flags: i32, _reply: ReplyOpen) {
        todo!()
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        _size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        _reply: ReplyData,
    ) {
        todo!()
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

    fn flush(&mut self, _req: &Request, _ino: u64, _fh: u64, _lock_owner: u64, _reply: ReplyEmpty) {
        todo!()
    }

    fn release(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        _reply: ReplyEmpty,
    ) {
        todo!()
    }

    fn fsync(&mut self, _req: &Request, _ino: u64, _fh: u64, _datasync: bool, _reply: ReplyEmpty) {
        todo!()
    }

    fn opendir(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        let Some((inode, _)) = self.get(ino) else {
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
        _req: &Request,
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

        let result = futures::executor::block_on(async move { self.children_of(ino).await });

        let children;
        match result {
            Ok(Some(c)) => children = c,
            Ok(None) => {
                reply.error(ENOENT);
                return;
            },
            Err(e) => todo!(),
        }

        for ((attr, name), offset) in children.skip(offset as usize).zip(offset..) {
            if reply.add(attr.ino, offset + 1, attr.kind, &*name) {
                reply.ok();
                return;
            }
        }

        reply.ok();
    }

    // fn releasedir(&mut self, _req: &Request, _ino: u64, _fh: u64, _flags: i32, reply: ReplyEmpty) {
    //     todo!()
    // }

    fn fsyncdir(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _datasync: bool,
        _reply: ReplyEmpty,
    ) {
        todo!()
    }

    fn statfs(&mut self, _req: &Request, _ino: u64, reply: ReplyStatfs) {
        let blocks = self.storage.max_capacity / BLOCK_SIZE as u64;
        let blocks_free = self.storage.free_space / BLOCK_SIZE as u64;
        let blocks_available = blocks_free;
        let files_free = self.storage.free_space_in_objects / BLOCK_SIZE;
        reply.statfs(
            blocks,
            blocks_free,
            blocks_available,
            0,
            files_free as u64,
            BLOCK_SIZE,
            0,
            0,
        );
    }
}
