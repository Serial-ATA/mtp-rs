use crate::error::{err, MtpError, Result};
use alloc::borrow::Cow;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::fmt::{Debug, Display, Formatter};

use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek, Write};
use deku::reader::Reader;
use deku::writer::Writer;
use deku::{DekuRead, DekuReader, DekuWrite, DekuWriter};

/// The string type of PTP (and thus MTP)
///
/// This is a string that enforces the following rules:
///
/// - A character limit of 255
///   - In reality, there is a limit of **254** characters, as the string is null-terminated
/// - No embedded null bytes
///
/// NOTE: When converting a `String` to a `PtpString`, the string will be truncated if it exceeds the
///       maximum length.
#[derive(Default, Clone, Eq, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "endian")]
pub struct PtpString(
	#[deku(
		reader = "ptp_string_read(deku::reader)",
		writer = "ptp_string_write(&self.0, deku::writer)"
	)]
	Vec<u16>,
);

impl TryFrom<String> for PtpString {
	type Error = MtpError;

	fn try_from(value: String) -> core::result::Result<Self, Self::Error> {
		if value.is_empty() {
			return Ok(Self::default());
		}

		// We need to validate the string's contents.
		if value.contains('\0') {
			err!(StringContainsNull);
		}

		// We need to validate the string's length (in characters, NOT bytes).
		let mut chars = value
			.encode_utf16()
			.take(Self::MAX_LENGTH - 1)
			.collect::<Vec<u16>>();

		// The strings ended with a codepoint that needs another unit.
		// We just have to trim it off.
		if (0xD8_00..=0xDB_FF).contains(chars.last().unwrap()) {
			chars.pop();
		}

		Ok(Self(chars))
	}
}

impl PtpString {
	pub const MAX_LENGTH: usize = 255;

	/// Returns the length of the string in UTF-16 characters.
	///
	/// NOTES:
	///   - This is **not** the same as the number of bytes in the string.
	///   - This will always be one more than the number of characters in the string, as the string is
	///     null-terminated.
	///
	/// # Examples
	///
	/// ```rust
	/// use mtp_spec::object::types::PtpString;
	///
	/// let some_message = String::from("Hello, world!");
	/// let string = PtpString::try_from(some_message).unwrap();
	///
	/// assert_eq!(string.len(), 14);
	/// ```
	pub fn len(&self) -> usize {
		self.0.len() + 1
	}

	/// Whether the string is empty.
	///
	/// # Examples
	///
	/// ```rust
	/// use mtp_spec::object::types::PtpString;
	///
	/// let message = String::new();
	/// let ptp_string = PtpString::try_from(message).unwrap();
	/// assert!(ptp_string.is_empty());
	///
	/// let filled_message = String::from("Hello, world!");
	/// let ptp_string = PtpString::try_from(filled_message).unwrap();
	/// assert!(!ptp_string.is_empty());
	/// ```
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	pub fn as_bytes(&self) -> Result<Vec<u8>> {
		// §3.2.3: "It should be noted that strings with embedded nulls are not permitted."
		if self.0.contains(&0x00) {
			err!(StringContainsNull);
		}

		// §3.2.3: "An empty string is represented by a single 8-bit integer containing a value of 0x00"
		if self.0.is_empty() {
			return Ok(vec![0x00]);
		}

		// §3.2.3: "Strings are limited to 255 characters, including the terminating null character."
		assert!(self.0.len() < Self::MAX_LENGTH);

		let mut ret = Vec::with_capacity((self.0.len() + 1) * 2);
		ret.push((self.0.len() + 1) as u8);

		for c in self.0.iter().copied() {
			ret.extend_from_slice(&c.to_be_bytes());
		}

		ret.extend_from_slice(&[0x00, 0x00]);

		Ok(ret)
	}
}

impl Display for PtpString {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}", String::from_utf16_lossy(&self.0))
	}
}

impl Debug for PtpString {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		write!(f, "{:?}", String::from_utf16_lossy(&self.0))
	}
}

/// String Definition
///
/// | Dataset field     | Size (bytes) | Datatype                       |
/// |-------------------|--------------|--------------------------------|
/// | NumChars          | 1            | UINT8                          |
/// | String Characters | Variable     | Unicode null-terminated string |
fn ptp_string_read<R: Read + Seek>(
	reader: &mut Reader<R>,
) -> core::result::Result<Vec<u16>, deku::DekuError> {
	let num_chars = u8::from_reader_with_ctx(reader, ())?;
	if num_chars == 0 {
		return Ok(Vec::new());
	}

	let mut string_characters = Vec::with_capacity(num_chars as usize);

	for _ in 0..(num_chars - 1) {
		string_characters.push(u16::from_reader_with_ctx(reader, Endian::Little)?);
	}

	let terminator = u16::from_reader_with_ctx(reader, Endian::Little)?;
	if terminator != 0 {
		return Err(deku::DekuError::Assertion(Cow::Owned(format!(
			"Expected null terminator, got 0x{:04X} (is the string the correct length?)",
			terminator
		))));
	}

	Ok(string_characters)
}

fn ptp_string_write<W: Write + Seek>(
	elements: &[u16],
	writer: &mut Writer<W>,
) -> core::result::Result<(), deku::DekuError> {
	let num_chars = elements.len() as u8;
	num_chars.to_writer(writer, ())?;

	for c in elements {
		c.to_writer(writer, Endian::Little)?;
	}

	0_u16.to_writer(writer, Endian::Little)?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	use alloc::string::ToString;

	#[test]
	fn string_truncation() {
		let string = "X".repeat(512);
		let ptp_string = PtpString::try_from(string).unwrap();

		assert_eq!(ptp_string.len(), 254 + 1);
		assert_eq!(ptp_string.to_string(), "X".repeat(254));
	}

	#[test]
	fn string_truncation_special_character() {
		let mut string = "X".repeat(253);
		// This character takes up two `u16`s, but we only have
		// space for one more. We should remove the character entirely.
		string.push('𝕊');

		let ptp_string = PtpString::try_from(string).unwrap();

		assert_eq!(ptp_string.len(), 253 + 1);
		assert_eq!(ptp_string.to_string(), "X".repeat(253));
	}

	#[test]
	fn string_null() {
		let string = "Hello, \0world!".to_string();
		let ptp_string = PtpString::try_from(string);

		assert!(ptp_string.is_err());
	}

	#[test]
	fn round_trip() {
		let message = String::from("Hello, world!");
		let ptp_string = PtpString::try_from(message).unwrap();

		assert_eq!(ptp_string.to_string(), "Hello, world!");
	}
}
