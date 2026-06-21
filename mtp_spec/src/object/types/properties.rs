//! MTP Object property definitions
//!
//! MTP defines a set of properties on [`objects`] that devices may allow you to read/write.
//!
//! # Property Descriptions
//!
//! Each property comes with a description that can be queried with [`Device::get_object_prop_desc()`].
//! These can specify default values, read/write capability, and as covered in the next section, constraints
//! on the value.
//!
//! ## Forms
//!
//! Some property descriptions will have a "form" field (e.g. [`ObjectFileName`]) that sets additional
//! constraints on what values one can set for the property.
//!
//! The different types of forms are:
//!
//! * [`RangeForm`]
//! * [`EnumerationForm`]
//! * [`DateTime`] form (handled as a special case)
//!   * Properties expected to be in DateTime form will automatically be parsed as a [`DateTime`]
//! * [`FixedLengthArrayForm`]
//! * [`RegularExpressionForm`]
//! * [`ByteArrayForm`]
//! * [`LongStringForm`]
//!
//! See the specific types for more information.
//!
//! In the case of [`ObjectFileName`], the device may provide a [`RegularExpressionForm`], where it could,
//! for example, restrict the value to only alphanumeric characters.
//!
//! [`Device::get_object_prop_desc()`]: crate::device::Device::get_object_prop_desc
//! [`objects`]: ObjectHandle

use crate::communication::Parameter;
use crate::device::properties::{EnumerationForm, GetSet, RangeForm};
use crate::object::types::{
    Array, ArrayEncodable, DateTime, ObjectFormatCode, ObjectHandle, PropertyDataType, PtpString,
};
use crate::property::{Property, SerializedProperty};

use alloc::borrow::Cow;
use alloc::format;

use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek, Write};
use deku::prelude::{Reader, Writer};
use deku::{DekuError, DekuReader, DekuWriter, deku_derive};

/// An [`ObjectProperty`] list
///
/// This is used in the [`GetObjectPropList`], [`SetObjectPropList`], and [`SendObjectPropList`] operations.
///
/// [`GetObjectPropList`]: crate::communication::operation::GetObjectPropList
/// [`SetObjectPropList`]: crate::communication::operation::SetObjectPropList
/// [`SendObjectPropList`]: crate::communication::operation::SendObjectPropList
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectPropList(pub Vec<SerializedProperty>);

impl FromIterator<SerializedProperty> for ObjectPropList {
    fn from_iter<T: IntoIterator<Item = SerializedProperty>>(iter: T) -> Self {
        Self(Vec::from_iter(iter))
    }
}

impl IntoIterator for ObjectPropList {
    type Item = SerializedProperty;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl DekuWriter<Endian> for ObjectPropList {
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        ctx: Endian,
    ) -> Result<(), DekuError> {
        (self.0.len() as u32).to_writer(writer, ctx)?;

        for prop in &self.0 {
            prop.to_writer(writer, ctx)?;
        }

        Ok(())
    }
}

impl DekuReader<'_, Endian> for ObjectPropList {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        ctx: Endian,
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let count = u32::from_reader_with_ctx(reader, ctx)?;
        let mut props = Vec::with_capacity(count as usize);

        for _ in 0..count {
            props.push(SerializedProperty::from_reader_with_ctx(reader, ctx)?);
        }

        Ok(Self(props))
    }
}

/// Get an array of [`ObjectPropertyDesc`] arrays, each describing an allowed collection of ranges
///
/// This is used in the [`GetInterdependentPropDesc`] operation.
///
/// [`GetInterdependentPropDesc`]: crate::communication::operation::GetInterdependentPropDesc
#[deku_derive(DekuRead)]
#[derive(Clone, Debug, PartialEq, Eq)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct InterdependentPropDesc {
    #[deku(temp)]
    number_of_interdependencies: u32,
    /// A list of interdependent object properties
    #[deku(count = "number_of_interdependencies")]
    pub interdependencies: Vec<InterdependentProperties>,
}

/// A list of interdependent object properties
///
/// This is part of [`InterdependentPropDesc`].
#[deku_derive(DekuRead)]
#[derive(Clone, Debug, PartialEq, Eq)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct InterdependentProperties {
    #[deku(temp)]
    number_of_prop_descs: u16,
    /// A list of interdependent object properties
    #[deku(count = "number_of_prop_descs")]
    pub properties: Vec<SerializedProperty>,
}

