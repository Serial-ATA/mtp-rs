use crate::communication::Parameter;
use crate::object::types::ArrayEncodable;

use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek, Write};
use deku::prelude::{Reader, Writer};
use deku::{DekuError, DekuReader, DekuWriter};

macro_rules! format_codes {
	(
		$(
			$(#[$meta:meta])*
			$name:ident = $value:expr,
		)*
	) => {
		/// Predefined object formats to identify device contents.
		///
		/// As MTP is file-system-agnostic, the usual method of overloading file extensions to
		/// determine file type is not a reliable method of identifying device contents. In many
		/// cases, objects on a device may have no qualified filename, or may not even exist until
		/// requested for transfer by the Initiator.
		#[repr(u16)]
		#[derive(
			Copy,
			Clone,
			Debug,
			Eq,
			PartialEq,
			Ord,
			PartialOrd,
			deku::DekuRead,
			deku::DekuWrite,
		)]
		#[allow(missing_docs)]
		#[deku(id_type = "u16", id_endian = "little")]
		pub enum ObjectFormatCode {
			$(
				$(#[$meta])*
				#[deku(id = $value)]
				$name = $value,
			)*
			// Some other vendor-specific or otherwise unknown format code
			#[deku(id_pat = "_", default)]
			Unknown(#[deku(endian = "little")] u16),
		}

		impl From<ObjectFormatCode> for u16 {
			fn from(value: ObjectFormatCode) -> Self {
				match value {
					$(
						ObjectFormatCode::$name => $value,
					)*
					ObjectFormatCode::Unknown(value) => value,
				}
			}
		}

		impl From<u16> for ObjectFormatCode {
			fn from(value: u16) -> Self {
				match value {
					$(
						$value => ObjectFormatCode::$name,
					)*
					_ => ObjectFormatCode::Unknown(value),
				}
			}
		}
	};
}

format_codes!(
	/// Undefined object
	Undefined = 0x3000,
	/// Association (for example, a folder)
	Association = 0x3001,
	/// Device model-specific script
	Script = 0x3002,
	/// Device model-specific executable
	Executable = 0x3003,
	/// Text file
	Text = 0x3004,
	/// Hypertext Markup Language file (text)
	Html = 0x3005,
	/// Digital Print Order Format file (text)
	Dpof = 0x3006,
	/// AIFF audio clip
	Aiff = 0x3007,
	/// WAVE audio clip
	Wav = 0x3008,
	/// MPEG-1 Layer III audio (ISO/IEC 13818-3)
	Mp3 = 0x3009,
	/// AVI video clip
	Avi = 0x300A,
	/// MPEG video clip
	Mpeg = 0x300B,
	/// Microsoft Advanced Streaming Format (video)
	Asf = 0x300C,
	/// Undefined image object
	UndefinedImage = 0x3800,
	/// Exchangeable File Format, JEIDA standard
	ExifJpeg = 0x3801,
	/// Tag Image File Format for Electronic Photography
	TiffEp = 0x3802,
	/// Structured Storage Image Format
	FlashPix = 0x3803,
	/// Microsoft Windows Bitmap file
	Bmp = 0x3804,
	/// Canon Camera Image File Format
	Ciff = 0x3805,
	/// Reserved
	UndefinedImage2 = 0x3806,
	/// Graphics Interchange Format
	Gif = 0x3807,
	/// JPEG File Interchange Format
	Jfif = 0x3808,
	/// PhotoCD Image Pac
	Cd = 0x3809,
	/// Quickdraw Image Format
	Pict = 0x380A,
	/// Portable Network Graphics
	Png = 0x380B,
	/// Reserved
	UndefinedImage3 = 0x380C,
	/// Tag Image File Format
	Tiff = 0x380D,
	/// Tag Image File Format for Information Technology (graphic arts)
	TiffIt = 0x380E,
	/// JPEG2000 Baseline File Format
	Jp2 = 0x380F,
	/// JPEG2000 Extended File Format
	Jpx = 0x3810,
	UndefinedFirmware = 0xB802,
	WindowsImageFormat = 0xB881,
	/// Wireless Application Protocol Bitmap Format (.wbmp) (MIME: image/vnd.wap.wbmp)
	///
	/// <http://www.wapforum.org/what/technical/SPEC-WAESpec-19990524.pdf>
	Wbmp = 0xB803,
	/// JPEG XR, also known as HD Photo (.hdp, jxr, .wpd) (MIME: image/vnd.ms-photo)
	///
	/// ISO/IEC 29199-2:2009: <http://www.iso.org/iso/iso_catalogue/catalogue_tc/catalogue_detail.htm?csnumber=51609>
	JpegXr = 0xB804,
	/// Undefined audio object
	UndefinedAudio = 0xB900,
	/// Windows Media Audio
	Wma = 0xB901,
	Ogg = 0xB902,
	/// Advanced Audio Coding (.aac) (MIME: audio/aac)
	///
	/// MPEG-4 AAC.
	Aac = 0xB903,
	Audible = 0xB904,
	/// Free Lossless Audio Codec
	Flac = 0xB906,
	/// Qualcomm Code Excited Linear Prediction (.qcp) (MIME: audio/qcelp)
	Qcelp = 0xB907,
	/// Adaptive Multi-Rate audio codec (.amr) (MIME: audio/amr)
	Amr = 0xB908,
	/// Undefined video object
	UndefinedVideo = 0xB980,
	/// Windows Media Video
	Wmv = 0xB981,
	/// ISO 14496-1
	Mp4Container = 0xB982,
	/// MPEG-1 Layer II audio (ISO/IEC 13818-3)
	Mp2 = 0xB983,
	/// 3GPP file format.
	///
	/// Details: <http://www.3gpp.org/ftp/Specs/html-info/26244.htm>
	Container3gp = 0xB984,
	/// 3GPP2 format (.3g2). (MIME: video/3gpp2, audio/3gpp2)
	///
	/// <http://www.3gpp2.org/Public_html/specs/C.S0050-B_v1.0_070521.pdf>
	Container3g2 = 0xB985,
	/// MPEG-4 AVC video and Dolby Digital audio within an MPEG-2
	/// Transport Stream as constrained by the AVCHD format specification
	///
	/// <http://www.avchd-info.org/>
	Avchd = 0xB986,
	/// MPEG-2 video and AC-3 audio within an ATSC-compliant MPEG-2 Transport Stream
	AtscTs = 0xB987,
	/// MPEG-2 video and MPEG-1 Layer II or AC-3 audio within a DVB-compliant MPEG-2
	/// Transport Stream
	DvbTs = 0xB988,
	UndefinedCollection = 0xBA00,
	AbstractMultimediaAlbum = 0xBA01,
	AbstractImageAlbum = 0xBA02,
	AbstractAudioAlbum = 0xBA03,
	AbstractVideoAlbum = 0xBA04,
	AbstractAVPlaylist = 0xBA05,
	AbstractContactGroup = 0xBA06,
	AbstractMessageFolder = 0xBA07,
	AbstractChapteredProduction = 0xBA08,
	AbstractAudioPlaylist = 0xBA09,
	AbstractVideoPlaylist = 0xBA0A,
	/// For use with mediacasts; references multimedia enclosures of RSS feeds or episodic content
	AbstractMediacast = 0xBA0B,
	WplPlaylist = 0xBA10,
	M3uPlaylist = 0xBA11,
	MplPlaylist = 0xBA12,
	AsxPlaylist = 0xBA13,
	PlsPlaylist = 0xBA14,
	UndefinedDocument = 0xBA80,
	AbstractDocument = 0xBA81,
	XmlDocument = 0xBA82,
	MicrosoftWordDocument = 0xBA83,
	MhtCompiledHtmlDocument = 0xBA84,
	MicrosoftExcelSpreadsheet = 0xBA85,
	MicrosoftPowerPointPresentation = 0xBA86,
	UndefinedMessage = 0xBB00,
	AbstractMessage = 0xBB01,
	UndefinedBookmark = 0xBB10,
	AbstractBookmark = 0xBB11,
	UndefinedAppointment = 0xBB20,
	AbstractAppointment = 0xBB21,
	/// vCalendar 1.0
	VCalendar1 = 0xBB22,
	UndefinedTask = 0xBD00,
	AbstractTask = 0xBD01,
	ICalendar = 0xBB42,
	UndefinedNote = 0xBB60,
	AbstractNote = 0xBB61,
	UndefinedContact = 0xBB80,
	AbstractContact = 0xBB81,
	VCard2 = 0xBB82,
	VCard3 = 0xBB83,
);

impl From<ObjectFormatCode> for Parameter {
	fn from(value: ObjectFormatCode) -> Self {
		Parameter::new(u32::from(u16::from(value)))
	}
}

impl<'a> DekuReader<'a, Endian> for ObjectFormatCode {
	fn from_reader_with_ctx<R: Read + Seek>(
		reader: &mut Reader<R>,
		_: Endian,
	) -> Result<Self, DekuError>
	where
		Self: Sized,
	{
		// Little endian always
		ObjectFormatCode::from_reader_with_ctx(reader, ())
	}
}

impl DekuWriter<Endian> for ObjectFormatCode {
	fn to_writer<W: Write + Seek>(
		&self,
		writer: &mut Writer<W>,
		_: Endian,
	) -> Result<(), DekuError> {
		// Little endian always
		self.to_writer(writer, ())
	}
}

impl ArrayEncodable for ObjectFormatCode {}
