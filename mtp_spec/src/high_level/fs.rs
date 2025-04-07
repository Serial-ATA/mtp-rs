use crate::communication::SessionId;
use crate::device::storage::id::StorageId;
use crate::device::{Device, PtpIo};
use crate::error::MtpError;
use crate::object::info::ProtectionStatus;
use crate::object::types::properties::{ObjectFileName, ObjectFormat, ParentObject};
use crate::object::types::{DateTime, ObjectFormatCode, ObjectHandle, PtpString};

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use deku::DekuReader;
use deku::ctx::Endian;
use deku::no_std_io::Cursor;
use deku::prelude::Reader;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct File {
    pub storage_id: StorageId,
    pub id: ObjectHandle,
    pub name: String,
    pub format: ObjectFormatCode,
    pub protection_status: ProtectionStatus,
    pub date_created: Option<DateTime>,
    pub date_modified: Option<DateTime>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Folder {
    pub id: ObjectHandle,
    pub storage_id: StorageId,
    pub name: String,
    pub format: ObjectFormatCode,
    pub protection_status: ProtectionStatus,
    pub date_created: Option<DateTime>,
    pub date_modified: Option<DateTime>,
    pub children: Vec<FolderEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderEntry {
    File(File),
    Folder(Folder),
}

pub struct FileSystem {
    session_id: SessionId,
    storage_id: StorageId,
    pub contents: Vec<Folder>,
}

impl FileSystem {
    pub async fn load<D>(
        device: &mut D,
        session_id: SessionId,
        storage_id: StorageId,
    ) -> Result<Self, MtpError>
    where
        D: Device,
        <D as PtpIo>::Error: From<MtpError>,
    {
        Self::load_with_callback(device, session_id, storage_id, |_| {}).await
    }

    pub async fn load_with_callback<D, F>(
        device: &mut D,
        session_id: SessionId,
        storage_id: StorageId,
        callback: F,
    ) -> Result<Self, MtpError>
    where
        D: Device,
        <D as PtpIo>::Error: From<MtpError>,
        F: FnMut(ObjectHandle),
    {
        let mut ret = Self {
            session_id,
            storage_id,
            contents: Vec::new(),
        };

        ret.refresh_with_callback(device, callback).await?;
        Ok(ret)
    }

    pub async fn refresh<D>(&mut self, device: &mut D) -> Result<(), MtpError>
    where
        D: Device,
        <D as PtpIo>::Error: From<MtpError>,
    {
        self.refresh_with_callback(device, |_| {}).await
    }

    pub async fn refresh_with_callback<D>(
        &mut self,
        device: &mut D,
        mut callback: impl FnMut(ObjectHandle),
    ) -> Result<(), MtpError>
    where
        D: Device,
        <D as PtpIo>::Error: From<MtpError>,
    {
        let objects = match device
            .get_object_handles(
                self.session_id,
                self.storage_id,
                Some(ObjectFormatCode::Association),
                None,
            )
            .await
            .unwrap()
        {
            Ok(objects) => objects.data.data,
            Err(e) => {
                todo!()
            },
        };

        let mut entries = Vec::with_capacity(objects.len());
        for object in objects.iter().copied() {
            callback(object);

            let parent_response = device
                .get_object_prop_value::<ParentObject>(self.session_id, object)
                .await
                .unwrap();

            let parent = match parent_response {
                Ok(parent) => {
                    if parent.data.data == [0; 4] {
                        None
                    } else {
                        Some(ObjectHandle::from(u32::from_le_bytes(
                            parent.data.data.try_into().unwrap(),
                        )))
                    }
                },
                Err(e) => {
                    log::warn!("Failed to get parent object, skipping: {e}");
                    continue;
                },
            };

            let name_response = device
                .get_object_prop_value::<ObjectFileName>(self.session_id, object)
                .await
                .unwrap();

            let name;
            match name_response {
                Ok(name_) => {
                    name = PtpString::from_reader_with_ctx(
                        &mut Reader::new(Cursor::new(name_.data.data)),
                        Endian::Little,
                    )?
                    .to_string();
                },
                Err(e) => {
                    log::warn!("Failed to get object name, skipping: {e}");
                    continue;
                },
            }

            let format_response = device
                .get_object_prop_value::<ObjectFormat>(self.session_id, object)
                .await
                .unwrap();

            let format;
            match format_response {
                Ok(format_) => {
                    format = ObjectFormatCode::from_reader_with_ctx(
                        &mut Reader::new(Cursor::new(format_.data.data)),
                        Endian::Little,
                    )?;
                },
                Err(e) => {
                    log::warn!("Failed to get object format, skipping: {e}");
                    continue;
                },
            }

            match parent {
                Some(parent) => match format {
                    ObjectFormatCode::Association => {
                        entries.push((
                            parent,
                            FolderEntry::Folder(Folder {
                                storage_id: self.storage_id,
                                id: object,
                                name,
                                format,
                                protection_status: ProtectionStatus::ReadOnly,
                                date_created: None,
                                date_modified: None,
                                children: Vec::new(),
                            }),
                        ));
                    },
                    _ => {
                        entries.push((
                            parent,
                            FolderEntry::File(File {
                                storage_id: self.storage_id,
                                id: object,
                                name,
                                format,
                                protection_status: ProtectionStatus::ReadOnly,
                                date_created: None,
                                date_modified: None,
                            }),
                        ));
                    },
                },
                None => {
                    self.contents.push(Folder {
                        storage_id: self.storage_id,
                        id: object,
                        name,
                        format,
                        protection_status: ProtectionStatus::ReadOnly,
                        date_created: None,
                        date_modified: None,
                        children: Vec::new(),
                    });
                },
            }
        }

        let root_ids = self.contents.iter().map(|dir| dir.id).collect::<Vec<_>>();
        for (index, root) in root_ids.iter().enumerate() {
            let children = collect_children(*root, &entries);
            self.contents[index].children = children;
        }

        Ok(())
    }
}

fn collect_children(
    root: ObjectHandle,
    all_entries: &[(ObjectHandle, FolderEntry)],
) -> Vec<FolderEntry> {
    let mut entries = Vec::new();
    for (parent, entry) in all_entries.iter() {
        if *parent != root {
            continue;
        }

        entries.push(entry.clone());
    }

    entries
}
