#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u8)]
pub enum GetSet {
	ReadOnly = 0x00,
	ReadWrite = 0x01,
}

impl From<u8> for GetSet {
	fn from(value: u8) -> Self {
		match value {
			0x00 => GetSet::ReadOnly,
			0x01 => GetSet::ReadWrite,
			_ => unreachable!(),
		}
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RangeForm {
	minimum: Vec<u8>,
	maximum: Vec<u8>,
	step_size: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumerationForm {
	number_of_values: u16,
	values: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Form {
	Range(RangeForm),
	Enumeration(EnumerationForm),
}

pub struct DevicePropertyDescribing {
	pub device_property_code: u16,
	pub data_type: u16,
	pub get_set: GetSet,
	pub factory_default_value: Vec<u8>,
	pub current_value: Vec<u8>,
	pub form: Option<Form>,
}
