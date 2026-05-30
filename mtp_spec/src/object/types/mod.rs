mod array;
mod association;
mod datetime;
mod format_code;
mod object_handle;
pub mod properties;
mod string;

pub use array::*;
pub use association::*;
pub use datetime::*;
pub use format_code::*;
pub use object_handle::*;
pub use string::*;

use crate::device::properties::PerceivedDeviceTypeValue;
use crate::object::info::ProtectionStatus;
use crate::object::types::properties::{
    ConsumableStatus, HiddenStatus, MetaGenreForm, SystemObjectStatus,
};

use alloc::vec::Vec;

use deku::{DekuRead, DekuWrite};

macro_rules! define_property_value_methods {
    ($([$($tt:tt)*]),* $(,)?) => {
        impl PropertyValue {
            $(
                define_property_value_methods!(@AS_METHOD $($tt)*);
                define_property_value_methods!(@INTO_METHOD $($tt)*);
            )*

			/// Get a reference to the value if it matches the variant
			pub fn as_string(&self) -> Option<&PtpString> {
				match self {
					Self::String(v) => Some(v),
					_ => None,
				}
			}

			/// Consume the value and return it if it matches the variant
			pub fn into_string(self) -> Option<PtpString> {
				match self {
					Self::String(v) => Some(v),
					_ => None,
				}
			}
        }
    };
    (@AS_METHOD @REF_COPY $variant:ident, $_ref_ty:ty, $ty:ty) => {
        paste::paste! {
			/// Get a reference to the value if it matches the variant
            pub fn [<as_ $variant>](&self) -> Option<$ty> {
                match self {
                    Self::[<$variant:camel>](v) => Some(*v),
                    _ => None,
                }
            }
        }
    };
    (@AS_METHOD $variant:ident, $ref_ty:ty, $ty:ty) => {
        paste::paste! {
			/// Get a reference to the value if it matches the variant
            pub fn [<as_ $variant>](&self) -> Option<$ref_ty> {
                match self {
                    Self::[<$variant:camel>](v) => Some(v.as_slice()),
                    _ => None,
                }
            }
        }
    };
    (@INTO_METHOD $(@REF_COPY)? $variant:ident, $_ref_ty:ty, $ty:ty) => {
        paste::paste! {
			/// Consume the value and return it if it matches the variant
            pub fn [<into_ $variant>](self) -> Option<$ty> {
                match self {
                    Self::[<$variant:camel>](v) => Some(v),
                    _ => None,
                }
            }
        }
    };
}

