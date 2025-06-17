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

use crate::object::info::ProtectionStatus;

use alloc::vec::Vec;

use deku::{DekuRead, DekuWrite};

#[derive(Clone, Debug, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(
    id = "data_type",
    id_endian = "endian",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian, data_type: u16"
)]
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
    #[deku(id_pat = "_")]
    Reserved(#[deku(read_all)] Vec<u8>),
}

/// Marker trait for types that are valid for use as an [`ObjectProperty`] value
pub trait PropertyDataType {
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

impl PropertyDataType for Array<ObjectFormatCode> {
    const CODE: u16 = <Array<u16> as PropertyDataType>::CODE;
}