/// Marker trait for object properties
pub trait ObjectProperty: Property {}

/// Information about fixed-length array properties
#[derive(Copy, Clone, Debug, PartialEq, Eq, deku::DekuRead, deku::DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct FixedLengthArrayForm {
    /// The exact number of elements that must be included in the array for the property
    ///
    /// Devices will only accept values that *exactly* match this length.
    pub length: u32,
}

/// Information about byte array properties, such as [`RepresentativeSampleData`]
#[derive(Copy, Clone, Debug, PartialEq, Eq, deku::DekuRead, deku::DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct ByteArrayForm {
    /// The maximum number of bytes allowed in this property
    ///
    /// Devices should accept any value with a byte count less than or equal to this value.
    pub max_length: u32,
}

/// Information about long string properties, such as [`Lyrics`]
#[derive(Copy, Clone, Debug, PartialEq, Eq, deku::DekuRead, deku::DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct LongStringForm {
    /// The maximum number of characters allowed in this property
    ///
    /// Devices should accept any value with a character count less than or equal to this value.
    pub max_length: u32,
}

/// A regex constraining the syntax of a property, such as [`ObjectFileName`]
///
/// Devices will only accept values that satisfy the specified regex.
///
/// An empty string typically indicates that any value will be accepted.
#[derive(Clone, Debug, PartialEq, Eq, deku::DekuRead, deku::DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
pub struct RegularExpressionForm(pub PtpString);

