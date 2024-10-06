use crate::object::types::{Array, PtpString};

use alloc::vec::Vec;

use deku::{DekuRead, DekuWrite};

#[derive(Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct DeviceInfo {
	/// This identifies the PTP version this device can support in hundredths. For MTP devices
	/// implemented under this specification, this shall contain the value `100` (representing 1.00).
	pub standard_version: u16,
	/// This identifies the PTP vendor-extension version in use by this device. For MTP devices
	/// implemented under this specification, this shall contain the value `0xFFFFFFFF`.
	pub vendor_extension_id: u32,
	/// This identifies the version of the MTP standard this device supports. It is expressed in
	/// hundredths. The final version of this specification will identify the correct value to place
	/// in this field.
	pub mtp_version: u16,
	/// This string is used to identify any extension sets applied to MTP
	pub mtp_extensions: PtpString,
	/// Modes allow the device to express different states with different capabilities. If the device
	/// supports only one mode, this field shall contain the value `0x00000000`.
	///
	/// The following values are defined:
	///
	/// | Value                                                     | Description             |
	/// |-----------------------------------------------------------|-------------------------|
	/// | `0x0000`                                                  | Standard mode           |
	/// | `0x0001`                                                  | Sleep state             |
	/// | All other values with bit 15 set to 0                     | Reserved                |
	/// | `0xC001`                                                  | Non-responsive playback |
	/// | `0xC002`                                                  | Responsive playback     |
	/// | All other values with bit 15 set to 1 and bit 14 set to 0 | MTP vendor extension    |
	/// | All other values with bit 15 set to 1 and bit 14 set to 1 | MTP-defined             |
	pub functional_mode: u16,
	pub operations_supported: Array<u16>,
	pub events_supported: Array<u16>,
	pub device_properties_supported: Array<u16>,
	pub capture_formats: Array<u16>,
	pub playback_formats: Array<u16>,
	/// Optional human-readable string that identifies the manufacturer of the device.
	pub manufacturer: Option<PtpString>,
	/// Optional human-readable string that identifies the model of the device.
	pub model: Option<PtpString>,
	/// Optional human-readable string that identifies the device's firmware version in a vendor-specific format.
	pub device_version: Option<PtpString>,
	pub serial_number: PtpString,
}
