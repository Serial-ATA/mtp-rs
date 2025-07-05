use crate::communication::Parameter;
use crate::device::properties::{GetSet, RangeForm};
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
pub trait ObjectProperty:
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
    use super::ObjectProperty;

    pub trait Sealed {}

    impl<T: ObjectProperty> Sealed for T {}
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
    P: ObjectProperty,
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

macro_rules! define_object_property_descriptions {
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
		pub enum ObjectPropertyCode {
			$(
			#[deku(id = $code)]
			$name = $code,
			)*
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

		$(
			$(#[$meta])*
			#[derive(Clone, Debug, PartialEq)]
			pub struct $name {
				pub default_value: $datatype,
				pub group_code: u32,
				pub get_set: GetSet,
				pub form: define_object_property_descriptions!(@FORM_TY ($($($form_tt)*)?) | $datatype),
			}

			impl ObjectProperty for $name {
				const CODE: u16 = $code;

				type DataType = $datatype;

				fn get_set(&self) -> GetSet {
					self.get_set
				}
			}

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
					if data_type != <Self as ObjectProperty>::DataType::CODE {
						return Err(deku::DekuError::Assertion(Cow::Owned(format!(
							"Expected datatype code {}, got {data_type}", <Self as ObjectProperty>::DataType::CODE
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

					let form = define_object_property_descriptions!(@FORM_DEFINITION reader ctx ($($($form_tt)*)?) | $datatype);

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, deku::DekuRead)]
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
            valid_forms: [None]
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
            valid_forms: [Enumeration]
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
            get_set: GetSet::ReadOnly,
            valid_forms: [None]
        },
        code: 0xDC07,
        form: enum ObjectFileNameForm {
            #[deku(id = "0x00")]
            None(PtpString),
            #[deku(id = "0x01")]
            RegularExpression(PtpString),
        }
    }

    /// The date and time when the object was created
    pub struct DateCreated {
        properties: {
            data_type: PtpString,
            valid_forms: [DateTime]
        },
        code: 0xDC08,
        form: DateTime
    }

    /// The date and time when the object was last altered
    pub struct DateModified {
        properties: {
            data_type: PtpString,
            valid_forms: [DateTime]
        },
        code: 0xDC09,
        form: DateTime
    }

    /// A list of keywords associated with the object, separated by ' '.
    pub struct Keywords {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC0A,
    }

    /// The object handle of the parent of this object, if it exists in a hierarchy
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

    /// Objects formats allowed in this folder
    ///
    /// If there is no restriction, the returned array will be empty.
    pub struct Hidden {
        properties: {
            data_type: u16,
            valid_forms: [Enumeration]
        },
        code: 0xDC0D,
        form: enum HiddenForm {
            #[deku(id = "0x00")]
            VisibleToAll,
            #[deku(id = "0x01")]
            HiddenFromNonTechnicalUsers,
            #[deku(id_pat = "t if t & 0x8000 == 0x8000")]
            MtpVendorExtension(u16),
            #[deku(id_pat = "t if t & 0x8000 == 0xC000")]
            MtpDefined(u16)
        }
    }

    /// Whether an object is a system file, and is required for property functioning of a device
    pub struct SystemObject {
        properties: {
            data_type: u16,
            valid_forms: [Enumeration]
        },
        code: 0xDC0E,
        form: enum SystemObjectForm {
            #[deku(id = "0x00")]
            VisibleToAll,
            #[deku(id = "0x01")]
            HiddenFromNonTechnicalUsers,
            #[deku(id_pat = "t if t & 0x8000 == 0x8000")]
            MtpVendorExtension(u16),
            #[deku(id_pat = "t if t & 0x8000 == 0xC000")]
            MtpDefined(u16)
        }
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
        form: PtpString // TODO
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
            data_type: PtpString,
            valid_forms: [DateTime]
        },
        code: 0xDC47,
        form: DateTime
    }

    /// A human-readable description of the object
    pub struct Description {
        properties: {
            data_type: Array<u16>,
            valid_forms: [LongString]
        },
        code: 0xDC48,
        form: PtpString // TODO
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
        form: PtpString // TODO
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
        form: PtpString // TODO
    }

    /// The copyright information for this object
    pub struct CopyrightInformation {
        properties: {
            data_type: Array<u16>,
            valid_forms: [LongString]
        },
        code: 0xDC4B,
        form: PtpString // TODO
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
        form: PtpString // TODO
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
        form: PtpString // TODO
    }

    /// The time and date when this object was added to the device
    ///
    /// This value comes from the internal clock on the device.
    pub struct DateAdded {
        properties: {
            data_type: PtpString,
            get_set: GetSet::ReadOnly,
            valid_forms: [DateTime]
        },
        code: 0xDC4E,
        form: DateTime
    }

    /// Whether this object is intended to be consumed by the device, or has been placed on the device
    /// just for storage
    pub struct NonConsumable {
        properties: {
            data_type: u8,
            valid_forms: [Enumeration]
        },
        code: 0xDC4F,
        form: enum ConsumptionStatus {
            Consumable = 0x00,
            ForStorage = 0x01,
        }
    }

    /// This object should be able to be understood, but for some reason, cannot be played
    pub struct CorruptOrUnplayable {
        properties: {
            data_type: u8,
            get_set: GetSet::ReadOnly,
            valid_forms: [Enumeration]
        },
        code: 0xDC50,
        form: enum CorruptOrUnplayableStatus {
            No = 0x00,
            Yes = 0x01,
        }
    }

    /// The unique serial number of the device which originally created the binary object to which
    /// this property applies
    pub struct ProducerSerialNumber {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0xDC51,
        form: PtpString // TODO
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
        form: u32 // TODO
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
        form: PtpString // TODO
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
        form: PtpString // TODO
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
        form: PtpString // TODO
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
        form: PtpString
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
        form: PtpString
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
            data_type: PtpString,
            valid_forms: [DateTime]
        },
        code: 0xDC93,
        form: DateTime
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
        form: PtpString // TODO
    }

    /// A qualifier for a piece of media in a contextual way
    pub struct MetaGenre {
        properties: {
            data_type: u16,
            valid_forms: [Enumeration]
        },
        code: 0xDC95,
        form: enum MetaGenreForm {
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
        form: PtpString // TODO
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
        form: PtpString // TODO
    }
}
