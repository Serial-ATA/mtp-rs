use deku::{DekuRead, DekuWrite};

/// The type of an [`Association::GenericFolder`]
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(
    id_type = "u32",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[repr(u32)]
#[allow(missing_docs)]
pub enum FolderType {
    #[deku(id = "0x0000")]
    Generic,
    /// The folder is bi-directionally linked, and must have object references to each object
    /// contained within it.
    #[deku(id = "0x0001")]
    BiDirectionallyLinked,
}

/// The type of an [`Association`]
///
/// This just matches the variants of [`Association`]
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(
    id_type = "u32",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[repr(u16)]
#[allow(missing_docs)]
pub enum AssociationType {
    #[deku(id = "0x0000")]
    Undefined,
    #[deku(id = "0x0001")]
    GenericFolder,
    #[deku(id = "0x0002")]
    Album,
    #[deku(id = "0x0003")]
    TimeSequence,
    #[deku(id = "0x0004")]
    HorizontalPanoramic,
    #[deku(id = "0x0005")]
    VerticalPanoramic,
    #[deku(id = "0x0006")]
    Panoramic2d,
    #[deku(id = "0x0007")]
    AncillaryData,
    #[deku(id_pat = "t if t & 0x8000 == 0")]
    Reserved(u32),
    #[deku(id_pat = "t if t & 0xC000 == 0x8000")]
    VendorDefined(u32),
    #[deku(id_pat = "_")]
    Mtp(u32),
}

/// The type of the collection to which an object is associated.
///
/// Note that all association types have an associated descriptor, which will be unused in most cases.
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(
    id_type = "u32",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[allow(missing_docs)]
pub enum Association {
    #[deku(id = "0x0000")]
    Undefined { undefined: u32 },
    #[deku(id = "0x0001")]
    GenericFolder { ty: FolderType },
    #[deku(id = "0x0002")]
    Album { reserved: u32 },
    #[deku(id = "0x0003")]
    TimeSequence { default_playback_delta: u32 },
    #[deku(id = "0x0004")]
    HorizontalPanoramic { unused: u32 },
    #[deku(id = "0x0005")]
    VerticalPanoramic { images_per_row: u32 },
    #[deku(id = "0x0006")]
    Panoramic2d { undefined: u32 },
    #[deku(id = "0x0007")]
    AncillaryData { unused: u32 },
    /// All other values with bit 15 set to 0
    #[deku(id_pat = "t if t & 0x8000 == 0")]
    Reserved { unused: u32 },
    /// All other values with bit 15 set to 1 and bit 14 set to 0
    #[deku(id_pat = "t if t & 0xC000 == 0x8000")]
    VendorDefined { undefined: u32 },
    /// All other values with bit 15 set to 1 and bit 14 set to 1
    #[deku(id_pat = "_")]
    Mtp { undefined: u32 },
}
