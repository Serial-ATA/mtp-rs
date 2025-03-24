use super::{MtpEligibility, UsbDeviceFlags};

use std::fmt::Debug;
use std::time::Duration;

pub use nusb;
use nusb::descriptors::language_id::US_ENGLISH;
use nusb::transfer::{Direction, EndpointType};

#[derive(Clone)]
pub struct Device {
	info: nusb::DeviceInfo,
	flags: UsbDeviceFlags,
	handle: Option<nusb::Device>,
}

impl Debug for Device {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Device").field("info", &self.info).finish()
	}
}

impl From<nusb::DeviceInfo> for Device {
	fn from(info: nusb::DeviceInfo) -> Self {
		Self {
			info,
			flags: UsbDeviceFlags::empty(),
			handle: None,
		}
	}
}

impl Device {
	pub fn open(self) -> Result<super::handle::DeviceHandle, super::error::UsbError> {
		// MTP has 3 endpoints: 2 bulk, 1 interrupt
		const MTP_ENDPOINT_COUNT: u8 = 3;

		let device = self.info.open()?;

		let mut interface_num = None;
		let mut endpoints = None;
		'outer: for config in device.configurations() {
			for interface in config.interfaces() {
				for alt_settings in interface.alt_settings() {
					// Not applicable
					if alt_settings.num_endpoints() != MTP_ENDPOINT_COUNT {
						continue;
					}

					let mut bulk_in = None;
					let mut bulk_in_buffer_size = 0;
					let mut bulk_out = None;
					let mut bulk_out_buffer_size = 0;
					let mut interrupt = None;
					for endpoint in alt_settings.endpoints() {
						match endpoint.transfer_type() {
							EndpointType::Bulk => match endpoint.direction() {
								Direction::In => {
									bulk_in = Some(endpoint.address());
									bulk_in_buffer_size = endpoint.max_packet_size();
								},
								Direction::Out => {
									bulk_out = Some(endpoint.address());
									bulk_out_buffer_size = endpoint.max_packet_size();
								},
							},
							EndpointType::Interrupt => interrupt = Some(endpoint.address()),
							_ => {},
						}
					}

					let (Some(bulk_in), Some(bulk_out), Some(interrupt)) =
						(bulk_in, bulk_out, interrupt)
					else {
						// Not all endpoints found
						continue;
					};

					endpoints = Some(super::handle::Endpoints {
						bulk_in,
						bulk_in_buffer_size,
						bulk_out,
						bulk_out_buffer_size,
						interrupt,
					});

					interface_num = Some(interface.interface_number());
					break 'outer;
				}
			}
		}

		let Some(interface_num) = interface_num else {
			return Err(super::error::UsbError::NoApplicableInterface);
		};

		let interface = device.claim_interface(interface_num)?;

		Ok(super::handle::DeviceHandle::new(
			device,
			self.flags,
			interface,
			endpoints.unwrap(),
		))
	}
}

impl Device {
	fn check_mtp_eligibility(&mut self) -> Result<MtpEligibility, super::error::UsbError> {
		if let Some(well_known_entry) = super::WELL_KNOWN_DEVICE_DESCRIPTORS.iter().find(|d| {
			d.vendor_id == self.info.vendor_id() && d.product_id == self.info.product_id()
		}) {
			self.flags = well_known_entry.flags;
			return Ok(MtpEligibility::Eligible);
		}

		if self.check_for_mtp_descriptor()? {
			return Ok(MtpEligibility::Eligible);
		}

		Ok(MtpEligibility::Ineligible)
	}

	fn check_for_mtp_descriptor(&mut self) -> Result<bool, super::error::UsbError> {
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
pub fn device_list(
) -> Result<impl Iterator<Item = Result<Device, super::error::UsbError>>, super::error::UsbError> {
	Ok(nusb::list_devices()?.filter_map(|info| {
		let mut device = Device::from(info);
		match device.check_mtp_eligibility() {
			Ok(MtpEligibility::Eligible) => Some(Ok(device)),
			Ok(MtpEligibility::Ineligible) => None,
			Err(e) => Some(Err(e)),
		}
	}))
}
