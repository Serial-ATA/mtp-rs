use crate::communication::Parameter;
use crate::device::properties::{Form, FormType, GetSet};
use crate::object::types::{
    Array, ArrayEncodable, DateTime, ObjectFormatCode, ObjectHandle, PropertyDataType,
    PropertyValue, PtpString,
};

use alloc::borrow::Cow;
use alloc::format;

use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek};
use deku::{DekuReader, DekuWriter};

/// Marker trait for object properties
pub trait DeviceProperty:
    sealed::Sealed + Send + PartialEq + core::fmt::Debug + Clone + for<'a> DekuReader<'a, Endian>
{
    /// The raw datacode for this property
    const CODE: u16;

    type DataType: PropertyDataType + for<'a> DekuReader<'a, Endian> + DekuWriter<Endian>;

    /// The read/write status of the property
    ///
    /// This is, in most cases, dependent on the device.
    fn get_set(&self) -> GetSet;
}

mod sealed {
    use super::DeviceProperty;

    pub trait Sealed {}

    impl<T: DeviceProperty> Sealed for T {}
}

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
    P: DeviceProperty,
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

macro_rules! define_device_properties {
	(
		$(
			$(#[$meta:meta])*
			pub struct $name:ident {
				properties: {
					data_type: $datatype:ty,
					$(get_set: $get_set:expr,)?
					valid_forms: [$($form:ident),* $(,)?]
				},
				code: $code:literal,
				$(form: $($form_tt:tt)* $(,)?)?
			}
		)*
	) => {
		#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, deku::DekuRead, deku::DekuWrite)]
		#[repr(u16)]
		#[deku(
			id_type = "u16",
			id_endian = "endian",
			ctx = "endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Big"
		)]
		pub enum DevicePropertyCode {
			$(
			#[deku(id = $code)]
			$name = $code,
			)*
			#[deku(id_pat = "_")]
			SomethingElse,
		}

		impl ArrayEncodable for DevicePropertyCode {}

		impl From<DevicePropertyCode> for Parameter {
			fn from(code: DevicePropertyCode) -> Self {
				Parameter::new(code as u32)
			}
		}

		impl Default for DevicePropertyCode {
			fn default() -> Self {
        		Self::Undefined
    		}
		}

		$(
			$(#[$meta])*
			#[derive(Clone, Debug, PartialEq)]
			pub struct $name {
				pub default_value: $datatype,
				pub group_code: u32,
				pub get_set: GetSet,
				pub form: define_device_properties!(@FORM_TY ($($($form_tt)*)?) | $datatype),
			}

			impl DeviceProperty for $name {
				const CODE: u16 = $code;

				type DataType = $datatype;

				fn get_set(&self) -> GetSet {
					self.get_set
				}
			}

			define_device_properties!(@FORM_ENUM $($($form_tt)*)?);

			impl $name {
				const VALID_FORMS: &'static [FormType] = &[$(FormType::$form),*];
			}

			impl DekuReader<'_, ()> for $name {
				#[inline]
				fn from_reader_with_ctx<R: Read + Seek>(reader: &mut ::deku::reader::Reader<R>, _: ()) -> core::result::Result<Self, ::deku::DekuError> {
					Self::from_reader_with_ctx(reader, Endian::Big)
				}
			}

			impl DekuReader<'_, Endian> for $name {
				fn from_reader_with_ctx<R: Read + Seek>(reader: &mut ::deku::reader::Reader<R>, ctx: Endian) -> core::result::Result<Self, ::deku::DekuError> {
					let code = u16::from_reader_with_ctx(reader, ctx)?;
					if code != Self::CODE {
						return Err(deku::DekuError::Assertion(Cow::Owned(format!(
							"Expected property code {}, got {code}", Self::CODE
						))))
					}

					let data_type = u16::from_reader_with_ctx(reader, ctx)?;
					if data_type != <Self as DeviceProperty>::DataType::CODE {
						return Err(deku::DekuError::Assertion(Cow::Owned(format!(
							"Expected datatype code {}, got {data_type}", <Self as DeviceProperty>::DataType::CODE
						))))
					}

					let get_set = GetSet::from_reader_with_ctx(reader, ctx)?;
					$(
						if get_set != $get_set {
							return Err(deku::DekuError::Assertion(Cow::Owned(format!(
								"Expected get_set to be {:?}, got {get_set:?}", $get_set
							))))
						}
					)?

					let default_value = <$datatype>::from_reader_with_ctx(reader, ctx)?;
					let group_code = u32::from_reader_with_ctx(reader, ctx)?;
					let form_flag = FormType::from_reader_with_ctx(reader, ctx)?;
					if !Self::VALID_FORMS.contains(&form_flag) {
						return Err(deku::DekuError::Assertion(Cow::Owned(format!(
							"Expected form flag to be one of {:?}, got {form_flag:?}",
							Self::VALID_FORMS
						))))
					}

					let form = define_device_properties!(@FORM_DEFINITION reader ctx ($($($form_tt)*)?) | $datatype);

					Ok(Self {
						default_value,
						group_code,
						get_set,
						form,
					})
				}
			}
		)*
	};

	(@FORM_ENUM enum $enum_name:ident {
		$($body:tt)*
	}) => {
		#[derive(Clone, Debug, PartialEq, Eq, Hash, deku::DekuRead)]
		#[deku(
			id_type = "u16",
			id_endian = "endian",
			ctx = "endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Big"
		)]
		pub enum $enum_name {
			$($body)*
		}
	};

	(@FORM_ENUM $_ty:ty) => {};
	(@FORM_ENUM) => {};

	(@FORM_DEFINITION $reader:ident $ctx:ident (enum $enum_name:ident {
		$($_tt:tt)*
	}) | $_data_type:ty) => {{
		let form: $enum_name = <$enum_name>::from_reader_with_ctx($reader, $ctx)?;
		form
	}};

	(@FORM_DEFINITION $reader:ident $ctx:ident ($form:ty) | $_data_type:ty) => {{
		let form: $form = <$form>::from_reader_with_ctx($reader, $ctx)?;
		form
	}};

	(@FORM_DEFINITION $reader:ident $ctx:ident () | $data_type:ty) => {{
		let form: $data_type = <$data_type>::from_reader_with_ctx($reader, $ctx)?;
		form
	}};

	(@FORM_TY (enum $enum_name:ident {
		$($_tt:tt)*
	}) | $_data_type:ty) => { $enum_name };
	(@FORM_TY ($ty:ty) | $_data_type:ty) => { $ty };
	(@FORM_TY () | $data_type:ty) => { $data_type };
}

