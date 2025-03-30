use crate::communication::Parameter;
use crate::device::property_describing::GetSet;
use crate::object::types::{
	Array, ArrayEncodable, DateTime, ObjectFormatCode, ObjectHandle, PtpString,
};

use alloc::borrow::Cow;
use alloc::format;

use deku::DekuReader;
use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek};

/// Marker trait for object properties
pub trait ObjectProperty:
	sealed::Sealed + Eq + core::fmt::Debug + Clone + for<'a> DekuReader<'a>
{
	/// The raw datacode for this property
	const CODE: u16;
}

mod sealed {
	use super::ObjectProperty;

	pub trait Sealed {}

	impl<T: ObjectProperty> Sealed for T {}
}

macro_rules! define_object_property_descriptions {
	(
		$(
			$(#[$meta:meta])*
			pub struct $name:ident {
				properties: {
					data_type: $datatype:ty,
					get_set: $get_set:expr,
					valid_forms: [$($form:ident),* $(,)?]
				},
				code: $code:literal,
				form: $($form_tt:tt)*
			}
		)*
	) => {
		#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, deku::DekuRead, deku::DekuWrite)]
		#[repr(u16)]
		#[deku(
			id_type = "u16",
			id_endian = "little",
			ctx = "_endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Little"
		)]
		pub enum ObjectPropertyCode {
			$(
			#[deku(id = $code)]
			$name = $code,
			)*
		}

		impl ArrayEncodable for ObjectPropertyCode {}

		impl From<ObjectPropertyCode> for Parameter {
			fn from(code: ObjectPropertyCode) -> Self {
				Parameter::new(code as u32)
			}
		}

		$(
			$(#[$meta])*
			#[derive(Clone, Debug, PartialEq, Eq)]
			pub struct $name {
				default_value: $datatype,
				group_code: u32,
				form: define_object_property_descriptions!(@FORM_TY $($form_tt)*),
			}

			impl ObjectProperty for $name {
				const CODE: u16 = $code;
			}

			define_object_property_descriptions!(@FORM_ENUM $($form_tt)*);

			impl $name {
				const VALID_FORMS: &'static [FormType] = &[$(FormType::$form),*];
			}

			impl DekuReader<'_, ()> for $name {
				#[inline]
				fn from_reader_with_ctx<R: Read + Seek>(reader: &mut ::deku::reader::Reader<R>, _: ()) -> core::result::Result<Self, ::deku::DekuError> {
					let code = u16::from_reader_with_ctx(reader, Endian::Little)?;
					if code != Self::CODE {
						return Err(deku::DekuError::Assertion(Cow::Owned(format!(
							"Expected property code {}, got {code}", Self::CODE
						))))
					}

					// TODO: Verify
					let _data_type = u16::from_reader_with_ctx(reader, Endian::Little)?;
					let get_set = GetSet::from_reader_with_ctx(reader, ())?;
					if get_set != $get_set {
						return Err(deku::DekuError::Assertion(Cow::Owned(format!(
							"Expected get_set to be {:?}, got {get_set:?}", $get_set
						))))
					}

					let default_value = <$datatype>::from_reader_with_ctx(reader, Endian::Little)?;
					let group_code = u32::from_reader_with_ctx(reader, Endian::Little)?;
					let form_flag = FormType::from_reader_with_ctx(reader, ())?;
					if !Self::VALID_FORMS.contains(&form_flag) {
						return Err(deku::DekuError::Assertion(Cow::Owned(format!(
							"Expected form flag to be one of {:?}, got {form_flag:?}",
							Self::VALID_FORMS
						))))
					}

					let form = define_object_property_descriptions!(@FORM_DEFINITION reader $($form_tt)*);

					Ok(Self {
						default_value,
						group_code,
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
			id_endian = "little",
			ctx = "_endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Little"
		)]
		pub enum $enum_name {
			$($body)*
		}
	};

	(@FORM_ENUM $_ty:ty) => {};

	(@FORM_DEFINITION $reader:ident enum $enum_name:ident {
		$($_tt:tt)*
	}) => {{
		let form: $enum_name = <$enum_name>::from_reader_with_ctx($reader, ())?;
		form
	}};

	(@FORM_DEFINITION $reader:ident $ty:ty) => {{
		let form: $ty = <$ty>::from_reader_with_ctx($reader, ())?;
		form
	}};

	(@FORM_TY enum $enum_name:ident {
		$($_tt:tt)*
	}) => { $enum_name };
	(@FORM_TY $ty:ty) => { $ty };
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, deku::DekuRead)]
#[deku(id_type = "u16", id_endian = "little")]
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
	pub struct StorageId {
		properties: {
			data_type: u32,
			get_set: GetSet::ReadOnly,
			valid_forms: [None]
		},
		code: 0xDC01,
		form: u32
	}

	/// The object format code describes this object
	///
	/// This value is also available on the [`ObjectInfo`].
	pub struct ObjectFormat {
		properties: {
			data_type: u16,
			get_set: GetSet::ReadOnly,
			valid_forms: [None]
		},
		code: 0xDC02,
		form: u16
	}

	/// The write-protection status of the binary component of the object
	pub struct ProtectionStatus {
		properties: {
			data_type: crate::object::info::ProtectionStatus,
			get_set: GetSet::ReadOnly,
			valid_forms: [None]
		},
		code: 0xDC03,
		form: crate::object::info::ProtectionStatus
	}

	/// The size of the binary component of the object, in bytes
	pub struct ObjectSize {
		properties: {
			data_type: u64,
			get_set: GetSet::ReadOnly,
			valid_forms: [None]
		},
		code: 0xDC04,
		form: u64
	}

	/// The [`AssociationType`] of the object
	///
	/// [`AssociationType`]: crate::object::types::AssociationType
	pub struct AssociationType {
		properties: {
			data_type: crate::object::types::AssociationType,
			get_set: GetSet::ReadOnly, // TODO: device-defined
			valid_forms: [Enumeration]
		},
		code: 0xDC05,
		form: crate::object::types::AssociationType
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
			get_set: GetSet::ReadOnly, // TODO: device-defined
			valid_forms: [Enumeration]
		},
		code: 0xDC06,
		form: u32
	}

	/// The write-protection status of the binary component of the object
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
			get_set: GetSet::ReadOnly, // TODO: device-defined
			valid_forms: [DateTime]
		},
		code: 0xDC08,
		form: DateTime
	}

	/// The date and time when the object was last altered
	pub struct DateModified {
		properties: {
			data_type: PtpString,
			get_set: GetSet::ReadOnly, // TODO: device-defined
			valid_forms: [DateTime]
		},
		code: 0xDC09,
		form: DateTime
	}

	/// A list of keywords associated with the object, separated by ' '.
	pub struct Keywords {
		properties: {
			data_type: PtpString,
			get_set: GetSet::ReadOnly, // TODO: device-defined
			valid_forms: [None]
		},
		code: 0xDC0A,
		form: PtpString
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
		form: ObjectHandle
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
		form: Array<ObjectFormatCode>
	}

	/// Objects formats allowed in this folder
	///
	/// If there is no restriction, the returned array will be empty.
	pub struct Hidden {
		properties: {
			data_type: u16,
			get_set: GetSet::ReadOnly, // TODO: device-defined
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
			get_set: GetSet::ReadOnly, // TODO: device-defined
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
		form: u128
	}

	/// An identifier for retaining state between sessions, determined by the initiator
	pub struct SyncId {
		properties: {
			data_type: PtpString,
			get_set: GetSet::ReadOnly, // TODO: device-defined
			valid_forms: [None]
		},
		code: 0xDC42,
		form: PtpString
	}

	/// An XML document specifying object properties
	///
	/// The contents are not intended to be understood by the responder, but they shall be preserved
	/// and returned to the initiator on request.
	pub struct PropertyBag {
		properties: {
			data_type: Array<u16>,
			get_set: GetSet::ReadOnly, // TODO: device-defined
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
			get_set: GetSet::ReadOnly, // TODO: device-defined
			valid_forms: [None]
		},
		code: 0xDC44,
		form: PtpString
	}

	/// The application, user, or organization that originally created the binary object
	///
	/// This property is not intended to identify the person who created the intellectual property
	/// contained within this object; that information is intended to be contained in the [`Artist`].
	pub struct CreatedBy {
		properties: {
			data_type: PtpString,
			get_set: GetSet::ReadOnly, // TODO: device-defined
			valid_forms: [None]
		},
		code: 0xDC45,
		form: PtpString
	}

	/// The person or people who originally created this object
	///
	/// This property differs from the [`CreatedBy`] property in that it always identifies a person.
	///
	/// This property and the [`CreatedBy`] property may often contain the same value.
	pub struct Artist {
		properties: {
			data_type: PtpString,
			get_set: GetSet::ReadOnly, // TODO: device-defined
			valid_forms: [None]
		},
		code: 0xDC46,
		form: PtpString
	}

	/// The date and time when the content in this object was originally created
	///
	/// The `DateAuthored` and [`DateCreated`] properties may contain the same value.
	pub struct DateAuthored {
		properties: {
			data_type: PtpString,
			get_set: GetSet::ReadOnly, // TODO: device-defined
			valid_forms: [DateTime]
		},
		code: 0xDC47,
		form: DateTime
	}

	/// A human-readable description of the object
	pub struct Description {
		properties: {
			data_type: Array<u16>,
			get_set: GetSet::ReadOnly, // TODO: device-defined
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
			get_set: GetSet::ReadOnly, // TODO: device-defined
			valid_forms: [RegularExpression]
		},
		code: 0xDC49,
		form: PtpString // TODO
	}
}
