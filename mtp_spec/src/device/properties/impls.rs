use crate::communication::Parameter;
use crate::device::properties::{EnumerationForm, Form, FormType, GetSet, RangeForm};
use crate::object::types::{Array, ArrayEncodable, ObjectHandle, PropertyDataType, PtpString};
use crate::property::Property;

use alloc::borrow::Cow;
use alloc::format;

use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek};
use deku::{DekuRead, DekuReader, DekuWrite};

/// Marker trait for object properties
pub trait DeviceProperty: Property {}

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
		/// The `PropertyCode` of a [`DeviceProperty`]
		#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, deku::DekuRead, deku::DekuWrite)]
		#[repr(u16)]
		#[deku(
			id_type = "u16",
			id_endian = "endian",
			ctx = "endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Big"
		)]
		#[allow(missing_docs)]
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
				/// The device's assigned default for this property
				pub default_value: $datatype,
				/// The retrieval group this property belongs to
				pub group_code: u32,
				/// The read-only status of the property
				pub get_set: GetSet,
				/// Device-defined constraints on the data. See [`forms`]
				///
				/// [`forms`]: https://docs.rs/mtp_spec/latest/mtp_spec/object/types/properties/index.html#forms
				pub form: define_device_properties!(@FORM_TY ($($($form_tt)*)?) | $datatype),
			}

			impl crate::property::sealed::Sealed for $name {}

			impl Property for $name {
				const CODE: u16 = $code;

				type DataType = $datatype;

				fn get_set(&self) -> GetSet {
					self.get_set
				}
			}

			impl DeviceProperty for $name {}

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

	(@FORM_ENUM $(#[$meta:meta])* enum $enum_name:ident {
		$($body:tt)*
	}) => {
		#[derive(Clone, Debug, PartialEq, Eq, Hash, deku::DekuRead)]
		#[deku(
			id_endian = "endian",
			ctx = "endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Big"
		)]
		$(#[$meta])*
		pub enum $enum_name {
			$($body)*
		}
	};

	(@FORM_ENUM $_ty:ty) => {};
	(@FORM_ENUM) => {};

	(@FORM_DEFINITION $reader:ident $ctx:ident ($(#[$_meta:meta])* enum $enum_name:ident {
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

	(@FORM_TY ($(#[$_meta:meta])* enum $enum_name:ident {
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
        form: crate::device::info::FunctionalMode
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

    /// How the device weights different color channels
    pub struct WhiteBalance {
        properties: {
            data_type: u16,
            valid_forms: [Enumeration]
        },
        code: 0x5005,
        form:
        /// Possible values for [`WhiteBalance`] properties
        #[repr(u16)]
        #[deku(id_type = "u16")]
        #[allow(missing_docs)]
        enum WhiteBalanceValue {
            #[deku(id = "0x0000")]
            Undefined = 0x0000,
            /// The white balance is set directly by using the [`RgbGain`] property, and is static until changed.
            #[deku(id = "0x0001")]
            Manual = 0x0001,
            /// The device attempts to set the white balance using some kind of automatic mechanism.
            #[deku(id = "0x0002")]
            Automatic = 0x0002,
            /// The user must press the capture button while pointing the device at a white field
            #[deku(id = "0x0003")]
            OnePushAutomatic = 0x0003,
            /// The device attempts to set the white balance to a value that is appropriate for use in daylight conditions.
            #[deku(id = "0x0004")]
            Daylight = 0x0004,
            /// The device attempts to set the white balance to a value that is appropriate for use in conditions with a florescent light source.
            #[deku(id = "0x0005")]
            Florescent = 0x0005,
            /// The device attempts to set the white balance to a value that is appropriate for use in conditions with a tungsten light source.
            #[deku(id = "0x0006")]
            Tungsten = 0x0006,
            /// The device attempts to set the white balance to a value that is appropriate for flash conditions.
            #[deku(id = "0x0007")]
            Flash = 0x0007,
            /// Some vendor-specific value
            #[deku(id_pat = "o if (0x8000_u16..=0xBFFF_u16).contains(&o)")]
            VendorSpecific(u16),
            /// An invalid, reserved value
            #[deku(id_pat = "t if (0x0000_u16..=0x7FFF_u16).contains(t) || (0xC000_u16..=0xFFFF_u16).contains(t)")]
            Reserved,
        }
    }

    /// The current RGB gain setting of the responder
    ///
    /// The value is a [`PtpString`] in the form: `"R:G:B"` where each segment is a `u16`.
    ///
    /// An example value would be: `"4:2:3"` for a gain value of 4 for red, 2 for green, and 3 for blue.
    ///
    /// This can be represented in both [`RangeForm`] and [`EnumerationForm`]
    ///
    /// Examples:
    /// * [`RangeForm`]
    ///   * A minimum of `"1:1:1"`, and a maximum of `"65535:65535:65535"`, with a step of `"1:1:1"`
    /// * [`EnumerationForm`]
    ///   * `values` will be a list of all possible gain values
    pub struct RgbGain {
        properties: {
            data_type: PtpString,
            valid_forms: [Range, Enumeration]
        },
        code: 0x5006,
        form: Form<PtpString>
    }

    /// The aperture setting of the lens, scaled by 100
    ///
    /// Setting this property may cause other properties (such as [`ExposureTime`] and [`ExposureIndex`]) to change.
    pub struct FNumber {
        properties: {
            data_type: u16,
            valid_forms: [Enumeration]
        },
        code: 0x5007,
        form: EnumerationForm<u16>
    }

    /// The 35 mm equivalent focal length in millimeters multiplied by 100
    pub struct FocalLength {
        properties: {
            data_type: u32,
            valid_forms: [Range, Enumeration]
        },
        code: 0x5008,
        form: Form<u32>
    }

    /// The focus distance in millimeters
    ///
    /// A value of [`u16::MAX`] indicates a setting greater than 655 meters.
    pub struct FocusDistance {
        properties: {
            data_type: u16,
            valid_forms: [Range, Enumeration]
        },
        code: 0x5009,
        form: Form<u16>
    }

    /// The current focusing mode for image capture
    pub struct FocusMode {
        properties: {
            data_type: u16,
            valid_forms: [Enumeration]
        },
        code: 0x500A,
        form:
        /// Possible values for [`FocusMode`] properties
        #[repr(u16)]
        #[deku(id_type = "u16")]
        #[allow(missing_docs)]
        enum FocusModeForm {
            #[deku(id = "0x0000")]
            Undefined = 0x0000,
            #[deku(id = "0x0001")]
            Manual = 0x0001,
            #[deku(id = "0x0002")]
            Automatic = 0x0002,
            #[deku(id = "0x0003")]
            AutomaticMacro = 0x0003,
            #[deku(id_pat = "o if (0x8000_u16..=0xBFFF_u16).contains(&o)")]
            VendorSpecific(u16),
            #[deku(id_pat = "t if (0x0000_u16..=0x7FFF_u16).contains(t) || (0xC000_u16..=0xFFFF_u16).contains(t)")]
            Reserved,
        }
    }

    /// The current exposure metering mode for image capture
    pub struct ExposureMeteringMode {
        properties: {
            data_type: u16,
            valid_forms: [Enumeration]
        },
        code: 0x500B,
        form:
        /// Possible values for [`ExposureMeteringMode`] properties
        #[repr(u16)]
        #[deku(id_type = "u16")]
        #[allow(missing_docs)]
        enum ExposureMeteringModeForm {
            #[deku(id = "0x0000")]
            Undefined = 0x0000,
            #[deku(id = "0x0001")]
            Average = 0x0001,
            #[deku(id = "0x0002")]
            CenterWeightedAverage = 0x0002,
            #[deku(id = "0x0003")]
            MultiSpot = 0x0003,
            #[deku(id = "0x0004")]
            CenterSpot = 0x0004,
            #[deku(id_pat = "o if (0x8000_u16..=0xBFFF_u16).contains(&o)")]
            VendorSpecific(u16),
            #[deku(id_pat = "t if (0x0000_u16..=0x7FFF_u16).contains(t) || (0xC000_u16..=0xFFFF_u16).contains(t)")]
            Reserved,
        }
    }

    /// The current flash mode for image capture
    pub struct FlashMode {
        properties: {
            data_type: u16,
            valid_forms: [Enumeration]
        },
        code: 0x500C,
        form:
        /// Possible values for [`FlashMode`] properties
        #[repr(u16)]
        #[deku(id_type = "u16")]
        #[allow(missing_docs)]
        enum FlashModeForm {
            #[deku(id = "0x0000")]
            Undefined = 0x0000,
            #[deku(id = "0x0001")]
            AutoFlash = 0x0001,
            #[deku(id = "0x0002")]
            FlashOff = 0x0002,
            #[deku(id = "0x0003")]
            FillFlash = 0x0003,
            #[deku(id = "0x0004")]
            RedEyeAuto = 0x0004,
            #[deku(id = "0x0005")]
            RedEyeFill = 0x0005,
            #[deku(id = "0x0006")]
            ExternalSync = 0x0006,
            #[deku(id_pat = "o if (0x8000_u16..=0xBFFF_u16).contains(&o)")]
            VendorSpecific(u16),
            #[deku(id_pat = "t if (0x0000_u16..=0x7FFF_u16).contains(t) || (0xC000_u16..=0xFFFF_u16).contains(t)")]
            Reserved,
        }
    }

    /// The current shutter speed of the device in seconds, scaled by 10,000
    pub struct ExposureTime {
        properties: {
            data_type: u32,
            valid_forms: [Range, Enumeration]
        },
        code: 0x500D,
        form: Form<u32>
    }

    /// The exposure program mode settings of the device
    ///
    /// This corresponds to the "Exposure Program" tag within EXIF metadata.
    pub struct ExposureProgramMode {
        properties: {
            data_type: u16,
            valid_forms: [Enumeration]
        },
        code: 0x500E,
        form:
        /// Possible values for [`ExposureProgramMode`] properties
        #[repr(u16)]
        #[deku(id_type = "u16")]
        #[allow(missing_docs)]
        enum ExposureProgramModeForm {
            #[deku(id = "0x0000")]
            Undefined = 0x0000,
            #[deku(id = "0x0001")]
            Manual = 0x0001,
            #[deku(id = "0x0002")]
            Automatic = 0x0002,
            #[deku(id = "0x0003")]
            AperturePriority = 0x0003,
            #[deku(id = "0x0004")]
            ShutterPriority = 0x0004,
            #[deku(id = "0x0005")]
            ProgramCreative = 0x0005,
            #[deku(id = "0x0006")]
            ProgramAction = 0x0006,
            #[deku(id = "0x0007")]
            Portrait = 0x0007,
            #[deku(id_pat = "o if (0x8000_u16..=0xBFFF_u16).contains(&o)")]
            VendorSpecific(u16),
            #[deku(id_pat = "t if (0x0000_u16..=0x7FFF_u16).contains(t) || (0xC000_u16..=0xFFFF_u16).contains(t)")]
            Reserved,
        }
    }

    /// The film speed settings of the device
    ///
    /// The settings of this property correspond to the ISO designations (ASA/DIN).
    ///
    /// A value of [`u16::MAX`] corresponds to automatic ISO setting.
    pub struct ExposureIndex {
        properties: {
            data_type: u16,
            valid_forms: [Range, Enumeration]
        },
        code: 0x500F,
        form: Form<u16>
    }

    /// The set point of the device's auto exposure control
    ///
    /// This is a scaling factor, representing "stops" scaled by a factor of 1000.
    ///
    /// For example, a value of `0` will not change the factor set auto exposure level, while a value of
    /// `2000` indicates two stops of additional exposure.
    ///
    /// Values are represented in APEX (Additive system of Photographic Exposure) units
    pub struct ExposureBiasCompensation {
        properties: {
            data_type: i16,
            valid_forms: [Range, Enumeration]
        },
        code: 0x5010,
        form: Form<i16>
    }

    /// The current date and time settings of the device
    pub struct DateTime {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0x5011,
        form: crate::object::types::DateTime
    }

    /// The millisecond delay between triggering image capture and the actual data capture
    pub struct CaptureDelay {
        properties: {
            data_type: u32,
            valid_forms: [Range, Enumeration]
        },
        code: 0x5012,
        form: Form<u32>
    }

    /// The type of still capture which will be performed
    pub struct StillCaptureMode {
        properties: {
            data_type: u16,
            valid_forms: [Enumeration]
        },
        code: 0x5013,
        form:
        /// Possible values for [`StillCaptureMode`] properties
        #[repr(u16)]
        #[deku(id_type = "u16")]
        #[allow(missing_docs)]
        enum StillCaptureModeForm {
            #[deku(id = "0x0000")]
            Undefined = 0x0000,
            #[deku(id = "0x0001")]
            Normal = 0x0001,
            #[deku(id = "0x0002")]
            Burst = 0x0002,
            #[deku(id = "0x0003")]
            Timelapse = 0x0003,
            #[deku(id_pat = "o if (0x8000_u16..=0xBFFF_u16).contains(&o)")]
            VendorSpecific(u16),
            #[deku(id_pat = "t if (0x0000_u16..=0x7FFF_u16).contains(t) || (0xC000_u16..=0xFFFF_u16).contains(t)")]
            Reserved,
        }
    }

    /// The perceived contrast of images captured with this device
    pub struct Contrast {
        properties: {
            data_type: u8,
            valid_forms: [Range, Enumeration]
        },
        code: 0x5014,
        form: Form<u8>
    }

    /// The perceived sharpness of images captured with this device
    pub struct Sharpness {
        properties: {
            data_type: u8,
            valid_forms: [Range, Enumeration]
        },
        code: 0x5015,
        form: Form<u8>
    }

    /// The effective digital zoom which will be applied to images, scaled by a factor of 10
    ///
    /// Examples:
    ///
    /// * A value of `10` means no digital zoom is applied.
    /// * A value of `20` means a zoom by a factor of 2 (2x)
    pub struct DigitalZoom {
        properties: {
            data_type: u8,
            valid_forms: [Range, Enumeration]
        },
        code: 0x5016,
        form: Form<u8>
    }

    /// Special image acquisition modes
    pub struct EffectMode {
        properties: {
            data_type: u16,
            valid_forms: [Enumeration]
        },
        code: 0x5017,
        form:
        /// Possible values for [`EffectMode`] properties
        #[repr(u16)]
        #[deku(id_type = "u16")]
        #[allow(missing_docs)]
        enum EffectModeForm {
            #[deku(id = "0x0000")]
            Undefined = 0x0000,
            /// Color
            #[deku(id = "0x0001")]
            Standard = 0x0001,
            #[deku(id = "0x0002")]
            BlackAndWhite = 0x0002,
            #[deku(id = "0x0003")]
            Sepia = 0x0003,
            #[deku(id_pat = "o if (0x8000_u16..=0xBFFF_u16).contains(&o)")]
            VendorSpecific(u16),
            #[deku(id_pat = "t if (0x0000_u16..=0x7FFF_u16).contains(t) || (0xC000_u16..=0xFFFF_u16).contains(t)")]
            Reserved,
        }
    }

    /// The number of images that will be captured upon a burst capture operation
    pub struct BurstNumber {
        properties: {
            data_type: u16,
            valid_forms: [Range, Enumeration]
        },
        code: 0x5018,
        form: Form<u8>
    }

    /// The time delay in milliseconds between image captures in a burst capture operation
    pub struct BurstInterval {
        properties: {
            data_type: u16,
            valid_forms: [Range, Enumeration]
        },
        code: 0x5019,
        form: Form<u8>
    }

    /// The number of images which will be captured when a timelapse capture starts
    pub struct TimelapseNumber {
        properties: {
            data_type: u16,
            valid_forms: [Range, Enumeration]
        },
        code: 0x501A,
        form: Form<u8>
    }

    /// The time delay in milliseconds between image captures in a timelapse capture operation
    pub struct TimelapseInterval {
        properties: {
            data_type: u16,
            valid_forms: [Range, Enumeration]
        },
        code: 0x501B,
        form: Form<u8>
    }

    /// The automatic-focus mechanism currently in use by the device
    pub struct FocusMeteringMode {
        properties: {
            data_type: u32,
            valid_forms: [Enumeration]
        },
        code: 0x501C,
        form:
        /// Possible values for [`FocusMeteringMode`] properties
        #[repr(u32)]
        #[deku(id_type = "u16")]
        #[allow(missing_docs)]
        enum FocusMeteringModeForm {
            #[deku(id = "0x0000")]
            Undefined = 0x0000,
            #[deku(id = "0x0001")]
            CenterSpot = 0x0001,
            #[deku(id = "0x0002")]
            MultiSpot = 0x0002,
            #[deku(id_pat = "o if (0x8000_u16..=0xBFFF_u16).contains(&o)")]
            VendorSpecific(u16),
            #[deku(id_pat = "t if (0x0000_u16..=0x7FFF_u16).contains(t) || (0xC000_u16..=0xFFFF_u16).contains(t)")]
            Reserved,
        }
    }

    /// A URL the initiator may use the upload objects acquired from the device
    pub struct UploadUrl {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0x501D,
        form: ()
    }

    /// The name of the device owner/operator
    ///
    /// This is the value used to populate the [`Artist`] object property.
    ///
    /// [`Artist`]: crate::object::types::properties::Artist
    pub struct Artist {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0x501E,
        form: ()
    }

    /// A copyright notification
    ///
    /// This is the value used to populate the [`CopyrightInformation`] object property.
    ///
    /// [`CopyrightInformation`]: crate::object::types::properties::CopyrightInformation
    pub struct CopyrightInfo {
        properties: {
            data_type: PtpString,
            valid_forms: [None]
        },
        code: 0x501F,
        form: ()
    }

    /// A human-readable description of a synchronization partner for a device
    pub struct SynchronizationPartner {
        properties: {
            data_type: PtpString,
            get_set: GetSet::ReadWrite,
            valid_forms: [None]
        },
        code: 0xD401,
        form: ()
    }

    /// A human-readable description of the device
    pub struct DeviceFriendlyName {
        properties: {
            data_type: PtpString,
            get_set: GetSet::ReadWrite,
            valid_forms: [None]
        },
        code: 0xD402,
        form: ()
    }

    /// The current volume of the device
    ///
    /// A value of `0` indicates that the device is muted.
    pub struct Volume {
        properties: {
            data_type: u32,
            get_set: GetSet::ReadWrite,
            valid_forms: [Range]
        },
        code: 0xD403,
        form: RangeForm<u32>
    }

    /// Whether the device is indicating its supported production and consumption object formats in order of preference
    pub struct SupportedFormatsOrdered {
        properties: {
            data_type: u8,
            get_set: GetSet::ReadOnly,
            valid_forms: [Enumeration]
        },
        code: 0xD404,
        form:
        /// Possible values for [`SupportedFormatsOrdered`] properties
        #[repr(u8)]
        #[deku(id_type = "u8")]
        #[allow(missing_docs)]
        enum SupportedFormatsOrderedForm {
            #[deku(id = "0x00")]
            Unordered = 0x00,
            #[deku(id = "0x01")]
            Ordered = 0x01,
            #[deku(id_pat = "o if (0x80_u8..=0xFF_u8).contains(&o)")]
            VendorSpecific(u8),
            #[deku(id_pat = "_")]
            Reserved,
        }
    }

    /// The `ICO` icon object that represents the device
    pub struct DeviceIcon {
        properties: {
            data_type: Array<u8>,
            valid_forms: [None]
        },
        code: 0xD405,
        form: ()
    }

    /// The current playback speed, identified linearly in thousandths
    ///
    /// Examples:
    ///
    /// * A value of `1000` indicates full speed
    /// * A value of `500` indicates half speed
    /// * A value of `-1000` indicates reverse at full speed
    /// * A value of `0` indicates the device is paused
    pub struct PlaybackRate {
        properties: {
            data_type: i32,
            get_set: GetSet::ReadWrite,
            valid_forms: [Enumeration]
        },
        code: 0xD410,
        form: EnumerationForm<i32>
    }

    /// The [`ObjectHandle`] of the object (or container, see [`PlaybackContainerIndex`]) currently being played on the device
    ///
    /// A value of [`ObjectHandle::NONE`] indicates that the device is stopped.
    pub struct PlaybackObject {
        properties: {
            data_type: ObjectHandle,
            get_set: GetSet::ReadWrite,
            valid_forms: [None]
        },
        code: 0xD411,
        form: ()
    }

    /// The [`ObjectHandle`] of the object within the current playback container
    ///
    /// [`PlaybackObject`] may be a container, so this narrows down which object in particular is being
    /// played.
    pub struct PlaybackContainerIndex {
        properties: {
            data_type: ObjectHandle,
            get_set: GetSet::ReadWrite,
            valid_forms: [None]
        },
        code: 0xD412,
        form: ()
    }

    /// The time offset of the object currently being played, in milliseconds
    ///
    /// NOTE: As this property changes frequently, it will not typically trigger [`Event::DevicePropChanged`] events.
    ///
    /// [`Event::DevicePropChanged`]: crate::communication::event::Event::DevicePropChanged
    pub struct PlaybackPosition {
        properties: {
            data_type: u32,
            get_set: GetSet::ReadWrite,
            valid_forms: [None]
        },
        code: 0xD413,
        form: ()
    }

    /// Version information of the session initiator, formatted as an [HTTP user agent]
    ///
    /// [HTTP user agent]: https://en.wikipedia.org/wiki/User-Agent_header
    pub struct SessionInitiatorVersionInfo {
        properties: {
            data_type: PtpString,
            get_set: GetSet::ReadWrite,
            valid_forms: [None]
        },
        code: 0xD406,
        form: ()
    }

    /// The device type of the responder, as perceived by the user
    pub struct PerceivedDeviceType {
        properties: {
            data_type: PerceivedDeviceTypeValue,
            get_set: GetSet::ReadOnly,
            valid_forms: [None]
        },
        code: 0xD407,
        form: ()
    }
}

/// The possible values for [`PerceivedDeviceType`] properties
#[derive(DekuRead, DekuWrite, Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
#[deku(
    id_type = "u32",
    id_endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
#[allow(missing_docs)]
pub enum PerceivedDeviceTypeValue {
    #[deku(id = "0x00000000")]
    Generic = 0x00000000,
    /// Still Image/Video Camera
    #[deku(id = "0x00000001")]
    StillCamera = 0x00000001,
    /// Media (Audio/Video) Player
    #[deku(id = "0x00000002")]
    MediaPlayer = 0x00000002,
    #[deku(id = "0x00000003")]
    MobileHandset = 0x00000003,
    #[deku(id = "0x00000004")]
    VideoPlayer = 0x00000004,
    /// Personal Information Manager / Personal Digital Assistant
    #[deku(id = "0x00000005")]
    PersonalInformationManager = 0x00000005,
    #[deku(id = "0x00000006")]
    AudioRecorder = 0x00000006,
    #[deku(id_pat = "o if o & (1 << 15) > 0")]
    VendorSpecific(u8),
    #[deku(id_pat = "o if o & (1 << 15) == 0")]
    Reserved,
}
