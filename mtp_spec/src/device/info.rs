use crate::communication::event::EventCode;
use crate::communication::operation::Operation;
use crate::object::types::{Array, ObjectFormatCode, PtpString};

use crate::device::properties::DevicePropertyCode;
use deku::{DekuRead, DekuWrite};

/// Modes allow the device to express different states with different capabilities.
#[derive(Copy, Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(
    id_type = "u16",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[repr(u16)]
#[allow(missing_docs)]
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
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct DeviceInfo {
    /// This identifies the PTP version this device can support in hundredths. For MTP devices
    /// implemented under this specification, this shall contain the value `100` (representing 1.00).
    pub standard_version: u16,
    /// This identifies the PTP vendor-extension version in use by this device.
    pub vendor_extension_id: u32,
    /// This identifies the version of the MTP standard this device supports. It is expressed in hundredths.
    pub mtp_version: u16,
    /// This string is used to identify any extension sets applied to MTP
    pub mtp_extensions: PtpString,
    pub functional_mode: FunctionalMode,
    /// All operations that the device claims to support
    pub operations_supported: Array<Operation>,
    /// All events that the device claims to support
    pub events_supported: Array<EventCode>,
    /// All device properties that the device claims to support
    pub device_properties_supported: Array<DevicePropertyCode>,
    pub capture_formats: Array<ObjectFormatCode>,
    pub playback_formats: Array<ObjectFormatCode>,
    /// Optional human-readable string that identifies the manufacturer of the device.
    #[deku(map = "PtpString::parse_optional")]
    pub manufacturer: Option<PtpString>,
    /// Optional human-readable string that identifies the model of the device.
    #[deku(map = "PtpString::parse_optional")]
    pub model: Option<PtpString>,
    /// Optional human-readable string that identifies the device's firmware version in a vendor-specific format.
    #[deku(map = "PtpString::parse_optional")]
    pub device_version: Option<PtpString>,
    pub serial_number: PtpString,
}