define_device_properties! {
    /// This is not used
    pub struct Undefined {
        properties: {
            data_type: (),
            valid_forms: [None]
        },
        code: 0x5000,
    }

    /// The current battery level of the receiver
    ///
    /// The battery level is indicated by an unsigned, read-only integer, and constrained by either
    /// an Enumeration or Range of integers (See [`Form`]).
    pub struct BatteryLevel {
        properties: {
            data_type: u8,
            get_set: GetSet::ReadOnly,
            valid_forms: [Range, Enumeration]
        },
        code: 0x5001,
        form: Form<u16>
    }

    /// The current [`FunctionalMode`] of the responder
    pub struct FunctionalMode {
        properties: {
            data_type: u16,
            valid_forms: [Enumeration]
        },
        code: 0x5002,
        form: FunctionalMode
    }

    /// The width and height of images captured by the responder
    ///
    /// The value is a [`PtpString`] in the form: `"WxH"`.
    ///
    /// An example value would be: `"640x480"` for a width of 640 pixels and a height of 480 pixels.
    ///
    /// This can be represented in both [`RangeForm`] and [`EnumerationForm`]
    ///
    /// Examples:
    /// * [`RangeForm`]
    ///   * A minimum of `"1x1"`, and a maximum of `"1024x768"`, with a step of `"1x1"`
    /// * [`EnumerationForm`]
    ///   * `values` will be a list of all possible image dimensions
    pub struct ImageSize {
        properties: {
            data_type: PtpString,
            valid_forms: [Range, Enumeration]
        },
        code: 0x5003,
        form: FunctionalMode
    }

    /// The level of compression used by the responder
    ///
    /// Smaller values indicate low quality and high compression, and large values indicate high quality
    /// and low compression.
    ///
    /// The value of this property is fully device-specific.
    pub struct CompressionSetting {
        properties: {
            data_type: u8,
            valid_forms: [Range, Enumeration]
        },
        code: 0x5004,
        form: Form<u8>
    }

    /// How the responder weights different color channels
    pub struct WhiteBalance {
        properties: {
            data_type: u16,
            valid_forms: [Enumeration]
        },
        code: 0x5005,
        form: enum WhiteBalanceValue {
            #[deku(id = "0x0000")]
            Undefined,
            /// The white balance is set directly by using the [`RgbGain`] property, and is static until changed.
            #[deku(id = "0x0001")]
            Manual,
            /// The responder attempts to set the white balance using some kind of automatic mechanism.
            #[deku(id = "0x0002")]
            Automatic,
        }
    }
}
