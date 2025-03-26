use crate::communication::Parameter;

use alloc::format;
use alloc::vec::Vec;

use deku::{DekuRead, DekuWrite};

/// Predefined object formats to identify device contents.
///
/// As MTP is file-system-agnostic, the usual method of overloading file extensions to
/// determine file type is not a reliable method of identifying device contents. In many
/// cases, objects on a device may have no qualified filename, or may not even exist until
/// requested for transfer by the Initiator.
#[repr(u16)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, DekuRead, DekuWrite)]
#[deku(id_type = "u16", endian = "big")]
pub enum ObjectFormatCode {
	/// Undefined object
	#[deku(id = "0x3000")]
	Undefined = 0x3000,
	/// Association (for example, a folder)
	#[deku(id = "0x3001")]
	Association = 0x3001,
	/// Device model-specific script
	#[deku(id = "0x3002")]
	Script = 0x3002,
	/// Device model-specific executable
	#[deku(id = "0x3003")]
	Executable = 0x3003,
	/// Text file
	#[deku(id = "0x3004")]
	Text = 0x3004,
	/// Hypertext Markup Language file (text)
	#[deku(id = "0x3005")]
	Html = 0x3005,
	/// Digital Print Order Format file (text)
	#[deku(id = "0x3006")]
	Dpof = 0x3006,
	/// AIFF audio clip
	#[deku(id = "0x3007")]
	Aiff = 0x3007,
	/// WAVE audio clip
	#[deku(id = "0x3008")]
	Wav = 0x3008,
	/// MPEG-1 Layer III audio (ISO/IEC 13818-3)
	#[deku(id = "0x3009")]
	Mp3 = 0x3009,
	/// AVI video clip
	#[deku(id = "0x300A")]
	Avi = 0x300A,
	/// MPEG video clip
	#[deku(id = "0x300B")]
	Mpeg = 0x300B,
	/// Microsoft Advanced Streaming Format (video)
	#[deku(id = "0x300C")]
	Asf = 0x300C,
	/// Undefined image object
	#[deku(id = "0x3800")]
	UndefinedImage = 0x3800,
	/// Exchangeable File Format, JEIDA standard
	#[deku(id = "0x3801")]
	ExifJpeg = 0x3801,
	/// Tag Image File Format for Electronic Photography
	#[deku(id = "0x3802")]
	TiffEp = 0x3802,
	/// Structured Storage Image Format
	#[deku(id = "0x3803")]
	FlashPix = 0x3803,
	/// Microsoft Windows Bitmap file
	#[deku(id = "0x3804")]
	Bmp = 0x3804,
	/// Canon Camera Image File Format
	#[deku(id = "0x3805")]
	Ciff = 0x3805,
	/// Reserved
	#[deku(id = "0x3806")]
	UndefinedImage2 = 0x3806,
	/// Graphics Interchange Format
	#[deku(id = "0x3807")]
	Gif = 0x3807,
	/// JPEG File Interchange Format
	#[deku(id = "0x3808")]
	Jfif = 0x3808,
	/// PhotoCD Image Pac
	#[deku(id = "0x3809")]
	Cd = 0x3809,
	/// Quickdraw Image Format
	#[deku(id = "0x380A")]
	Pict = 0x380A,
	/// Portable Network Graphics
	#[deku(id = "0x380B")]
	Png = 0x380B,
	/// Reserved
	#[deku(id = "0x380C")]
	UndefinedImage3 = 0x380C,
	/// Tag Image File Format
	#[deku(id = "0x380D")]
	Tiff = 0x380D,
	/// Tag Image File Format for Information Technology (graphic arts)
	#[deku(id = "0x380E")]
	TiffIt = 0x380E,
	/// JPEG2000 Baseline File Format
	#[deku(id = "0x380F")]
	Jp2 = 0x380F,
	/// JPEG2000 Extended File Format
	#[deku(id = "0x3810")]
	Jpx = 0x3810,
	#[deku(id = "0xB802")]
	UndefinedFirmware = 0xB802,
	#[deku(id = "0xB881")]
	WindowsImageFormat = 0xB881,
	/// Wireless Application Protocol Bitmap Format (.wbmp) (MIME: image/vnd.wap.wbmp)
	///
	/// <http://www.wapforum.org/what/technical/SPEC-WAESpec-19990524.pdf>
	#[deku(id = "0xB803")]
	Wbmp = 0xB803,
	/// JPEG XR, also known as HD Photo (.hdp, jxr, .wpd) (MIME: image/vnd.ms-photo)
	///
	/// ISO/IEC 29199-2:2009: <http://www.iso.org/iso/iso_catalogue/catalogue_tc/catalogue_detail.htm?csnumber=51609>
	#[deku(id = "0xB804")]
	JpegXr = 0xB804,
	/// Undefined audio object
	#[deku(id = "0xB900")]
	UndefinedAudio = 0xB900,
	/// Windows Media Audio
	#[deku(id = "0xB901")]
	Wma = 0xB901,
	#[deku(id = "0xB902")]
	Ogg = 0xB902,
	/// Advanced Audio Coding (.aac) (MIME: audio/aac)
	///
	/// MPEG-4 AAC.
	#[deku(id = "0xB903")]
	Aac = 0xB903,
	#[deku(id = "0xB904")]
	Audible = 0xB904,
	/// Free Lossless Audio Codec
	#[deku(id = "0xB906")]
	Flac = 0xB906,
	/// Qualcomm Code Excited Linear Prediction (.qcp) (MIME: audio/qcelp)
	#[deku(id = "0xB907")]
	Qcelp = 0xB907,
	/// Adaptive Multi-Rate audio codec (.amr) (MIME: audio/amr)
	#[deku(id = "0xB908")]
	Amr = 0xB908,
	/// Undefined video object
	#[deku(id = "0xB980")]
	UndefinedVideo = 0xB980,
	/// Windows Media Video
	#[deku(id = "0xB981")]
	Wmv = 0xB981,
	/// ISO 14496-1
	#[deku(id = "0xB982")]
	Mp4Container = 0xB982,
	/// MPEG-1 Layer II audio (ISO/IEC 13818-3)
	#[deku(id = "0xB983")]
	Mp2 = 0xB983,
	/// 3GPP file format.
	///
	/// Details: <http://www.3gpp.org/ftp/Specs/html-info/26244.htm>
	#[deku(id = "0xB984")]
	Container3gp = 0xB984,
	/// 3GPP2 format (.3g2). (MIME: video/3gpp2, audio/3gpp2)
	///
	/// <http://www.3gpp2.org/Public_html/specs/C.S0050-B_v1.0_070521.pdf>
	#[deku(id = "0xB985")]
	Container3g2 = 0xB985,
	/// MPEG-4 AVC video and Dolby Digital audio within an MPEG-2
	/// Transport Stream as constrained by the AVCHD format specification
	///
	/// <http://www.avchd-info.org/>
	#[deku(id = "0xB986")]
	Avchd = 0xB986,
	/// MPEG-2 video and AC-3 audio within an ATSC-compliant MPEG-2 Transport Stream
	#[deku(id = "0xB987")]
	AtscTs = 0xB987,
	/// MPEG-2 video and MPEG-1 Layer II or AC-3 audio within a DVB-compliant MPEG-2
	/// Transport Stream
	#[deku(id = "0xB988")]
	DvbTs = 0xB988,
	#[deku(id = "0xBA00")]
	UndefinedCollection = 0xBA00,
	#[deku(id = "0xBA01")]
	AbstractMultimediaAlbum = 0xBA01,
	#[deku(id = "0xBA02")]
	AbstractImageAlbum = 0xBA02,
	#[deku(id = "0xBA03")]
	AbstractAudioAlbum = 0xBA03,
	#[deku(id = "0xBA04")]
	AbstractVideoAlbum = 0xBA04,
	#[deku(id = "0xBA05")]
	AbstractAVPlaylist = 0xBA05,
	#[deku(id = "0xBA06")]
	AbstractContactGroup = 0xBA06,
	#[deku(id = "0xBA07")]
	AbstractMessageFolder = 0xBA07,
	#[deku(id = "0xBA08")]
	AbstractChapteredProduction = 0xBA08,
	#[deku(id = "0xBA09")]
	AbstractAudioPlaylist = 0xBA09,
	#[deku(id = "0xBA0A")]
	AbstractVideoPlaylist = 0xBA0A,
	/// For use with mediacasts; references multimedia enclosures of RSS feeds or episodic content
	#[deku(id = "0xBA0B")]
	AbstractMediacast = 0xBA0B,
	#[deku(id = "0xBA10")]
	WplPlaylist = 0xBA10,
	#[deku(id = "0xBA11")]
	M3uPlaylist = 0xBA11,
	#[deku(id = "0xBA12")]
	MplPlaylist = 0xBA12,
	#[deku(id = "0xBA13")]
	AsxPlaylist = 0xBA13,
	#[deku(id = "0xBA14")]
	PlsPlaylist = 0xBA14,
	#[deku(id = "0xBA80")]
	UndefinedDocument = 0xBA80,
	#[deku(id = "0xBA81")]
	AbstractDocument = 0xBA81,
	#[deku(id = "0xBA82")]
	XmlDocument = 0xBA82,
	#[deku(id = "0xBA83")]
	MicrosoftWordDocument = 0xBA83,
	#[deku(id = "0xBA84")]
	MhtCompiledHtmlDocument = 0xBA84,
	#[deku(id = "0xBA85")]
	MicrosoftExcelSpreadsheet = 0xBA85,
	#[deku(id = "0xBA86")]
	MicrosoftPowerPointPresentation = 0xBA86,
	#[deku(id = "0xBB00")]
	UndefinedMessage = 0xBB00,
	#[deku(id = "0xBB01")]
	AbstractMessage = 0xBB01,
	#[deku(id = "0xBB10")]
	UndefinedBookmark = 0xBB10,
	#[deku(id = "0xBB11")]
	AbstractBookmark = 0xBB11,
	#[deku(id = "0xBB20")]
	UndefinedAppointment = 0xBB20,
	#[deku(id = "0xBB21")]
	AbstractAppointment = 0xBB21,
	/// vCalendar 1.0
	#[deku(id = "0xBB22")]
	VCalendar1 = 0xBB22,
	#[deku(id = "0xBD00")]
	UndefinedTask = 0xBD00,
	#[deku(id = "0xBD01")]
	AbstractTask = 0xBD01,
	#[deku(id = "0xBB42")]
	ICalendar = 0xBB42,
	#[deku(id = "0xBB60")]
	UndefinedNote = 0xBB60,
	#[deku(id = "0xBB61")]
	AbstractNote = 0xBB61,
	#[deku(id = "0xBB80")]
	UndefinedContact = 0xBB80,
	#[deku(id = "0xBB81")]
	AbstractContact = 0xBB81,
	#[deku(id = "0xBB82")]
	VCard2 = 0xBB82,
	#[deku(id = "0xBB83")]
	VCard3 = 0xBB83,
}

impl From<ObjectFormatCode> for Parameter {
	fn from(value: ObjectFormatCode) -> Self {
		Parameter::new(value as u32)
	}
}
