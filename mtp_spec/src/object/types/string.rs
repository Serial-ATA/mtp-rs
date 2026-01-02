use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::fmt::{Debug, Display, Formatter};
use core::str::FromStr;

use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek, Write};
use deku::reader::Reader;
use deku::writer::Writer;
use deku::{DekuError, DekuRead, DekuReader, DekuWrite, DekuWriter};

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
///
/// ## Usage
///
/// ```rust
/// use mtp_spec::object::types::PtpString;
///
/// // Converting `String`s to `PtpString`
/// let some_text = String::from("foo");
/// let some_text_ptp = PtpString::try_from(some_text).expect("string contains no null bytes");
///
/// let some_long_text = "X".repeat(300);
/// let some_long_text_ptp =
///     PtpString::try_from(some_long_text).expect("string contains no null bytes");
///
/// // `some_long_text` was truncated to 254 bytes + 1 for the null terminator
/// assert_eq!(some_long_text_ptp.len(), 255);
/// ```
#[derive(Default, Clone, Eq, PartialEq, Hash, DekuRead, DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Little"
)]
pub struct PtpString(
    #[deku(
        reader = "ptp_string_read(deku::reader, endian)",
        writer = "ptp_string_write(&self.0, deku::writer)"
    )]
    Vec<u16>,
);

impl PtpString {
    /// In some contexts, empty strings are used to signify no value. Convert them to `Option`s.
    pub(crate) fn parse_optional(string: PtpString) -> Result<Option<PtpString>, DekuError> {
        if string.is_empty() {
            Ok(None)
        } else {
            Ok(Some(string))
        }
    }
}

/// Attempting to deserialize a [`PtpString`] containing a null byte
///
/// [`PtpString`]: crate::object::types::PtpString
#[derive(Copy, Clone, Debug)]
pub struct NulError;

impl Display for NulError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "String contains null bytes")
    }
}

impl TryFrom<&str> for PtpString {
    type Error = NulError;

    fn try_from(value: &str) -> core::result::Result<Self, Self::Error> {
        if value.is_empty() {
            return Ok(Self::default());
        }

        // We need to validate the string's contents.
        if value.contains('\0') {
            return Err(NulError);
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

impl FromStr for PtpString {
    type Err = NulError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl PtpString {
    /// The maximum length of a `PtpString`, including the null-terminator.
    ///
    /// This is in characters, not bytes.
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

    /// Convert the `PtpString` into bytes, for transport
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mtp_spec::object::types::PtpString;
    ///
    /// let message = String::from("Hello, world!");
    /// let ptp_string = PtpString::try_from(message.clone()).unwrap();
    ///
    /// let bytes = ptp_string.as_bytes();
    ///
    /// // A `PtpString` is just a null-terminated UTF-16 string
    /// let mut expected_bytes = message.encode_utf16().collect::<Vec<_>>();
    /// expected_bytes.extend_from_slice(&[0, 0]);
    ///
    /// assert_eq!(bytes, expected_bytes);
    /// ```
    #[allow(clippy::missing_panics_doc)]
    pub fn as_bytes(&self) -> Vec<u8> {
        // §3.2.3: "An empty string is represented by a single 8-bit integer containing a value of 0x00"
        if self.0.is_empty() {
            return vec![0x00];
        }

        // §3.2.3: "Strings are limited to 255 characters, including the terminating null character."
        assert!(self.0.len() < Self::MAX_LENGTH);

        let mut ret = Vec::with_capacity((self.0.len() + 1) * 2);
        ret.push((self.0.len() + 1) as u8);

        for c in self.0.iter().copied() {
            ret.extend_from_slice(&c.to_be_bytes());
        }

        ret.extend_from_slice(&[0x00, 0x00]);

        ret
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
    endian: Endian,
) -> core::result::Result<Vec<u16>, deku::DekuError> {
    let num_chars = u8::from_reader_with_ctx(reader, endian)?;
    if num_chars == 0 {
        return Ok(Vec::new());
    }

    let mut string_characters = Vec::with_capacity(num_chars as usize);

    for _ in 0..(num_chars - 1) {
        string_characters.push(u16::from_reader_with_ctx(reader, endian)?);
    }

    let terminator = u16::from_reader_with_ctx(reader, endian)?;
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
    assert!(elements.len() < 255);

    if elements.is_empty() {
        0_u16.to_writer(writer, Endian::Little)?;
        return Ok(());
    }

    // The character count includes the terminator
    let num_chars = (elements.len() as u8) + 1;
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
