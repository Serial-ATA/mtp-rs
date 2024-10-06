use crate::usb::{UsbDeviceDescriptor, UsbDeviceFlags};

use std::collections::HashSet;
use std::sync::LazyLock;

mod error;
mod usb;

static DEVICE_DESCRIPTORS: LazyLock<HashSet<UsbDeviceDescriptor>> =
	LazyLock::new(|| HashSet::from_iter(include!("../generated/devices.rs")));
