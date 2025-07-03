use crate::communication::Parameter;
use crate::device::storage::id::StorageId;
use crate::object::types::{
    Association, AssociationType, DateTime, ObjectFormatCode, ObjectHandle, PtpString,
};

use alloc::vec::Vec;

use deku::ctx::{Endian, Limit};
use deku::no_std_io::{Cursor, Read, Seek, SeekFrom};
use deku::prelude::Reader;
use deku::{DekuError, DekuRead, DekuReader, DekuWrite};

/// The write-protection status of an object
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, DekuRead, DekuWrite)]
#[repr(u16)]
#[deku(
    id_type = "u16",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub enum ProtectionStatus {
    /// This object has no protection; it may be modified or deleted arbitrarily, and its properties
    /// may be modified freely.
    #[deku(id = "0x0000")]
    NoProtection = 0x0000,
    /// This object cannot be deleted or modified; none of the properties of this object can be modified
    /// by the initiator. (However, properties can be modified by the device that contains the object.)
    #[deku(id = "0x0001")]
    ReadOnly = 0x0001,
    /// This object’s binary component cannot be deleted or modified; however, any object properties
    /// may be modified if allowed by the object property constraints.
    #[deku(id = "0x8002")]
    ReadOnlyData = 0x8002,
    /// This object’s properties may be read and modified, and it may be moved or deleted on the device,
    /// but this object’s binary data may not be retrieved from the device using a [`GetObject`] operation.
    ///
    /// [`GetObject`]: crate::communication::operation::GetObject
    #[deku(id = "0x8003")]
    NonTransferableData = 0x8003,
    /// All other values
    #[deku(id_pat = "_", default)]
    Reserved,
}

impl From<u16> for ProtectionStatus {
    fn from(value: u16) -> Self {
        match value {
            0x0000 => ProtectionStatus::NoProtection,
            0x0001 => ProtectionStatus::ReadOnly,
            0x8002 => ProtectionStatus::ReadOnlyData,
            0x8003 => ProtectionStatus::NonTransferableData,
            _ => ProtectionStatus::Reserved,
        }
    }
}

impl From<ProtectionStatus> for Parameter {
    fn from(value: ProtectionStatus) -> Self {
        Parameter::new(value as u32)
    }
}

/// PTP-compatible thumbnail information for image objects
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct Thumbnail {
    /// The format of the image
    pub format: ObjectFormatCode,
    /// The size of the data component of the object in bytes.
    ///
    /// If the object is larger than `2^32` bytes in size (4GB), this field shall contain a value
    /// of [`u32::MAX`].
    pub compressed_size: u32,
    /// The width in pixels
    pub width: u32,
    /// The height in pixels
    pub height: u32,
    /// The bit depth of the image
    pub bit_depth: u32,
}

impl Thumbnail {
    fn parse_optional(thumbnail: Thumbnail) -> Result<Option<Thumbnail>, DekuError> {
        if thumbnail.format == ObjectFormatCode::Unknown(0)
            && thumbnail.compressed_size == 0
            && thumbnail.width == 0
            && thumbnail.height == 0
            && thumbnail.bit_depth == 0
        {
            Ok(None)
        } else {
            Ok(Some(thumbnail))
        }
    }
}

/// Information about an object residing on the responder
#[derive(Clone, Debug, Eq, PartialEq, DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct ObjectInfo {
    /// The storage in which this object is located
    pub storage_id: StorageId,
    /// The format of the object's binary data
    pub object_format: ObjectFormatCode,
    /// The write-protection status of this object
    pub protection_status: ProtectionStatus,
    /// The size of the data component of the object in bytes.
    ///
    /// If the object is larger than `2^32` bytes in size (4GB), this field shall contain a value
    /// of [`u32::MAX`].
    pub compressed_size: u32,
    /// PTP-compatible thumbnail information for image objects
    ///
    /// This field will most likely be unused by responders.
    pub thumbnail: Option<Thumbnail>,
    /// The parent of this object, if it exists in a hierarchy
    #[deku(map = "ObjectHandle::parse_optional")]
    pub parent_object: Option<ObjectHandle>,
    /// The association type, if this is an association
    pub association_type: Option<AssociationType>,
    /// Unused in MTP, but required by PTP
    pub sequence_number: u32,
    /// The file name of this object, without any directory or file system information.
    pub filename: PtpString,
    /// The creation date of this object
    #[deku(map = "DateTime::parse_optional")]
    pub date_created: Option<DateTime>,
    /// The last modification date of this object
    #[deku(map = "DateTime::parse_optional")]
    pub date_modified: Option<DateTime>,
    /// Keywords associated with the object, separated by ' '
    pub keywords: PtpString,
}

