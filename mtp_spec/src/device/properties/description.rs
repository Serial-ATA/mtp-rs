use super::{EnumerationForm, Form, GetSet, RangeForm};
use crate::object::types::PropertyValue;

use alloc::format;
use alloc::vec::Vec;

use deku::ctx::Endian;
use deku::no_std_io::{Cursor, Read, Seek, Write};
use deku::prelude::Writer;
use deku::{DekuEnumExt, DekuError, DekuReader, DekuWriter, deku_derive};

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
    pub value: PropertyValue,
}

impl DekuWriter<Endian> for PropertyValueWrapper {
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        ctx: Endian,
    ) -> Result<(), DekuError> {
        let data_type = self.value.deku_id()?;
        self.value.to_writer(writer, (ctx, data_type))
    }
}

/// Descriptor for a device property
#[derive(Clone, Debug, PartialEq)]
pub struct DevicePropDesc {
    /// A unique code that identifies the property.
    pub device_property_code: u16,
    /// Indicates whether the property is read-only or read-write.
    pub get_set: GetSet,
    /// The factory default value of the property.
    pub factory_default_value: PropertyValue,
    /// The current value of the property.
    pub current_value: PropertyValue,
    /// The constraints on the property, see [`Form`]
    pub form: Option<Form<PropertyValueWrapper>>,
}

impl deku::DekuContainerRead<'_> for DevicePropDesc {
    fn from_reader<R: Read + Seek>(
        (reader, bits_read): (&mut R, usize),
    ) -> Result<(usize, Self), DekuError> {
        let reader = &mut deku::reader::Reader::new(reader);
        if bits_read != 0 {
            reader.skip_bits(bits_read)?;
        }
        let value = Self::from_reader_with_ctx(reader, ())?;
        Ok((reader.bits_read, value))
    }
    #[allow(non_snake_case)]
    fn from_bytes(
        (bytes, bits_read): (&'_ [u8], usize),
    ) -> Result<((&'_ [u8], usize), Self), DekuError> {
        let mut cursor = Cursor::new(bytes);
        let reader = &mut deku::reader::Reader::new(&mut cursor);
        if bits_read != 0 {
            reader.skip_bits(bits_read)?;
        }
        let value = Self::from_reader_with_ctx(reader, ())?;
        let read_whole_byte = (reader.bits_read % 8) == 0;
        let idx = if read_whole_byte {
            reader.bits_read / 8
        } else {
            (reader.bits_read - (reader.bits_read % 8)) / 8
        };
        Ok(((&bytes[idx..], reader.bits_read % 8), value))
    }
}

impl DekuReader<'_, Endian> for DevicePropDesc {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut deku::reader::Reader<R>,
        endian: Endian,
    ) -> Result<Self, DekuError> {
        let device_property_code = u16::from_reader_with_ctx(reader, endian)?;
        let data_type = u16::from_reader_with_ctx(reader, endian)?;
        let get_set = GetSet::from_reader_with_ctx(reader, endian)?;
        let factory_default_value =
            PropertyValue::from_reader_with_ctx(reader, (endian, data_type))?;
        let current_value = PropertyValue::from_reader_with_ctx(reader, (endian, data_type))?;
        let form_flag = u8::from_reader_with_ctx(reader, endian)?;

        let form;
        match form_flag {
            0 => form = None,
            1 => {
                let minimum = PropertyValueWrapper {
                    value: PropertyValue::from_reader_with_ctx(reader, (endian, data_type))?,
                };

                let maximum = PropertyValueWrapper {
                    value: PropertyValue::from_reader_with_ctx(reader, (endian, data_type))?,
                };

                let step_size = PropertyValueWrapper {
                    value: PropertyValue::from_reader_with_ctx(reader, (endian, data_type))?,
                };

                form = Some(Form::Range(RangeForm {
                    minimum,
                    maximum,
                    step_size,
                }));
            },
            2 => {
                let number_of_values = u16::from_reader_with_ctx(reader, endian)?;
                let mut values = Vec::with_capacity(number_of_values as usize);
                for _ in 0..number_of_values {
                    values.push(PropertyValueWrapper {
                        value: PropertyValue::from_reader_with_ctx(reader, (endian, data_type))?,
                    });
                }

                form = Some(Form::Enumeration(EnumerationForm { values }));
            },
            _ => {
                return Err(DekuError::Parse(
                    format!("Invalid form_flag: {form_flag}").into(),
                ));
            },
        }

        Ok(Self {
            device_property_code,
            get_set,
            factory_default_value,
            current_value,
            form,
        })
    }
}

impl DekuReader<'_> for DevicePropDesc {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut deku::reader::Reader<R>,
        _: (),
    ) -> Result<Self, DekuError> {
        <Self as DekuReader<'_, Endian>>::from_reader_with_ctx(reader, Endian::Big)
    }
}
