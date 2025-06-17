use crate::object::types::{PropertyValue, PtpString};

use alloc::format;
use alloc::vec::Vec;

use deku::{DekuRead, DekuWrite, deku_derive};

// TODO: Ask if this is necessary
/// Wrapper around a [`PropertyValue`], used for standalone decoding
#[deku_derive(DekuRead)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropertyValueWrapper {
    #[deku(temp)]
    data_type: u16,
    #[deku(ctx = "*data_type")]
    value: PropertyValue,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, DekuRead, DekuWrite)]
#[deku(
    id_type = "u8",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub enum GetSet {
    #[deku(id = "0x00")]
    ReadOnly = 0x00,
    #[deku(id = "0x01")]
    ReadWrite = 0x01,
}

#[derive(Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(ctx = "endian: deku::ctx::Endian, data_type: u16")]
pub struct RangeForm {
    #[deku(ctx = "endian, data_type")]
    minimum: PropertyValue,
    #[deku(ctx = "endian, data_type")]
    maximum: PropertyValue,
    #[deku(ctx = "endian, data_type")]
    step_size: PropertyValue,
}

#[deku_derive(DekuRead)]
#[derive(Clone, Debug, Eq, PartialEq)]
#[deku(ctx = "endian: deku::ctx::Endian, data_type: u16")]
pub struct EnumerationForm {
    #[deku(temp)]
    number_of_values: u16,
    #[deku(count = "number_of_values", ctx = "endian, data_type")]
    values: Vec<PropertyValue>,
}

#[derive(Clone, Debug, Eq, PartialEq, DekuRead)]
#[deku(
    id = "form",
    ctx = "endian: deku::ctx::Endian, form: u8, data_type: u16"
)]
pub enum Form {
    #[deku(id = "0x01")]
    Range(#[deku(ctx = "endian, data_type")] RangeForm),
    #[deku(id = "0x02")]
    Enumeration(#[deku(ctx = "endian, data_type")] EnumerationForm),
}

#[deku_derive(DekuRead)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevicePropDesc {
    /// A unique code that identifies the property.
    pub device_property_code: u16,
    data_type: u16,
    /// Indicates whether the property is read-only or read-write.
    pub get_set: GetSet,
    /// The factory default value of the property.
    #[deku(ctx = "*data_type")]
    pub factory_default_value: PropertyValue,
    /// The current value of the property.
    #[deku(ctx = "*data_type")]
    pub current_value: PropertyValue,
    #[deku(temp)]
    form_flag: u8,
    #[deku(cond = "*form_flag != 0x00", ctx = "*form_flag, *data_type")]
    pub form: Option<Form>,
}
