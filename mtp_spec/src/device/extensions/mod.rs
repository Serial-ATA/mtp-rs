//! Popular vendor-specific MTP extensions
//!
//! Some MTP implementations (e.g. the Android stack) provide additional functionality. Before attempting
//! to use any of the extensions, you must check the `mtp_extensions` field of the device's [`DeviceInfo`]
//! to determine if they're supported.
//!
//! [`DeviceInfo`]: crate::device::info::DeviceInfo

pub mod android;
