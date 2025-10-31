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
    #[deku(id = "0x01")]
    Range = 0x01,
    #[deku(id = "0x02")]
    Enumeration = 0x02,
}

#[derive(Clone, Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(ctx = "endian: Endian")]
pub struct RangeForm<T>
where
    T: for<'a> DekuReader<'a, Endian> + DekuWriter<Endian> + Clone,
{
    #[deku(ctx = "endian")]
    pub minimum: T,
    #[deku(ctx = "endian")]
    pub maximum: T,
    #[deku(ctx = "endian")]
    pub step_size: T,
}

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

#[derive(Clone, Debug, PartialEq, DekuRead)]
#[deku(id = "form", ctx = "endian: Endian, form: u8")]
pub enum Form<T>
where
    T: for<'a> DekuReader<'a, Endian> + DekuWriter<Endian> + Clone,
{
    #[deku(id = "0x01")]
    Range(#[deku(ctx = "endian")] RangeForm<T>),
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