macro_rules! define_object_property_descriptions {
	(
		$(
			$(#[$meta:meta])*
			pub struct $name:ident {
				properties: {
					data_type: $datatype:ty,
					$(data_type_parser: $data_type_parser:expr,)?
					$(get_set: $get_set:expr,)?
					valid_forms: [$($form:ident),* $(,)?]
				},
				code: $code:literal,
				$(form: $($form_tt:tt)* $(,)?)?
			}
		)*
	) => {
		/// The `PropertyCode` of an [`ObjectProperty`]
		#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, deku::DekuRead, deku::DekuWrite)]
		#[repr(u16)]
		#[deku(
			id_type = "u16",
			id_endian = "endian",
			ctx = "endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Big"
		)]
		#[allow(missing_docs)]
		pub enum ObjectPropertyCode {
			$(
			#[deku(id = $code)]
			$name = $code,
			)*
			#[deku(id = 0xFFFF)]
			All,
			#[deku(id_pat = "_")]
			SomethingElse,
		}

		impl ArrayEncodable for ObjectPropertyCode {}

		impl From<ObjectPropertyCode> for Parameter {
			fn from(code: ObjectPropertyCode) -> Self {
				Parameter::new(code as u32)
			}
		}

		impl Default for ObjectPropertyCode {
			fn default() -> Self {
				ObjectPropertyCode::SomethingElse
			}
		}

		impl From<u16> for ObjectPropertyCode {
			fn from(code: u16) -> Self {
				match code {
					$( $code => Self::$name, )*
					0xFFFF => Self::All,
					_ => Self::SomethingElse,
				}
			}
		}

		$(
			define_object_property_descriptions!(@PROPERTY_STRUCT $(#[$meta])* $name ($datatype) $($($form_tt)*)?);

			impl crate::property::sealed::Sealed for $name {}

			impl Property for $name {
				const CODE: u16 = $code;

				type DataType = $datatype;

				fn get_set(&self) -> GetSet {
					self.get_set
				}
			}

			impl ObjectProperty for $name {}

			define_object_property_descriptions!(@FORM_ENUM $($($form_tt)*)?);

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
					if data_type != <Self as Property>::DataType::CODE {
						return Err(deku::DekuError::Assertion(Cow::Owned(format!(
							"Expected datatype code {}, got {data_type}", <Self as Property>::DataType::CODE
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

					let default_value = define_object_property_descriptions!(@PARSE_DEFAULT reader, ctx, $datatype | $($data_type_parser)?);
					let group_code = u32::from_reader_with_ctx(reader, ctx)?;
					let form_flag = FormType::from_reader_with_ctx(reader, ctx)?;
					if !Self::VALID_FORMS.contains(&form_flag) {
						return Err(deku::DekuError::Assertion(Cow::Owned(format!(
							"Expected form flag to be one of {:?}, got {form_flag:?}",
							Self::VALID_FORMS
						))))
					}

					define_object_property_descriptions!(@FORM_PARSE_AND_RET reader ctx $($($form_tt)*)?, (default_value, group_code, get_set, form_flag))
				}
			}
		)*
	};

	// Property with a form (type name)
	(@PROPERTY_STRUCT $(#[$meta:meta])* $name:ident ($datatype:ty) $form:ty) => {
		$(#[$meta])*
		#[derive(Clone, Debug, PartialEq)]
		pub struct $name {
			/// The device's assigned default for this property
			pub default_value: $datatype,
			/// The retrieval group this property belongs to
			pub group_code: u32,
			/// The read-only status of the property
			pub get_set: GetSet,
			/// Device-defined constraints on the data. See [`forms`]
			///
			/// [`forms`]: https://docs.rs/mtp_spec/latest/mtp_spec/object/types/properties/index.html#forms
			pub form: $form,
		}
	};

	// Property with an optional form
	(@PROPERTY_STRUCT $(#[$meta:meta])* $name:ident ($datatype:ty) @MAYBE $form:ty) => {
		$(#[$meta])*
		#[derive(Clone, Debug, PartialEq)]
		pub struct $name {
			/// The device's assigned default for this property
			pub default_value: $datatype,
			/// The retrieval group this property belongs to
			pub group_code: u32,
			/// The read-only status of the property
			pub get_set: GetSet,
			/// Device-defined constraints on the data. See [`forms`]
			///
			/// [`forms`]: https://docs.rs/mtp_spec/latest/mtp_spec/object/types/properties/index.html#forms
			pub form: Option<$form>,
		}
	};

	// Property without a form
	(@PROPERTY_STRUCT $(#[$meta:meta])* $name:ident ($datatype:ty)) => {
		$(#[$meta])*
		#[derive(Clone, Debug, PartialEq)]
		pub struct $name {
			/// The device's assigned default for this property
			pub default_value: $datatype,
			/// The retrieval group this property belongs to
			pub group_code: u32,
			/// The read-only status of the property
			pub get_set: GetSet,
		}
	};

	// Datatype default parse
	(@PARSE_DEFAULT $reader:expr, $ctx:expr, $datatype:ty |) => {{
		let _val: $datatype = <$datatype>::from_reader_with_ctx($reader, $ctx)?;
		_val
	}};

	// Datatype with custom parser fn
	(@PARSE_DEFAULT $reader:expr, $ctx:expr, $datatype:ty | $data_type_parser:expr) => {{
		let _val: $datatype = $data_type_parser($reader, $ctx)?;
		_val
	}};

	(@FORM_ENUM $(@MAYBE)? $_ty:ty) => {};
	(@FORM_ENUM) => {};

	// Form expected (inline enum)
	(@FORM_PARSE_AND_RET $reader:ident $ctx:ident enum $enum_name:ident {
		$($_tt:tt)*
	}, ($default_value:expr, $group_code:expr, $get_set:expr)) => {{
		let form: $enum_name = <$enum_name>::from_reader_with_ctx($reader, $ctx)?;
		Ok(Self {
			default_value: $default_value,
			group_code: $group_code,
			get_set: $get_set,
			form,
		})
	}};

	// Form expected (type name)
	(@FORM_PARSE_AND_RET $reader:ident $ctx:ident $form:ty, ($default_value:expr, $group_code:expr, $get_set:expr, $_form_flag:expr)) => {{
		let form: $form = <$form>::from_reader_with_ctx($reader, $ctx)?;
		Ok(Self {
			default_value: $default_value,
			group_code: $group_code,
			get_set: $get_set,
			form,
		})
	}};

	// Optional form
	(@FORM_PARSE_AND_RET $reader:ident $ctx:ident @MAYBE $form:ty, ($default_value:expr, $group_code:expr, $get_set:expr, $form_flag:expr)) => {{
		let form: Option<$form> = match $form_flag {
			v if v == FormType::None => None,
			_ => Some(<$form>::from_reader_with_ctx($reader, $ctx)?),
		};

		Ok(Self {
			default_value: $default_value,
			group_code: $group_code,
			get_set: $get_set,
			form,
		})
	}};

	// No form
	(@FORM_PARSE_AND_RET $reader:ident $ctx:ident, ($default_value:expr, $group_code:expr, $get_set:expr, $_form_flag:expr)) => {{
		Ok(Self {
			default_value: $default_value,
			group_code: $group_code,
			get_set: $get_set,
		})
	}};
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, deku::DekuRead)]
#[deku(
    id_type = "u8",
    id_endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[repr(u8)]
enum FormType {
    #[deku(id = "0x00")]
    None = 0x00,
    #[deku(id = "0x01")]
    Range = 0x01,
    #[deku(id = "0x02")]
    Enumeration = 0x02,
    #[deku(id = "0x03")]
    DateTime = 0x03,
    #[deku(id = "0x04")]
    FixedLengthArray = 0x04,
    #[deku(id = "0x05")]
    RegularExpression = 0x05,
    #[deku(id = "0x06")]
    ByteArray = 0x06,
    #[deku(id = "0x07")]
    LongString = 0x07,
}

define_object_property_descriptions! {
    /// The storage on which this object exists
    ///
    /// This value is also available on the [`ObjectInfo`].
    ///
    /// [`ObjectInfo`]: crate::object::info::ObjectInfo
    pub struct StorageId {
        properties: {
            data_type: u32,
            get_set: GetSet::ReadOnly,
            valid_forms: [None]
        },
        code: 0xDC01,
    }

    /// The object format code describes this object
    ///
    /// This value is also available on the [`ObjectInfo`].
    ///
    /// [`ObjectInfo`]: crate::object::info::ObjectInfo
    pub struct ObjectFormat {
        properties: {
            data_type: ObjectFormatCode,
            get_set: GetSet::ReadOnly,
            valid_forms: [None]
        },
        code: 0xDC02,
    }

    /// The write-protection status of the binary component of the object
    pub struct ProtectionStatus {
        properties: {
            data_type: crate::object::info::ProtectionStatus,
            get_set: GetSet::ReadOnly,
            valid_forms: [Enumeration]
        },
        code: 0xDC03,
    }

    /// The size of the binary component of the object, in bytes
    pub struct ObjectSize {
        properties: {
            data_type: u64,
            get_set: GetSet::ReadOnly,
            valid_forms: [None]
        },
        code: 0xDC04,
    }

    /// The [`AssociationType`] of the object
    ///
    /// [`AssociationType`]: crate::object::types::AssociationType
    pub struct AssociationType {
        properties: {
            data_type: crate::object::types::AssociationType,
            valid_forms: [Enumeration]
        },
        code: 0xDC05,
    }

    /// Additional information about Association objects
    ///
    /// See [`Association`] for information on how to handle the returned value, based on the
    /// object's [`AssociationType`].
    ///
    /// [`Association`]: crate::object::types::Association
    pub struct AssociationDesc {
        properties: {
            data_type: u32,
            valid_forms: [None]
        },
        code: 0xDC06,
    }

    /// The file name of the object
    ///
    /// This may or may not reference the actual file name of the object on the device, and does not
    /// contain path information.
    pub struct ObjectFileName {
        properties: {
            data_type: PtpString,
            valid_forms: [None, RegularExpression]
        },
        code: 0xDC07,
        form: @MAYBE RegularExpressionForm
    }

    /// The date and time when the object was created
    pub struct DateCreated {
        properties: {
            data_type: DateTime,
            data_type_parser: DateTime::parse_ptp_string,
            valid_forms: [DateTime]
        },
        code: 0xDC08,
    }

    /// The date and time when the object was last altered
    pub struct DateModified {
        properties: {
            data_type: DateTime,
            data_type_parser: DateTime::parse_ptp_string,
            valid_forms: [DateTime]
        },
        code: 0xDC09,
    }

    /// A list of keywords associated with the object, separated by ' '.
    pub struct Keywords {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC0A,
    }

    /// The object handle of this object's parent, if it exists in a hierarchy
    ///
    /// For root objects, or devices that do not support associations, the value will be [`ObjectHandle::NONE`].
    pub struct ParentObject {
        properties: {
            data_type: ObjectHandle,
            get_set: GetSet::ReadOnly,
            valid_forms: [None]
        },
        code: 0xDC0B,
    }

    /// Objects formats allowed in this folder
    ///
    /// If there is no restriction, the returned array will be empty.
    pub struct AllowedFolderContents {
        properties: {
            data_type: Array<ObjectFormatCode>,
            get_set: GetSet::ReadOnly,
            valid_forms: [None]
        },
        code: 0xDC0C,
    }

    /// Whether an object is intended to be shown to users
    pub struct Hidden {
        properties: {
            data_type: HiddenStatus,
            valid_forms: [Enumeration]
        },
        code: 0xDC0D,
        form: EnumerationForm<u16>
    }

    /// Whether an object is a system file, and is required for property functioning of a device
    pub struct SystemObject {
        properties: {
            data_type: SystemObjectStatus,
            valid_forms: [Enumeration]
        },
        code: 0xDC0E,
        form: EnumerationForm<u16>
    }

    /// A unique identifier for an object, determined by the responder
    ///
    /// As long as this object is present on the device, it must contain the same persistent unique
    /// object identifier, and it shall be the only object with that identifier.
    pub struct PersistentUniqueObjectIdentifier {
        properties: {
            data_type: u128,
            get_set: GetSet::ReadOnly,
            valid_forms: [None]
        },
        code: 0xDC41,
    }

    /// An identifier for retaining state between sessions, determined by the initiator
    pub struct SyncId {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC42,
    }

    /// An XML document specifying object properties
    ///
    /// The contents are not intended to be understood by the responder, but they shall be preserved
    /// and returned to the initiator on request.
    pub struct PropertyBag {
        properties: {
            data_type: Array<u16>,
            valid_forms: [LongString]
        },
        code: 0xDC43,
        form: LongStringForm
    }

    /// The name of the object
    ///
    /// In many cases this will overlap with other properties, such as [`ObjectFileName`], and can be
    /// seen as a consistently available, unique, human-readable identifier.
    pub struct Name {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC44,
    }

    /// The application, user, or organization that originally created the binary object
    ///
    /// This property is not intended to identify the person who created the intellectual property
    /// contained within this object; that information is intended to be contained in the [`Artist`].
    pub struct CreatedBy {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC45,
    }

    /// The person or people who originally created this object
    ///
    /// This property differs from the [`CreatedBy`] property in that it always identifies a person.
    ///
    /// This property and the [`CreatedBy`] property may often contain the same value.
    pub struct Artist {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC46,
    }

    /// The date and time when the content in this object was originally created
    ///
    /// The `DateAuthored` and [`DateCreated`] properties may contain the same value.
    pub struct DateAuthored {
        properties: {
            data_type: DateTime,
            data_type_parser: DateTime::parse_ptp_string,
            valid_forms: [DateTime]
        },
        code: 0xDC47,
    }

    /// A human-readable description of the object
    pub struct Description {
        properties: {
            data_type: Array<u16>,
            valid_forms: [LongString]
        },
        code: 0xDC48,
        form: LongStringForm
    }

    /// A URL for this object
    ///
    /// The form fields of this property may contain a Regular Expression limiting this field to valid
    /// HTML addresses, as follows:
    ///
    /// `((http:)?/{0,2}([^/\?#\[\];:&=\+\$,]*\.)+([^/\?#\[\];:&=\+\$,]{2,3}))(/[^<>])*`
    ///
    /// or it may contain a null string to indicate that the value is not used or validated.
    pub struct UrlReference {
        properties: {
            data_type: Array<u16>,
            valid_forms: [RegularExpression]
        },
        code: 0xDC49,
        form: RegularExpressionForm
    }

    /// The language of this object
    ///
    /// If multiple languages are contained in this object, it shall identify the primary language (if any).
    ///
    /// This property may contain either a language code, as defined in ISO-639, such as: “en” or “ja”.
    ///
    /// It may also contain a language-country code, which consists of a language code of two or three
    /// characters as defined in the ISO-639 standard, followed by a hyphen, then followed by a country
    /// code as defined in ISO-3166, such as: “en-US” or “ja-JP”.
    ///
    /// The FORM fields of this property may contain a Regular Expression limiting this field to valid
    /// Language-Locales, such as: `[a-zA-Z]{2,3}(-[a-zA-Z]{2})?`, or it may contain a null string
    /// to indicate that the value is not used or validated.
    pub struct LanguageLocale {
        properties: {
            data_type: PtpString,
            valid_forms: [RegularExpression]
        },
        code: 0xDC4A,
        form: RegularExpressionForm
    }

    /// The copyright information for this object
    pub struct CopyrightInformation {
        properties: {
            data_type: Array<u16>,
            valid_forms: [LongString]
        },
        code: 0xDC4B,
        form: LongStringForm
    }

    /// The source of this object
    ///
    /// In general, this is not intended to identify an individual or organization. For audio files,
    /// this is intended to contain the collection from which the work was retrieved (generally the album name).
    ///
    /// This property may overlap with the [`Artist`] and [`CreatedBy`] properties for some object formats,
    /// and is intended to be implemented only if those two properties are insufficient or inapplicable.
    pub struct Source {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC4C,
    }

    /// The origin location for this object
    ///
    /// If the FORM field contains a null string, it indicates that any human-readable string may
    /// be placed in this field (such as City/Country name).
    pub struct OriginLocation {
        properties: {
            data_type: PtpString,
            valid_forms: [RegularExpression]
        },
        code: 0xDC4D,
        form: RegularExpressionForm
    }

    /// The time and date when this object was added to the device
    ///
    /// This value comes from the internal clock on the device.
    pub struct DateAdded {
        properties: {
            data_type: DateTime,
            data_type_parser: DateTime::parse_ptp_string,
            get_set: GetSet::ReadOnly,
            valid_forms: [DateTime]
        },
        code: 0xDC4E,
    }

    /// Whether this object is intended to be consumed by the device, or has been placed on the device
    /// just for storage
    pub struct NonConsumable {
        properties: {
            data_type: ConsumableStatus,
            valid_forms: [Enumeration]
        },
        code: 0xDC4F,
        form: EnumerationForm<u8>
    }

    /// This object should be able to be understood, but for some reason, cannot be played
    pub struct CorruptOrUnplayable {
        properties: {
            data_type: ConsumableStatus,
            get_set: GetSet::ReadOnly,
            valid_forms: [Enumeration]
        },
        code: 0xDC50,
        form: EnumerationForm<u8>
    }

    /// The unique serial number of the device which originally created the binary object to which
    /// this property applies
    pub struct ProducerSerialNumber {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC51,
    }

    /// The object format of the representative sample for the object, using an [`ObjectFormatCode`]
    pub struct RepresentativeSampleFormat {
        properties: {
            data_type: ObjectFormatCode,
            valid_forms: [Enumeration]
        },
        code: 0xDC81,
    }

    /// The size in bytes of the representative sample for this object
    pub struct RepresentativeSampleSize {
        properties: {
            data_type: u32,
            valid_forms: [Range]
        },
        code: 0xDC82,
        form: RangeForm<u32>
    }

    /// The height of the representative sample for the object in pixels
    pub struct RepresentativeSampleHeight {
        properties: {
            data_type: u32,
            valid_forms: [Range]
        },
        code: 0xDC83,
        form: RangeForm<u32>
    }

    /// The height of the representative sample for the object in pixels
    pub struct RepresentativeSampleWidth {
        properties: {
            data_type: u32,
            valid_forms: [Range]
        },
        code: 0xDC84,
        form: RangeForm<u32>
    }

    /// The duration of the representative sample for the object in milliseconds
    pub struct RepresentativeSampleDuration {
        properties: {
            data_type: u32,
            valid_forms: [Range]
        },
        code: 0xDC85,
        form: RangeForm<u32>
    }

    /// A representative sample of the object
    pub struct RepresentativeSampleData {
        properties: {
            data_type: Array<u8>,
            valid_forms: [ByteArray]
        },
        code: 0xDC86,
        form: ByteArrayForm
    }

    /// The width of the object in pixels
    ///
    /// If this property is [`GetSet::ReadOnly`], it must be calculated by the device based on the
    /// object when requested. If this property is [`GetSet::ReadWrite`], the device may return the
    /// default value ([`u32::MIN`]) when it has not yet extracted the correct value from the object,
    /// and may allow the value to be set in by the initiator.
    pub struct Width {
        properties: {
            data_type: u32,
            valid_forms: [Range]
        },
        code: 0xDC87,
        form: RangeForm<u32>
    }

    /// The height of the object in pixels
    ///
    /// If this property is [`GetSet::ReadOnly`], it must be calculated by the device based on the
    /// object when requested. If this property is [`GetSet::ReadWrite`], the device may return the
    /// default value ([`u32::MIN`]) when it has not yet extracted the correct value from the object,
    /// and may allow the value to be set in by the initiator.
    pub struct Height {
        properties: {
            data_type: u32,
            valid_forms: [Range]
        },
        code: 0xDC88,
        form: RangeForm<u32>
    }

    /// The duration of the object in milliseconds
    ///
    /// If this property is [`GetSet::ReadOnly`], it must be calculated by the device based on the
    /// object when requested. If this property is [`GetSet::ReadWrite`], the device may return the
    /// default value ([`u32::MIN`]) when it has not yet extracted the correct value from the object,
    /// and may allow the value to be set in by the initiator.
    pub struct Duration {
        properties: {
            data_type: u32,
            valid_forms: [Range] // TODO: Range formless?
        },
        code: 0xDC89,
        form: RangeForm<u32>
    }

    /// The value of rating for the object, as set by a user
    ///
    /// This represents a rating of how much this object is appreciated (such as a “star” rating from 1 to 5 stars),
    /// and does not identify a maturity rating (such as R or PG-13).
    ///
    /// The user rating is always exchanged as a value from 1 to 100. If the user rating has not
    /// been set, it shall have a value of 0.
    pub struct Rating {
        properties: {
            data_type: u16,
            valid_forms: [Range]
        },
        code: 0xDC8A,
        form: RangeForm<u16>
    }

    /// The track on which this object is found on its distribution media
    ///
    /// It primarily applies to objects which are also distributed on optical media, such as CDs
    /// and DVDs, and generally is set by the initiator.
    ///
    /// A value of `0x0000` (default value) indicates that it is not in use, so track numbers shall
    /// be 1-based.
    pub struct Track {
        properties: {
            data_type: u16,
            valid_forms: [None]
        },
        code: 0xDC8B,
    }

    /// The genre of this object
    ///
    /// This genre may be any human-readable genre-describing string, and generally must be set by
    /// the initiator.
    pub struct Genre {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC8C,
    }

    /// Credits for this object
    ///
    /// The format of these credits shall be simple text and the value is generally set by the initiator.
    pub struct Credits {
        properties: {
            data_type: Array<u16>,
            valid_forms: [LongString]
        },
        code: 0xDC8D,
        form: LongStringForm
    }

    /// The lyrics or script for this object
    ///
    /// The format of these lyrics shall be simple text and the value is generally set by the initiator.
    pub struct Lyrics {
        properties: {
            data_type: Array<u16>,
            valid_forms: [LongString]
        },
        code: 0xDC8E,
        form: LongStringForm
    }

    /// Additional information to identify a piece of content relative to an online subscription service
    ///
    /// This is generally set by the initiator, and is a specific format.
    ///
    /// This property shall contain a Regular Expression FORM in its Object Property
    /// Description dataset. If no specific subscription service is supported, or there is no
    /// constraint on the subscription identifier value, this regular expression may be a null
    /// string, indicating that any string is supported. Alternatively, the regular expression may
    /// be ".*", indicating that all strings are supported.
    pub struct SubscriptionContentId {
        properties: {
            data_type: PtpString,
            valid_forms: [RegularExpression]
        },
        code: 0xDC8F,
        form: RegularExpressionForm
    }

    /// The person or organization that produced this work
    ///
    /// This primarily applies to audio and video content, and is generally set by the initiator.
    pub struct ProducedBy {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC90,
    }

    /// The number of times this object has been played or viewed
    pub struct UseCount {
        properties: {
            data_type: u32,
            valid_forms: [None]
        },
        code: 0xDC91,
    }

    /// The number of times this object was set up to be played, but manually skipped by the user
    pub struct SkipCount {
        properties: {
            data_type: u32,
            valid_forms: [None]
        },
        code: 0xDC92,
    }

    /// The date and time when this object was last viewed, accessed, or otherwise used, relative to a device's onboard clock
    pub struct LastAccessed {
        properties: {
            data_type: DateTime,
            data_type_parser: DateTime::parse_ptp_string,
            valid_forms: [DateTime]
        },
        code: 0xDC93,
    }

    /// The parental rating assigned to this object
    ///
    /// The purpose of this property is to identify objects that may not be appropriate to be viewed
    /// or heard by minors.
    ///
    /// The contents of this field are intended to be human-readable.
    pub struct ParentalRating {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC94,
    }

    /// A qualifier for a piece of media in a contextual way
    pub struct MetaGenre {
        properties: {
            data_type: MetaGenreForm,
            valid_forms: [Enumeration]
        },
        code: 0xDC95,
        form: EnumerationForm<u16>
    }

    /// The composer of the audio or video content
    ///
    /// It applies primarily to musical works, but can be applied to any created performance.
    ///
    /// This property may often overlap with the [`CreatedBy`] and [`Artist`] properties.
    pub struct Composer {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC96,
    }

    /// An assigned rating for the object
    ///
    /// This rating is not set by the user, but is generated based upon usage statistics (such as use count or skip count), or set by an
    /// external authority.
    ///
    /// The effective rating is always exchanged as a value from 1 to 100. If the effective rating
    /// has not been set, it shall have a value of 0.
    pub struct EffectiveRating {
        properties: {
            data_type: u16,
            valid_forms: [None]
        },
        code: 0xDC97,
    }

    /// A further qualifier for the title, when it is ambiguous or general
    pub struct Subtitle {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC98,
    }
}

/// The possible values for [`Hidden`] properties
#[derive(Copy, Clone, Debug, PartialEq, Eq, deku::DekuRead, deku::DekuWrite)]
#[deku(
    id_type = "u16",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[allow(missing_docs)]
pub enum HiddenStatus {
    /// The object can be shown to all users (e.g. those browsing in a file browser)
    #[deku(id = "0x00")]
    VisibleToAll,
    /// The object should only be shown to technical users (e.g. in debug/developer interfaces)
    #[deku(id = "0x01")]
    HiddenFromNonTechnicalUsers,
    #[deku(id_pat = "t if t & 0x8000 == 0x8000")]
    MtpVendorExtension(u16),
    #[deku(id_pat = "t if t & 0x8000 == 0xC000")]
    MtpDefined(u16),
}

/// The possible values for [`SystemObject`] properties
///
/// This is identical to [`HiddenStatus`]
#[derive(Copy, Clone, Debug, PartialEq, Eq, deku::DekuRead, deku::DekuWrite)]
#[deku(
    id_type = "u16",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[allow(missing_docs)]
pub enum SystemObjectStatus {
    #[deku(id = "0x00")]
    VisibleToAll,
    #[deku(id = "0x01")]
    HiddenFromNonTechnicalUsers,
    #[deku(id_pat = "t if t & 0x8000 == 0x8000")]
    MtpVendorExtension(u16),
    #[deku(id_pat = "t if t & 0x8000 == 0xC000")]
    MtpDefined(u16),
}

/// The possible values for [`NonConsumable`] properties
#[derive(Copy, Clone, Debug, PartialEq, Eq, deku::DekuRead, deku::DekuWrite)]
#[deku(
    id_type = "u8",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[allow(missing_docs)]
#[repr(u8)]
pub enum ConsumableStatus {
    Consumable = 0x00,
    ForStorage = 0x01,
}

/// The possible values for [`MetaGenre`] properties
#[derive(Copy, Clone, Debug, PartialEq, Eq, deku::DekuRead, deku::DekuWrite)]
#[deku(
    id_type = "u16",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[allow(missing_docs)]
#[repr(u16)]
pub enum MetaGenreForm {
    #[deku(id = "0x0000")]
    NotUsed,
    #[deku(id = "0x0001")]
    GenericMusicAudioFile,
    #[deku(id = "0x0011")]
    GenericNonMusicAudioFile,
    #[deku(id = "0x0012")]
    SpokenWordAudioBookFiles,
    #[deku(id = "0x0013")]
    SpokenWordFilesNonAudioBook,
    #[deku(id = "0x0014")]
    SpokenWordNews,
    #[deku(id = "0x0015")]
    SpokenWordTalkShows,
    #[deku(id = "0x0021")]
    GenericVideoFile,
    #[deku(id = "0x0022")]
    NewsVideoFile,
    #[deku(id = "0x0023")]
    MusicVideoFile,
    #[deku(id = "0x0024")]
    HomeVideoFile,
    #[deku(id = "0x0025")]
    FeatureFilmVideoFile,
    #[deku(id = "0x0026")]
    TelevisionShowVideoFile,
    #[deku(id = "0x0027")]
    TrainingEducationalVideoFile,
    #[deku(id = "0x0028")]
    PhotoMontageVideoFile,
    #[deku(id = "0x0030")]
    GenericNonAudioNonVideo,
    #[deku(id = "0x0040")]
    AudioMediacast,
    #[deku(id = "0x0041")]
    VideoMediacast,
    #[deku(id = "0x0042")]
    MixedMediaMediacast,
    #[deku(id_pat = "t if t & 0x8000 == 0")]
    Reserved(u16),
    #[deku(id_pat = "t if t & 0x8000 == 0x8000")]
    VendorExtension(u16),
}
