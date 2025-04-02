use fuser::{
    FUSE_ROOT_ID, FileAttr, FileType, Filesystem, Reply, ReplyAttr, ReplyData, ReplyDirectory,
    ReplyEmpty, ReplyEntry, ReplyLock, ReplyOpen, ReplyStatfs, ReplyWrite, ReplyXattr, Request,
    TimeOrNow,
};
use id_tree::{InsertBehavior, Node, NodeId, Tree, TreeBuilder};
use libc::{ENOENT, ENOTDIR};
use mtp::device::storage::info::StorageInfo;
use mtp::usb::DeviceHandle;
use std::ffi::OsStr;
use std::iter;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

const TTL: Duration = Duration::from_secs(0);
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
    nlink: 2,
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
    nlink: 2,
    uid: 0,
    gid: 0,
    rdev: 0,
    blksize: BLOCK_SIZE,
    flags: 0,
};

#[derive(Copy, Clone, Debug)]
struct INode {
    parent: u64,
    file_handle: u64,
    attr: FileAttr,
}

#[derive(Default)]
struct Dirty {
    playlists: bool,
    lost_and_found: bool,
}

pub struct MtpFuse {
    device: Arc<Mutex<DeviceHandle>>,
    storage: StorageInfo,
    dirty: Dirty,
    node_ids: Vec<NodeId>,
    fs: Tree<INode>,
}

impl MtpFuse {
    pub fn new(device: Arc<Mutex<DeviceHandle>>, storage: StorageInfo) -> MtpFuse {
        let root = INode {
            parent: 0,
            file_handle: 0,
            attr: ROOT_ATTR,
        };

        let mut fs = Tree::new();

        let root_node_id = fs.insert(Node::new(root), InsertBehavior::AsRoot).unwrap();
        let node_ids = vec![root_node_id];

        let mut ret = MtpFuse {
            device,
            storage,
            dirty: Dirty::default(),
            node_ids,
            fs,
        };

        ret.insert(FUSE_ROOT_ID, PLAYLISTS_ATTR);
        ret.insert(FUSE_ROOT_ID, LOST_AND_FOUND_ATTR);
        ret
    }

    fn next_inode(&self) -> u64 {
        self.node_ids.len() as u64
    }

    fn get_node_id(&self, inode: u64) -> Option<&NodeId> {
        self.node_ids.get((inode - FUSE_ROOT_ID) as usize)
    }

    fn get(&self, inode: u64) -> Option<INode> {
        let inode_id = self.get_node_id(inode)?;
        self.fs.get(inode_id).ok().map(|node| *node.data())
    }

    fn insert(&mut self, parent_inode: u64, attr: FileAttr) {
        let Some(parent) = self.get_node_id(parent_inode).cloned() else {
            return;
        };

        let new_inode = INode {
            parent: parent_inode,
            file_handle: 0,
            attr,
        };

        let new_inode_id = self
            .fs
            .insert(Node::new(new_inode), InsertBehavior::UnderNode(&parent))
            .unwrap();

        self.node_ids.push(new_inode_id);
    }

    fn stat(&self, inode: u64, file_handle: Option<u64>) -> Option<FileAttr> {
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

        let mut attr = FileAttr {
            ino: inode,
            size: 0,
            blocks: 0,
            atime: SystemTime::UNIX_EPOCH,
            mtime: SystemTime::UNIX_EPOCH,
            ctime: SystemTime::UNIX_EPOCH,
            crtime: SystemTime::UNIX_EPOCH,
            kind: FileType::NamedPipe,
            perm: 0,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 0,
            flags: 0,
        };

        todo!()
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
}

impl Filesystem for MtpFuse {
    fn destroy(&mut self) {
        log::info!("Closing filesystem");
    }

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, mut reply: ReplyEntry) {
        dbg!(name.to_str(), parent, self.get(parent));
        let s = name.to_str();
        if parent == FUSE_ROOT_ID {
            if s == Some("Playlists") {
                reply.entry(&TTL, &PLAYLISTS_ATTR, 0);
                return;
            }

            if s == Some("lost+found") {
                reply.entry(&TTL, &LOST_AND_FOUND_ATTR, 0);
                return;
            }
        }

        let Some(parent) = self.get(parent) else {
            reply.error(ENOENT);
            return;
        };

        if s == Some(".") {
            reply.entry(&TTL, &parent.attr, 0);
            return;
        }

        if s == Some("..") {
            let Some(up) = self.get(parent.parent) else {
                reply.error(ENOENT);
                return;
            };

            reply.entry(&TTL, &up.attr, 0);
            return;
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
        parent: u64,
        name: &OsStr,
        mode: u32,
        umask: u32,
        reply: ReplyEntry,
    ) {
        todo!()
    }

    fn unlink(&mut self, _req: &Request, _parent: u64, _name: &OsStr, reply: ReplyEmpty) {
        todo!()
    }

    fn rmdir(&mut self, _req: &Request, _parent: u64, _name: &OsStr, reply: ReplyEmpty) {
        todo!()
    }

    fn rename(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        newparent: u64,
        newname: &OsStr,
        flags: u32,
        reply: ReplyEmpty,
    ) {
        todo!()
    }

    fn link(
        &mut self,
        _req: &Request,
        _ino: u64,
        _newparent: u64,
        _newname: &OsStr,
        reply: ReplyEntry,
    ) {
        todo!()
    }

    fn open(&mut self, req: &Request, _ino: u64, flags: i32, reply: ReplyOpen) {
        todo!()
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        todo!()
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        todo!()
    }

    fn flush(&mut self, _req: &Request, _ino: u64, _fh: u64, _lock_owner: u64, reply: ReplyEmpty) {
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
        reply: ReplyEmpty,
    ) {
        todo!()
    }

    fn fsync(&mut self, _req: &Request, _ino: u64, _fh: u64, _datasync: bool, reply: ReplyEmpty) {
        todo!()
    }

    fn opendir(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        let Some(inode) = self.get(ino) else {
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
        fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino == 0 {
            reply.error(ENOENT);
            return;
        }

        let Some(inode) = self.get(ino) else {
            reply.error(ENOENT);
            return;
        };

        let _ = reply.add(ino, 1, FileType::Directory, ".");
        let _ = reply.add(inode.parent, 2, FileType::Directory, "..");

        if ino == FUSE_ROOT_ID {
            let _ = reply.add(PLAYLISTS_NODE_ID, 3, FileType::Directory, "Playlists");
            let _ = reply.add(LOST_AND_FOUND_NODE_ID, 4, FileType::Directory, "lost+found");

            reply.ok();
            return;
        }

        if ino == PLAYLISTS_NODE_ID {
            for (index, (name, attr)) in self.playlists().enumerate() {
                let _ = reply.add(1, (index + 3) as i64, FileType::RegularFile, name);
            }

            reply.ok();
            return;
        }

        if ino == LOST_AND_FOUND_NODE_ID {
            for (index, (name, attr)) in self.lost_and_found().enumerate() {
                let _ = reply.add(1, (index + 3) as i64, FileType::RegularFile, name);
            }

            reply.ok();
            return;
        }

        dbg!(ino, fh, offset);
        todo!()
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
        reply: ReplyEmpty,
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