/// Possible values for [`ObjectProperty`]s
///
/// [`ObjectProperty`]: properties::ObjectProperty
#[derive(Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(
    id = "data_type",
    id_endian = "endian",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian, data_type: u16"
)]
#[allow(missing_docs)]
pub enum PropertyValue {
    #[deku(id = "0x0000")]
    Undefined(#[deku(read_all)] Vec<u8>),
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
    /// Some other reserved or device-specific value
    #[deku(id_pat = "_")]
    Reserved(#[deku(read_all)] Vec<u8>),
}

define_property_value_methods! {
    [undefined, &[u8], Vec<u8>],
    [@REF_COPY i8, i8, i8],
    [@REF_COPY u8, u8, u8],
    [@REF_COPY i16, i16, i16],
    [@REF_COPY u16, u16, u16],
    [@REF_COPY i32, i32, i32],
    [@REF_COPY u32, u32, u32],
    [@REF_COPY i64, i64, i64],
    [@REF_COPY u64, u64, u64],
    [@REF_COPY i128, i128, i128],
    [@REF_COPY u128, u128, u128],
    [i8_array, &[i8], Array<i8>],
    [u8_array, &[u8], Array<u8>],
    [i16_array, &[i16], Array<i16>],
    [u16_array, &[u16], Array<u16>],
    [i32_array, &[i32], Array<i32>],
    [u32_array, &[u32], Array<u32>],
    [i64_array, &[i64], Array<i64>],
    [u64_array, &[u64], Array<u64>],
    [i128_array, &[i128], Array<i128>],
    [u128_array, &[u128], Array<u128>],
    [reserved, &[u8], Vec<u8>],
}

impl From<Vec<u8>> for PropertyValue {
    fn from(v: Vec<u8>) -> Self {
        Self::Undefined(v)
    }
}

impl From<i8> for PropertyValue {
    fn from(v: i8) -> Self {
        Self::I8(v)
    }
}

impl From<u8> for PropertyValue {
    fn from(v: u8) -> Self {
        Self::U8(v)
    }
}

impl From<i16> for PropertyValue {
    fn from(v: i16) -> Self {
        Self::I16(v)
    }
}

impl From<u16> for PropertyValue {
    fn from(v: u16) -> Self {
        Self::U16(v)
    }
}

impl From<i32> for PropertyValue {
    fn from(v: i32) -> Self {
        Self::I32(v)
    }
}

impl From<u32> for PropertyValue {
    fn from(v: u32) -> Self {
        Self::U32(v)
    }
}

impl From<i64> for PropertyValue {
    fn from(v: i64) -> Self {
        Self::I64(v)
    }
}

impl From<u64> for PropertyValue {
    fn from(v: u64) -> Self {
        Self::U64(v)
    }
}

impl From<i128> for PropertyValue {
    fn from(v: i128) -> Self {
        Self::I128(v)
    }
}

impl From<u128> for PropertyValue {
    fn from(v: u128) -> Self {
        Self::U128(v)
    }
}

impl From<Array<i8>> for PropertyValue {
    fn from(v: Array<i8>) -> Self {
        Self::I8Array(v)
    }
}

impl From<Array<u8>> for PropertyValue {
    fn from(v: Array<u8>) -> Self {
        Self::U8Array(v)
    }
}

impl From<Array<i16>> for PropertyValue {
    fn from(v: Array<i16>) -> Self {
        Self::I16Array(v)
    }
}

impl From<Array<u16>> for PropertyValue {
    fn from(v: Array<u16>) -> Self {
        Self::U16Array(v)
    }
}

impl From<Array<i32>> for PropertyValue {
    fn from(v: Array<i32>) -> Self {
        Self::I32Array(v)
    }
}

impl From<Array<u32>> for PropertyValue {
    fn from(v: Array<u32>) -> Self {
        Self::U32Array(v)
    }
}

impl From<Array<i64>> for PropertyValue {
    fn from(v: Array<i64>) -> Self {
        Self::I64Array(v)
    }
}

impl From<Array<u64>> for PropertyValue {
    fn from(v: Array<u64>) -> Self {
        Self::U64Array(v)
    }
}

impl From<Array<i128>> for PropertyValue {
    fn from(v: Array<i128>) -> Self {
        Self::I128Array(v)
    }
}

impl From<Array<u128>> for PropertyValue {
    fn from(v: Array<u128>) -> Self {
        Self::U128Array(v)
    }
}

impl From<PtpString> for PropertyValue {
    fn from(v: PtpString) -> Self {
        Self::String(v)
    }
}

impl From<DateTime> for PropertyValue {
    fn from(v: DateTime) -> Self {
        Self::String(v.into())
    }
}

/// Marker trait for types that are valid for use as an [`ObjectProperty`] value
///
/// [`ObjectProperty`]: properties::ObjectProperty
pub trait PropertyDataType: Eq + core::fmt::Debug + Clone {
    /// The raw datacode for this datatype
    const CODE: u16;
}

impl PropertyDataType for Vec<u8> {
    const CODE: u16 = 0x0000;
}

impl PropertyDataType for i8 {
    const CODE: u16 = 0x0001;
}

impl PropertyDataType for u8 {
    const CODE: u16 = 0x0002;
}

impl PropertyDataType for i16 {
    const CODE: u16 = 0x0003;
}

impl PropertyDataType for u16 {
    const CODE: u16 = 0x0004;
}

impl PropertyDataType for i32 {
    const CODE: u16 = 0x0005;
}

impl PropertyDataType for u32 {
    const CODE: u16 = 0x0006;
}

impl PropertyDataType for i64 {
    const CODE: u16 = 0x0007;
}

impl PropertyDataType for u64 {
    const CODE: u16 = 0x0008;
}

impl PropertyDataType for i128 {
    const CODE: u16 = 0x0009;
}

impl PropertyDataType for u128 {
    const CODE: u16 = 0x000A;
}

impl PropertyDataType for Array<i8> {
    const CODE: u16 = 0x4001;
}

impl PropertyDataType for Array<u8> {
    const CODE: u16 = 0x4002;
}

impl PropertyDataType for Array<i16> {
    const CODE: u16 = 0x4003;
}

impl PropertyDataType for Array<u16> {
    const CODE: u16 = 0x4004;
}

impl PropertyDataType for Array<i32> {
    const CODE: u16 = 0x4005;
}

impl PropertyDataType for Array<u32> {
    const CODE: u16 = 0x4006;
}

impl PropertyDataType for Array<i64> {
    const CODE: u16 = 0x4007;
}

impl PropertyDataType for Array<u64> {
    const CODE: u16 = 0x4008;
}

impl PropertyDataType for Array<i128> {
    const CODE: u16 = 0x4009;
}

impl PropertyDataType for Array<u128> {
    const CODE: u16 = 0x400A;
}

impl PropertyDataType for PtpString {
    const CODE: u16 = 0xFFFF;
}

// For some wrapper types as well (NOT from the spec)

impl PropertyDataType for ObjectHandle {
    const CODE: u16 = <u32 as PropertyDataType>::CODE;
}

impl PropertyDataType for ObjectFormatCode {
    const CODE: u16 = <u16 as PropertyDataType>::CODE;
}

impl PropertyDataType for AssociationType {
    const CODE: u16 = <u32 as PropertyDataType>::CODE;
}

impl PropertyDataType for ProtectionStatus {
    const CODE: u16 = <u16 as PropertyDataType>::CODE;
}

impl PropertyDataType for PerceivedDeviceTypeValue {
    const CODE: u16 = <u32 as PropertyDataType>::CODE;
}

impl PropertyDataType for Array<ObjectFormatCode> {
    const CODE: u16 = <Array<u16> as PropertyDataType>::CODE;
}

impl PropertyDataType for DateTime {
    const CODE: u16 = <PtpString as PropertyDataType>::CODE;
}

// Object property enums
impl PropertyDataType for HiddenStatus {
    const CODE: u16 = <u16 as PropertyDataType>::CODE;
}

impl PropertyDataType for SystemObjectStatus {
    const CODE: u16 = <u16 as PropertyDataType>::CODE;
}

impl PropertyDataType for ConsumableStatus {
    const CODE: u16 = <u8 as PropertyDataType>::CODE;
}

impl PropertyDataType for MetaGenreForm {
    const CODE: u16 = <u16 as PropertyDataType>::CODE;
}

// Used for the "undefined" properties that exist for some reason...
impl PropertyDataType for () {
    const CODE: u16 = 0x0000;
}
