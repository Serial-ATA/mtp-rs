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
//! use futures::stream::StreamExt;
//! use mtp::high_level::storages::DeviceStorageExt;
//! use mtp::usb::device_list;
//!
//! # #[tokio::main]
//! # async fn main() -> mtp::usb::error::Result<()> {
//! let mut all_mtp_devices = device_list().await?;
//! while let Some(maybe_device) = all_mtp_devices.next().await {
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

use std::collections::HashSet;
use std::sync::LazyLock;

use mtp_spec::device::DeviceFlags;

static WELL_KNOWN_DEVICE_DESCRIPTORS: LazyLock<HashSet<UsbDeviceDescriptor>> =
    LazyLock::new(|| HashSet::from_iter(include!("../../generated/devices.rs")));

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
    pub flags: DeviceFlags,
}

enum MtpEligibility {
    Eligible,
    Ineligible,
}