impl DekuReader<'_, ()> for ObjectInfo {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        _: (),
    ) -> Result<Self, DekuError> {
        Self::from_reader_with_ctx(reader, Endian::Big)
    }
}

// Parsing of ObjectInfo is extra complicated due to Samsung writing 64-bit compressed sizes for some reason ??
impl DekuReader<'_, Endian> for ObjectInfo {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        ctx: Endian,
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        // The size of the object info all the way up to (not including) the file name field
        const OBJECT_INFO_SIZE_UP_TO_FILE_NAME: usize = size_of::<StorageId>()
            + size_of::<ObjectFormatCode>()
            + size_of::<ProtectionStatus>()
            + size_of::<u32>()
            + size_of::<Thumbnail>()
            + size_of::<ObjectHandle>()
            + size_of::<Association>()
            + size_of::<u32>();

        let storage_id = StorageId::from_reader_with_ctx(reader, ctx)?;
        let object_format = ObjectFormatCode::from_reader_with_ctx(reader, ctx)?;
        let protection_status = ProtectionStatus::from_reader_with_ctx(reader, ctx)?;
        let compressed_size = u32::from_reader_with_ctx(reader, ctx)?;

        // Bytes read up to this point
        const BYTES_READ: usize = size_of::<StorageId>()
            + size_of::<ObjectFormatCode>()
            + size_of::<ProtectionStatus>()
            + size_of::<u32>();

        // The offset of the `filename` field from our current position
        const FILE_NAME_FROM_OFFSET: usize = OBJECT_INFO_SIZE_UP_TO_FILE_NAME - BYTES_READ;

        // Read the rest of the buffer
        let rest = <Vec<u8>>::from_reader_with_ctx(reader, (Limit::end(), ()))?;

        let mut is_64_bit_compressed_size = false;
        if rest[FILE_NAME_FROM_OFFSET] == 0 && rest[FILE_NAME_FROM_OFFSET + 4] != 0 {
            // Samsung bug. Need to discard the next 4 bytes, as a 64 bit `compressed_size` was written.
            is_64_bit_compressed_size = true;
        }

        // Now continue parsing as normal...

        let mut reader = Reader::new(Cursor::new(rest));
        if is_64_bit_compressed_size {
            log::warn!("Received a 64 bit compressed size, discarding the next 4 bytes");
            reader
                .seek(SeekFrom::Current(4))
                .map_err(|e| DekuError::Io(e.kind()))?;
        }

        let thumbnail =
            Thumbnail::parse_optional(Thumbnail::from_reader_with_ctx(&mut reader, ctx)?)?;
        let parent_object =
            ObjectHandle::parse_optional(ObjectHandle::from_reader_with_ctx(&mut reader, ctx)?)?;
        let association_type = AssociationType::from_reader_with_ctx(&mut reader, ctx)?;

        let association_type_opt;
        if object_format == ObjectFormatCode::Association {
            association_type_opt = Some(association_type);
        } else {
            association_type_opt = None;
        }

        let sequence_number = u32::from_reader_with_ctx(&mut reader, ctx)?;
        let filename = PtpString::from_reader_with_ctx(&mut reader, ctx)?;
        let date_created =
            DateTime::parse_optional(PtpString::from_reader_with_ctx(&mut reader, ctx)?)?;
        let date_modified =
            DateTime::parse_optional(PtpString::from_reader_with_ctx(&mut reader, ctx)?)?;
        let keywords = PtpString::from_reader_with_ctx(&mut reader, ctx)?;

        Ok(ObjectInfo {
            storage_id,
            object_format,
            protection_status,
            compressed_size,
            thumbnail,
            parent_object,
            association_type: association_type_opt,
            sequence_number,
            filename,
            date_created,
            date_modified,
            keywords,
        })
    }
}
