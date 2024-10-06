use crate::error::Result;
use bitflags::bitflags;

pub use rusb;
use rusb::constants::LIBUSB_DT_HUB;
use rusb::UsbContext;

bitflags! {
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
		const PLAYLIST_SPL_V1 = 0b0100_0000_0000;
		const NO_ZERO_READS = 0b1000_0000_0000;
		const PLAYLIST_SPL_V2 = 0b0001_0000_0000_0000;
		const UNIQUE_FILENAMES = 0b0010_0000_0000_0000;
		const BROKEN_BATTERY_LEVEL = 0b0100_0000_0000_0000;
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
	pub vendor: &'static str,
	pub vendor_id: u16,
	pub product: &'static str,
	pub product_id: u16,
	pub flags: UsbDeviceFlags,
}

/// Returns a list of MTP eligible devices.
///
/// # Errors
///
/// See [`rusb::devices`].
pub fn device_list() -> Result<()> {
	for device in rusb::devices()?.iter() {
		if is_mtp_eligible(&device)? {
			println!("Found MTP device: {:?}", device);
		}
	}

	Ok(())
}

fn is_mtp_eligible<T: UsbContext>(device: &rusb::Device<T>) -> Result<bool> {
	let descriptor = device.device_descriptor()?;
	if descriptor.descriptor_type() == LIBUSB_DT_HUB {
		return Ok(false);
	}

	descriptor.vendor_id();
	descriptor.product_id();

	Ok(false)
}
