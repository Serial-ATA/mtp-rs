use alloc::format;

use deku::{DekuRead, DekuWrite};

#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u32", endian = "little")]
#[repr(u32)]
pub enum FolderType {
    #[deku(id = "0x0000")]
    Generic,
    /// The folder is bi-directionally linked, and must have object references to each object
    /// contained within it.
    #[deku(id = "0x0001")]
    BiDirectionallyLinked,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(
    id_type = "u32",
    endian = "little",
    ctx = "_endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Little"
)]
#[repr(u16)]
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
#[deku(id_type = "u16")]
pub enum Association {
    #[deku(id = "0x0000")]
    Undefined {
        #[deku(endian = "little")]
        undefined: u32,
    },
    #[deku(id = "0x0001")]
    GenericFolder { ty: FolderType },
    #[deku(id = "0x0002")]
    Album {
        #[deku(endian = "little")]
        reserved: u32,
    },
    #[deku(id = "0x0003")]
    TimeSequence {
        #[deku(endian = "little")]
        default_playback_delta: u32,
    },
    #[deku(id = "0x0004")]
    HorizontalPanoramic {
        #[deku(endian = "little")]
        unused: u32,
    },
    #[deku(id = "0x0005")]
    VerticalPanoramic {
        #[deku(endian = "little")]
        images_per_row: u32,
    },
    #[deku(id = "0x0006")]
    Panoramic2d {
        #[deku(endian = "little")]
        undefined: u32,
    },
    #[deku(id = "0x0007")]
    AncillaryData {
        #[deku(endian = "little")]
        unused: u32,
    },
    /// All other values with bit 15 set to 0
    #[deku(id_pat = "t if t & 0x8000 == 0")]
    Reserved {
        #[deku(endian = "little")]
        unused: u32,
    },
    /// All other values with bit 15 set to 1 and bit 14 set to 0
    #[deku(id_pat = "t if t & 0xC000 == 0x8000")]
    VendorDefined {
        #[deku(endian = "little")]
        undefined: u32,
    },
    /// All other values with bit 15 set to 1 and bit 14 set to 1
    #[deku(id_pat = "_")]
    Mtp {
        #[deku(endian = "little")]
        undefined: u32,
    },
}
