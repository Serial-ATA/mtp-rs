//! MTP device property definitions
//!
//! MTP defines a set of properties that devices may allow you to read/write.
//!
//! # Forms
//!
//! See the [object property form section]
//!
//! [object property form section]: https://docs.rs/mtp_spec/latest/mtp_spec/object/types/properties/index.html#forms

mod description;

pub use description::*;

mod impls;
pub use impls::*;

use alloc::vec::Vec;

use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek};
use deku::prelude::Reader;
use deku::{DekuError, DekuRead, DekuReader, DekuWrite, DekuWriter, deku_derive};

/// The read/write status of a property
///
/// The mutability of individual properties is determined by the device. Attempting to overwrite
/// any read-only properties will return an error.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, DekuRead, DekuWrite)]
#[deku(
    id_type = "u8",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub enum GetSet {
    /// The property cannot be modified
    #[deku(id = "0x00")]
    ReadOnly = 0x00,
    /// The property can be modified
    #[deku(id = "0x01")]
    ReadWrite = 0x01,
}

/// The type of a [`Form`]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, DekuRead)]
#[deku(
    id_type = "u16",
    id_endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[repr(u16)]
enum FormType {
    #[deku(id = "0x00")]
    None = 0x00,
    /// See [`Form::Range`]
    #[deku(id = "0x01")]
    Range = 0x01,
    /// See [`Form::Enumeration`]
    #[deku(id = "0x02")]
    Enumeration = 0x02,
}

/// A range of possible property values
///
/// See [`Form`] for an explanation of forms.
#[derive(Clone, Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(ctx = "endian: Endian")]
#[allow(missing_docs)]
pub struct RangeForm<T>
where
    T: for<'a> DekuReader<'a, Endian> + DekuWriter<Endian> + Clone,
{
    /// The start of the range
    #[deku(ctx = "endian")]
    pub minimum: T,
    /// The end of the range
    #[deku(ctx = "endian")]
    pub maximum: T,
    #[deku(ctx = "endian")]
    pub step_size: T,
}

/// A fixed list of possible property values
///
/// See [`Form`] for an explanation of forms.
#[deku_derive(DekuRead)]
#[derive(Clone, Debug, PartialEq)]
#[deku(ctx = "endian: Endian")]
pub struct EnumerationForm<T>
where
    T: for<'a> DekuReader<'a, Endian> + Clone,
{
    #[deku(temp)]
    number_of_values: u16,
    #[deku(count = "number_of_values", ctx = "endian")]
    values: Vec<T>,
}

/// The constraints placed on a certain property
///
/// [`DeviceProperty`]s can be constrained in multiple ways, depending on the property and device.
///
/// For example, [`BatteryLevel`], in the ideal case, would be represented as a [`RangeForm`] with a `minimum` of `0`,
/// a maximum of `100`, and a `step_size` of `1`.
///
/// However, a device could *also*:
///
/// * Change any of those values (e.g. provide a `step_size` of `5`, minimum of `1`, etc.)
/// * Or even use an [`EnumerationForm`], which will list every possible [`BatteryLevel`] value
///
/// The forms available depend on the property. [`BatteryLevel`] supports both [`RangeForm`] and [`EnumerationForm`],
/// while something like [`FunctionalMode`] only supports [`EnumerationForm`], and others like [`DateTime`] support neither.
#[derive(Clone, Debug, PartialEq, DekuRead)]
#[deku(id = "form", ctx = "endian: Endian, form: u8")]
pub enum Form<T>
where
    T: for<'a> DekuReader<'a, Endian> + DekuWriter<Endian> + Clone,
{
    /// The property's value is constrained to a range of values
    #[deku(id = "0x01")]
    Range(#[deku(ctx = "endian")] RangeForm<T>),
    /// The property's value is constrained to a fixed list of values
    #[deku(id = "0x02")]
    Enumeration(#[deku(ctx = "endian")] EnumerationForm<T>),
}

// For standalone decoding
impl<T> DekuReader<'_, Endian> for Form<T>
where
    T: for<'a> DekuReader<'a, Endian> + DekuWriter<Endian> + Clone,
{
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        ctx: Endian,
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let form_code = u8::from_reader_with_ctx(reader, ctx)?;
        <Form<T>>::from_reader_with_ctx(reader, (ctx, form_code))
    }
}
