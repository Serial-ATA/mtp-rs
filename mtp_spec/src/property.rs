use deku::ctx::Endian;
use deku::{DekuReader, DekuWriter};

use crate::device::properties::GetSet;
use crate::object::types::{ObjectHandle, PropertyDataType, PropertyValue};

/// Marker trait for properties
pub trait Property:
    sealed::Sealed + Send + PartialEq + core::fmt::Debug + Clone + for<'a> DekuReader<'a, Endian>
{
    /// The raw datacode for this property
    const CODE: u16;

    /// The type used to decode the property
    ///
    /// This is typically types like integers, [`PtpString`], or [`Array`]. But is occasionally used
    /// for custom enums for well-known variants.
    ///
    /// [`PtpString`]: crate::object::types::PtpString
    /// [`Array`]: crate::object::types::Array
    type DataType: PropertyDataType + for<'a> DekuReader<'a, Endian> + DekuWriter<Endian> + Send;

    /// The read/write status of the property
    ///
    /// This is, in most cases, dependent on the device.
    fn get_set(&self) -> GetSet;
}

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// A serialized [`Property`] for transport
///
/// See [`SerializeableProperty::serialize()`]
#[derive(Clone, Debug, PartialEq, Eq, deku::DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct SerializedProperty {
    code: u16,
    data_type: u16,
    object: ObjectHandle,
    // Passing in a dummy data type, doesn't actually matter for writing
    #[deku(ctx = "0")]
    value: PropertyValue,
}

pub trait SerializeableProperty<T>: Send {
    fn serialize(object: ObjectHandle, value: T) -> SerializedProperty;
}

impl<P> SerializeableProperty<P::DataType> for P
where
    P: Property,
    P::DataType: Into<PropertyValue>,
{
    fn serialize(object: ObjectHandle, value: P::DataType) -> SerializedProperty {
        SerializedProperty {
            code: P::CODE,
            data_type: P::DataType::CODE,
            object,
            value: value.into(),
        }
    }
}
