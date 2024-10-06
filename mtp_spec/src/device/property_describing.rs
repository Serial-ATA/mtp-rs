use crate::object::types::{Array, PtpString};

use alloc::format;
use alloc::vec::Vec;

use deku::{deku_derive, DekuRead, DekuWrite};

#[derive(Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(
	endian = "endian",
	id = "data_type",
	ctx = "endian: deku::ctx::Endian, data_type: u16"
)]
pub enum PropertyValue {
	#[deku(id = "0x0000")]
	Undefined,
	#[deku(id = "0x0001")]
	I8(i8),
	#[deku(id = "0x0002")]
	U8(u8),
	#[deku(id = "0x0003")]
	I16(i16),
	#[deku(id = "0x0004")]
	U16(u16),
	#[deku(id = "0x0005")]
	I32(i32),
	#[deku(id = "0x0006")]
	U32(u32),
	#[deku(id = "0x0007")]
	I64(i64),
	#[deku(id = "0x0008")]
	U64(u64),
	#[deku(id = "0x0009")]
	I128(i128),
	#[deku(id = "0x000A")]
	U128(u128),
	#[deku(id = "0x4001")]
	I8Array(Array<i8>),
	#[deku(id = "0x4002")]
	U8Array(Array<u8>),
	#[deku(id = "0x4003")]
	I16Array(Array<i16>),
	#[deku(id = "0x4004")]
	U16Array(Array<u16>),
	#[deku(id = "0x4005")]
	I32Array(Array<i32>),
	#[deku(id = "0x4006")]
	U32Array(Array<u32>),
	#[deku(id = "0x4007")]
	I64Array(Array<i64>),
	#[deku(id = "0x4008")]
	U64Array(Array<u64>),
	#[deku(id = "0x4009")]
	I128Array(Array<i128>),
	#[deku(id = "0x400A")]
	U128Array(Array<u128>),
	#[deku(id = "0xFFFF")]
	String(PtpString),
	#[deku(id_pat = "_")]
	Reserved,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, DekuRead, DekuWrite)]
#[deku(id_type = "u8", endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub enum GetSet {
	#[deku(id = "0x00")]
	ReadOnly = 0x00,
	#[deku(id = "0x01")]
	ReadWrite = 0x01,
}

#[derive(Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian, data_type: u16")]
pub struct RangeForm {
	#[deku(ctx = "data_type")]
	minimum: PropertyValue,
	#[deku(ctx = "data_type")]
	maximum: PropertyValue,
	#[deku(ctx = "data_type")]
	step_size: PropertyValue,
}

#[deku_derive(DekuRead)]
#[derive(Clone, Debug, Eq, PartialEq)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian, data_type: u16")]
pub struct EnumerationForm {
	#[deku(temp)]
	number_of_values: u16,
	#[deku(count = "number_of_values", ctx = "data_type")]
	values: Vec<PropertyValue>,
}

#[derive(Clone, Debug, Eq, PartialEq, DekuRead)]
#[deku(
	endian = "endian",
	id = "form",
	ctx = "endian: deku::ctx::Endian, form: u8, data_type: u16"
)]
pub enum Form {
	#[deku(id = "0x01")]
	Range(#[deku(ctx = "data_type")] RangeForm),
	#[deku(id = "0x02")]
	Enumeration(#[deku(ctx = "data_type")] EnumerationForm),
}

#[deku_derive(DekuRead)]
#[derive(Clone, Debug, Eq, PartialEq)]
#[deku(endian = "big")]
pub struct DevicePropDesc {
	/// A unique code that identifies the property.
	pub device_property_code: u16,
	#[deku(temp)]
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
