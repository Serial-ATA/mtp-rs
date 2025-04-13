mod device_ext;
pub use device_ext::*;

use crate::communication::SessionId;
use crate::device::storage::id::StorageId;
use crate::device::{Device, PtpIo};
use crate::error::{Error, MtpError};
use crate::object::info::ProtectionStatus;
use crate::object::types::properties::{ObjectFileName, ObjectFormat, ObjectSize, ParentObject};
use crate::object::types::{DateTime, ObjectFormatCode, ObjectHandle, PtpString};

use std::io::Write;
use std::sync::Arc;

use deku::DekuReader;
use deku::ctx::Endian;
use deku::no_std_io::Cursor;
use deku::prelude::Reader;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct File {
    pub storage_id: StorageId,
    pub id: ObjectHandle,
    pub name: String,
    pub size: u64,
    pub format: ObjectFormatCode,
    pub protection_status: ProtectionStatus,
    pub date_created: Option<DateTime>,
    pub date_modified: Option<DateTime>,
}

impl File {
    pub async fn open<D>(
        &self,
        device: &mut D,
        session_id: SessionId,
    ) -> Result<std::fs::File, <D as PtpIo>::Error>
    where
        D: Device,
        <D as PtpIo>::Error: From<Error>,
        <D as PtpIo>::Error: From<MtpError>,
    {
        let object = device
            .get_object(session_id, self.id)
            .await?
            .map_err(Into::<MtpError>::into)?;
        let file_data = object.data.data;

        let mut tmp = tempfile::tempfile().map_err(Into::<Error>::into)?;
        tmp.write_all(&file_data).map_err(Into::<Error>::into)?;

        Ok(tmp)
    }

    pub async fn rename<D, N>(
        &self,
        device: &mut D,
        session_id: SessionId,
        name: N,
    ) -> Result<Self, <D as PtpIo>::Error>
    where
        D: Device,
        N: Into<String>,
        <D as PtpIo>::Error: From<MtpError>,
    {
        dbg!(
            device
                .get_object_props_supported(session_id, self.format)
                .await
                .unwrap()
        );

        if !dbg!(
            device
                .property_can_be_modified::<ObjectFileName>(session_id, self.format)
                .await
        )? {
            return Err(MtpError::Generic("Property cannot be modified".into()).into());
        }

        let name = PtpString::try_from(name.into())?;
        let name_str = name.to_string();
        let _ = device
            .set_object_prop_value::<ObjectFileName>(session_id, self.id, name)
            .await?
            .map_err(Into::<MtpError>::into)?;

        Ok(Self {
            storage_id: self.storage_id,
            id: self.id,
            name: name_str,
            size: self.size,
            format: self.format,
            protection_status: self.protection_status,
            date_created: self.date_created,
            date_modified: self.date_modified,
        })
    }
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
    pub children: Vec<Arc<FolderEntry>>,
}

impl Folder {
    pub async fn rename<D, N>(
        &self,
        device: &mut D,
        session_id: SessionId,
        name: N,
    ) -> Result<Self, <D as PtpIo>::Error>
    where
        D: Device,
        N: Into<String>,
        <D as PtpIo>::Error: From<MtpError>,
    {
        let name = PtpString::try_from(name.into())?;
        let name_str = name.to_string();
        let _ = device
            .set_object_prop_value::<ObjectFileName>(session_id, self.id, name)
            .await?
            .map_err(Into::<MtpError>::into)?;

        Ok(Self {
            storage_id: self.storage_id,
            id: self.id,
            name: name_str,
            format: self.format,
            protection_status: self.protection_status,
            date_created: self.date_created,
            date_modified: self.date_modified,
            children: self.children.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderEntry {
    File(File),
    Folder(Folder),
}

impl FolderEntry {
    pub fn name(&self) -> &str {
        match self {
            FolderEntry::File(f) => &f.name,
            FolderEntry::Folder(f) => &f.name,
        }
    }

    pub async fn rename<D, N>(
        &self,
        device: &mut D,
        session_id: SessionId,
        name: N,
    ) -> Result<Self, <D as PtpIo>::Error>
    where
        D: Device,
        N: Into<String>,
        <D as PtpIo>::Error: From<MtpError>,
    {
        match self {
            FolderEntry::File(f) => {
                Ok(FolderEntry::File(f.rename(device, session_id, name).await?))
            },
            FolderEntry::Folder(f) => Ok(FolderEntry::Folder(
                f.rename(device, session_id, name).await?,
            )),
        }
    }
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
    ) -> Result<Self, <D as PtpIo>::Error>
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
    ) -> Result<Self, <D as PtpIo>::Error>
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

    pub async fn refresh<D>(&mut self, device: &mut D) -> Result<(), <D as PtpIo>::Error>
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
    ) -> Result<(), <D as PtpIo>::Error>
    where
        D: Device,
        <D as PtpIo>::Error: From<MtpError>,
    {
        let objects_response = device
            .get_object_handles(self.session_id, self.storage_id, None, None)
            .await?
            .map_err(Into::<MtpError>::into)?;
        let objects = objects_response.data.data;

        let mut entries = Vec::with_capacity(objects.len());
        for object in objects.iter().copied() {
            callback(object);

            let parent_response = device
                .get_object_prop_value::<ParentObject>(self.session_id, object)
                .await?;

            let parent = match parent_response {
                Ok(parent) => {
                    if parent.data.data == [0; 4] {
                        None
                    } else {
                        // TODO
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
                .await?;

            let name;
            match name_response {
                Ok(name_) => {
                    // TODO
                    name = PtpString::from_reader_with_ctx(
                        &mut Reader::new(Cursor::new(name_.data.data)),
                        Endian::Little,
                    )
                    .map_err(Into::<MtpError>::into)?
                    .to_string();
                },
                Err(e) => {
                    log::warn!("Failed to get object name, skipping: {e}");
                    continue;
                },
            }

            let format_response = device
                .get_object_prop_value::<ObjectFormat>(self.session_id, object)
                .await?;

            let format;
            match format_response {
                Ok(format_) => {
                    format = ObjectFormatCode::from_reader_with_ctx(
                        &mut Reader::new(Cursor::new(format_.data.data)),
                        Endian::Little,
                    )
                    .map_err(Into::<MtpError>::into)?;
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
                        let size_response = device
                            .get_object_prop_value::<ObjectSize>(self.session_id, object)
                            .await?;

                        let size;
                        match size_response {
                            Ok(size_) => {
                                // TODO
                                size = u64::from_le_bytes(size_.data.data.try_into().unwrap());
                            },
                            Err(e) => {
                                log::warn!("Failed to get object size, skipping: {e}");
                                continue;
                            },
                        }

                        entries.push((
                            parent,
                            FolderEntry::File(File {
                                storage_id: self.storage_id,
                                id: object,
                                name,
                                size,
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
) -> Vec<Arc<FolderEntry>> {
    let mut entries = Vec::new();
    for (parent, entry) in all_entries.iter() {
        if *parent != root {
            continue;
        }

        match entry {
            FolderEntry::File(_) => {
                entries.push(Arc::new(entry.clone()));
            },
            FolderEntry::Folder(folder) => {
                let mut entry = folder.clone();
                entry.children = collect_children(folder.id, all_entries);
                entries.push(Arc::new(FolderEntry::Folder(entry)));
            },
        }
    }

    entries
}
