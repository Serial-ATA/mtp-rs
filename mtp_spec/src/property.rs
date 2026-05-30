//! Generic types for device/object properties

use crate::device::properties::GetSet;
use crate::object::{ObjectHandle, PropertyDataType, PropertyValue};

use deku::ctx::Endian;
use deku::{DekuReader, DekuWriter};

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
#[derive(Clone, Debug, PartialEq, Eq, deku::DekuRead, deku::DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct SerializedProperty {
    object: ObjectHandle,
    code: u16,
    data_type: u16,
    // Passing in a dummy data type, doesn't actually matter for writing
    #[deku(ctx = "0")]
    value: PropertyValue,
}

impl SerializedProperty {
    /// Get the associated [`ObjectHandle`]
    pub fn object(&self) -> ObjectHandle {
        self.object
    }

    /// Get the property code
    pub fn code(&self) -> u16 {
        self.code
    }

    /// Get the value of the property
    pub fn value(&self) -> &PropertyValue {
        &self.value
    }
}

/// A [`Property`] that can be serialized for transport
pub trait SerializeableProperty<T>: Send {
    /// Serialize a [`Property`] for transport
    ///
    /// This is useful for operations such as [`SetObjectPropList`] and [`SendObjectPropList`]
    ///
    /// # Examples
    ///
    /// ```
    /// use mtp_spec::object::types::properties::{DateCreated, DateModified, ObjectPropList};
    /// use mtp_spec::object::types::{DateTime, ObjectHandle};
    /// use mtp_spec::property::SerializeableProperty;
    ///
    /// // Some object handle obtained from the device...
    /// let object: ObjectHandle = ObjectHandle::NONE;
    ///
    /// // Then the properties can be used to build up an `ObjectPropList`.
    /// // It's important to note that the properties in a list do **NOT** all
    /// // have to refer to the same object handle.
    /// let properties = [
    ///     DateCreated::serialize(
    ///         object,
    ///         DateTime {
    ///             year: 1984,
    ///             ..Default::default()
    ///         },
    ///     ),
    ///     DateModified::serialize(
    ///         object,
    ///         DateTime {
    ///             year: 1984,
    ///             ..Default::default()
    ///         },
    ///     ),
    /// ]
    /// .into_iter()
    /// .collect::<ObjectPropList>();
    ///
    /// // Then the resulting `properties` can be used in SetObjectPropList, etc.
    /// ```
    ///
    /// [`SetObjectPropList`]: crate::communication::operation::SetObjectPropList
    /// [`SendObjectPropList`]: crate::communication::operation::SendObjectPropList
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
