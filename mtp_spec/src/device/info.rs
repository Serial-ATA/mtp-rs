use crate::communication::event::Event;
use crate::communication::operation::Operation;
use crate::object::types::{Array, ObjectFormatCode, PtpString};

use deku::{DekuRead, DekuWrite};

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
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(id_type = "u16", endian = "big")]
#[repr(u16)]
pub enum FunctionalMode {
	#[deku(id = 0x0000)]
	Standard = 0x0000,
	#[deku(id = 0x0001)]
	SleepState = 0x0001,
	#[deku(id_pat = "t if t & 0x8000 == 0")]
	Reserved(u16),
	#[deku(id = 0xC001)]
	NonResponsivePlayback = 0xC001,
	#[deku(id = 0xC002)]
	ResponsivePlayback = 0xC002,
	#[deku(id_pat = "t if t & 0xC000 == 0x8000")]
	MtpVendorExtension(u16),
	#[deku(id_pat = "t if t & 0xC000 == 0xC000")]
	MtpDefined(u16),
}

#[derive(Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
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
	pub functional_mode: FunctionalMode,
	pub operations_supported: Array<Operation>,
	pub events_supported: Array<Event>,
	pub device_properties_supported: Array<u16>,
	pub capture_formats: Array<ObjectFormatCode>,
	pub playback_formats: Array<ObjectFormatCode>,
	/// Optional human-readable string that identifies the manufacturer of the device.
	pub manufacturer: Option<PtpString>,
	/// Optional human-readable string that identifies the model of the device.
	pub model: Option<PtpString>,
	/// Optional human-readable string that identifies the device's firmware version in a vendor-specific format.
	pub device_version: Option<PtpString>,
	pub serial_number: PtpString,
}
