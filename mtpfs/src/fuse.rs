use fuser::{
    FileAttr, Filesystem, ReplyAttr, ReplyBmap, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, ReplyLock, ReplyOpen, ReplyStatfs, ReplyWrite, ReplyXattr, Request, TimeOrNow,
};
use id_tree::{Node, Tree, TreeBuilder};
use mtp::device::storage::info::StorageInfo;
use mtp::object::info::ObjectInfo;
use mtp::usb::DeviceHandle;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, SystemTime};

pub struct MtpFuse {
    device: DeviceHandle,
    storage: StorageInfo,
}

impl MtpFuse {
    pub fn new(device: DeviceHandle, storage: StorageInfo) -> MtpFuse {
        let root = Node::new(1);
        let mut fs = TreeBuilder::new().with_root(root).build();

        MtpFuse { device, storage }
    }
}

impl Filesystem for MtpFuse {
    fn destroy(&mut self) {
        log::info!("Closing filesystem");
    }

    fn lookup(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        todo!()
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, fh: Option<u64>, reply: ReplyAttr) {
        todo!()
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

    fn readdir(
        &mut self,
        _req: &Request,
        _ino: u64,
        _fh: u64,
        _offset: i64,
        reply: ReplyDirectory,
    ) {
        todo!()
    }

    fn releasedir(&mut self, _req: &Request, _ino: u64, _fh: u64, _flags: i32, reply: ReplyEmpty) {
        todo!()
    }

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
        const BLOCK_SIZE: u32 = 1024;
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
