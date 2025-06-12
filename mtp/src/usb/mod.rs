//! USB transport backend for MTP
//!
//! ## Important Items
//!
//! Be sure to check out the docs of the following items:
//!
//! * [`device_list()`]
//! * [`Device`]
//! * [`DeviceHandle`]
//!
//! ## Usage
//!
//! This is a simple program that will find all connected MTP-eligible devices and print out their
//! storage devices.
//!
//! ```rust,no_run
//! use mtp::high_level::storages::DeviceStorageExt;
//! use mtp::usb::device_list;
//!
//! # #[tokio::main]
//! # async fn main() -> mtp::error::Result<()> {
//! let mut all_mtp_devices = device_list()?;
//! for maybe_device in all_mtp_devices {
//!     let device = maybe_device?;
//!
//!     println!(
//!         "Storages for device '{:?}':",
//!         device.info().product_string()
//!     );
//!     let (mut handle, session_id) = device.open().await?;
//!     let all_storages = handle.storages(session_id).await?;
//!
//!     for storage in all_storages {
//!         println!(
//!             "{} ({}/{} bytes free)",
//!             storage.description.unwrap_or(String::from("Unnamed")),
//!             storage.free_space,
//!             storage.max_capacity
//!         )
//!     }
//! }
//! # Ok(()) }
//! ```

mod handle;
pub use handle::*;

pub mod error;

mod init;

pub use init::*;

use bitflags::bitflags;
use std::collections::HashSet;
use std::sync::LazyLock;

static WELL_KNOWN_DEVICE_DESCRIPTORS: LazyLock<HashSet<UsbDeviceDescriptor>> =
    LazyLock::new(|| HashSet::from_iter(include!("../../generated/devices.rs")));

bitflags! {
    /// Flags indicating the bugs/unexpected behaviors of MTP devices
    ///
    /// These flags match device flags of `libmtp` here: <https://sourceforge.net/p/libmtp/code/ci/master/tree/src/device-flags.h>
    #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct UsbDeviceFlags: u32 {
        const BROKEN_MTP_GET_OBJECT_PROP_LIST_ALL = 0b0000_0001;
        const NO_RELEASE_INTERFACE = 0b0000_0010;
        const IGNORE_HEADER_ERRORS = 0b0000_0100;
        const ANDROID_BUGS = 0b0000_1000;
        const BROKEN_SET_SAMPLE_DIMENSIONS = 0b0001_0000;
        const UNLOAD_DRIVER = 0b0010_0000;
        const BROKEN_MTP_GET_OBJECT_PROP_LIST = 0b0100_0000;
        const ALWAYS_PROBE_DESCRIPTOR = 0b1000_0000;
        const CANNOT_HANDLE_DATEMODIFIED = 0b0001_0000_0000;
        const OGG_IS_UNKNOWN = 0b0010_0000_0000;
        /// The playlist format is the Samsung SPL format v1.00, rather than a proper MTP playlist.
        const PLAYLIST_SPL_V1 = 0b0100_0000_0000;
        const NO_ZERO_READS = 0b1000_0000_0000;
        /// The playlist format is the Samsung SPL format v2.00, rather than a proper MTP playlist.
        const PLAYLIST_SPL_V2 = 0b0001_0000_0000_0000;
        /// The device needs unique filenames, no two files can be named the same string.
        const UNIQUE_FILENAMES = 0b0010_0000_0000_0000;
        const BROKEN_BATTERY_LEVEL = 0b0100_0000_0000_0000;
        /// The device may need additional time to respond, extend its timeout
        const LONG_TIMEOUT = 0b1000_0000_0000_0000;
        const PROPLIST_OVERRIDES_OI = 0b0001_0000_0000_0000_0000;
        const SAMSUNG_OFFSET_BUG = 0b0010_0000_0000_0000_0000;
        const FLAC_IS_UNKNOWN = 0b0100_0000_0000_0000_0000;
        const ONLY_7BIT_FILENAMES = 0b1000_0000_0000_0000_0000;
        const IRIVER_OGG_ALZHEIMER = 0b0001_0000_0000_0000_0000_0000;
        const BROKEN_SEND_OBJECT_PROP_LIST = 0b0010_0000_0000_0000_0000_0000;
        const SONY_NWZ_BUGS = 0b0100_0000_0000_0000_0000_0000;
        const BROKEN_SET_OBJECT_PROP_LIST = 0b1000_0000_0000_0000_0000_0000;
        const SWITCH_MODE_BLACKBERRY = 0b0001_0000_0000_0000_0000_0000_0000;
        const FORCE_RESET_ON_CLOSE = 0b0010_0000_0000_0000_0000_0000_0000;
    }
}

/// A USB device descriptor.
///
/// This struct is used to identify MTP eligible devices.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct UsbDeviceDescriptor {
    /// A human-readable name for the vendor
    pub vendor: &'static str,
    /// The USB vendor ID, specified in the `idVendor` field of the device descriptor.
    pub vendor_id: u16,
    /// A human-readable name for the device
    pub product: &'static str,
    /// The USB product ID, specified in the `idProduct` field of the device descriptor.
    pub product_id: u16,
    /// Flags indicating problematic behavior of the device
    pub flags: UsbDeviceFlags,
}

enum MtpEligibility {
    Eligible,
    Ineligible,
}
