use crate::error::Result;

use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::LazyLock;
use std::time::Duration;

use bitflags::bitflags;
pub use nusb;
use nusb::descriptors::language_id::US_ENGLISH;

static WELL_KNOWN_DEVICE_DESCRIPTORS: LazyLock<HashSet<UsbDeviceDescriptor>> =
	LazyLock::new(|| HashSet::from_iter(include!("../generated/devices.rs")));

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

enum MtpEligibility {
	Eligible,
	Ineligible,
}

#[derive(Clone)]
pub struct Device {
	info: nusb::DeviceInfo,
	handle: Option<nusb::Device>,
}

impl Debug for Device {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Device").field("info", &self.info).finish()
	}
}

impl From<nusb::DeviceInfo> for Device {
	fn from(info: nusb::DeviceInfo) -> Self {
		Self { info, handle: None }
	}
}

impl Device {
	pub fn open(&mut self) -> Result<()> {
		if self.handle.is_none() {
			self.handle = Some(self.info.open()?);
		}

		Ok(())
	}
}

impl Device {
	fn check_mtp_eligibility(&mut self) -> Result<MtpEligibility> {
		let is_well_known = WELL_KNOWN_DEVICE_DESCRIPTORS.iter().any(|d| {
			d.vendor_id == self.info.vendor_id() && d.product_id == self.info.product_id()
		});

		if is_well_known {
			return Ok(MtpEligibility::Eligible);
		}

		if self.check_for_mtp_descriptor()? {
			return Ok(MtpEligibility::Eligible);
		}

		Ok(MtpEligibility::Ineligible)
	}

	fn check_for_mtp_descriptor(&mut self) -> Result<bool> {
		const CLASS_PER_INTERFACE: u8 = 0;
		const CLASS_COMM: u8 = 2;
		const CLASS_PTP: u8 = 6;
		const CLASS_VENDOR_SPECIFIC: u8 = 255;

		const LIKELY_DEVICE_CLASSES: &[u8] = &[
			CLASS_PER_INTERFACE,
			CLASS_COMM,
			CLASS_PTP,
			0xEF,
			CLASS_VENDOR_SPECIFIC,
		];

		if !LIKELY_DEVICE_CLASSES.contains(&self.info.class()) {
			return Ok(false);
		}

		let Ok(handle) = self.info.open() else {
			return Ok(false);
		};

		self.handle = Some(handle);

		let handle = self.handle.as_ref().unwrap();
		for config in handle.configurations() {
			for interface in config.interfaces() {
				for alt_settings in interface.alt_settings() {
					if alt_settings.num_endpoints() != 3 {
						continue;
					}

					if alt_settings.class() == CLASS_VENDOR_SPECIFIC {
						todo!()
					}

					let Some(string_index) = alt_settings.string_index() else {
						continue;
					};

					let timeout = Duration::from_secs(1);

					let Ok(interface_name) =
						handle.get_string_descriptor(string_index, US_ENGLISH, timeout)
					else {
						continue;
					};

					if interface_name.contains("MTP") {
						return Ok(true);
					}
				}
			}
		}

		Ok(false)
	}
}

/// Returns a list of connected devices that are MTP eligible.
///
/// # Errors
///
/// See [`nusb::list_devices`].
pub fn device_list() -> Result<impl Iterator<Item = Result<Device>>> {
	Ok(nusb::list_devices()?.filter_map(|info| {
		let mut device = Device::from(info);
		match device.check_mtp_eligibility() {
			Ok(MtpEligibility::Eligible) => Some(Ok(device)),
			Ok(MtpEligibility::Ineligible) => None,
			Err(e) => Some(Err(e)),
		}
	}))
}
