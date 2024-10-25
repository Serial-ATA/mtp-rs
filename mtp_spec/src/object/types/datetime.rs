use crate::error::{MtpError, MtpErrorKind};
use crate::object::types::PtpString;

use alloc::borrow::Cow;
use alloc::string::ToString;
use core::fmt::Display;
use core::str::FromStr;

use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek, Write};
use deku::reader::Reader;
use deku::writer::Writer;
use deku::{DekuError, DekuReader, DekuWriter};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DateTime {
	year: u16,
	month: Option<u8>,
	day: Option<u8>,
	hour: Option<u8>,
	minute: Option<u8>,
	second: Option<u8>,
	decisecond: Option<u8>,
}

impl TryFrom<PtpString> for DateTime {
	type Error = MtpError;

	fn try_from(value: PtpString) -> Result<Self, Self::Error> {
		Self::from_str(value.to_string().as_str())
	}
}

impl FromStr for DateTime {
	type Err = MtpError;

	fn from_str(mut s: &str) -> Result<Self, Self::Err> {
		fn parse_int(s: &mut &str) -> Result<Option<u8>, MtpError> {
			if s.is_empty() {
				return Ok(None);
			}

			if s.len() < 2 {
				return Err(MtpError::new(MtpErrorKind::BadDateTime(
					"A DateTime segment must be at least 2 characters long",
				)));
			}

			let segment = &s[..2];
			*s = &s[2..];

			let ret = segment.parse::<u8>().map_err(|_| {
				MtpError::new(MtpErrorKind::BadDateTime(
					"A DateTime string must contain only digits",
				))
			})?;

			Ok(Some(ret))
		}

		if s.len() < 4 {
			return Err(MtpError::new(MtpErrorKind::BadDateTime(
				"A DateTime string must be at least 4 characters long",
			)));
		}

		let year = &s[..4];
		s = &s[4..];

		let mut datetime = DateTime {
			year: year.parse().map_err(|_| {
				MtpError::new(MtpErrorKind::BadDateTime(
					"A DateTime string must start with a 4-digit year",
				))
			})?,
			month: None,
			day: None,
			hour: None,
			minute: None,
			second: None,
			decisecond: None,
		};

		'segments: {
			let Some(month) = parse_int(&mut s)? else {
				break 'segments;
			};
			datetime.month = Some(month);

			let Some(day) = parse_int(&mut s)? else {
				break 'segments;
			};
			datetime.day = Some(day);

			let mut remaining_chars = s.chars();
			if remaining_chars.next() != Some('T') {
				if s.is_empty() {
					break 'segments;
				}

				return Err(MtpError::new(MtpErrorKind::BadDateTime(
					"Expected a 'T' marking the start of the time segment",
				)));
			}

			s = &s[1..];

			let Some(hour) = parse_int(&mut s)? else {
				break 'segments;
			};
			datetime.hour = Some(hour);

			let Some(minute) = parse_int(&mut s)? else {
				break 'segments;
			};
			datetime.minute = Some(minute);

			let Some(second) = parse_int(&mut s)? else {
				break 'segments;
			};
			datetime.second = Some(second);

			if s.is_empty() {
				break 'segments;
			}

			let mut remaining_chars = s.chars();
			if remaining_chars.next() != Some('.') {
				return Err(MtpError::new(MtpErrorKind::BadDateTime(
					"Expected a period marking the start of the decisecond segment",
				)));
			}

			let Some(deciseconds) = remaining_chars.next() else {
				return Err(MtpError::new(MtpErrorKind::BadDateTime(
					"Expected a decisecond digit",
				)));
			};
			datetime.decisecond = deciseconds.to_digit(10).map(|d| d as u8);
		}

		// TODO: This string can optionally be appended with a constant character “Z” to indicate UTC, or
		//       +/-hhmm to indicate that the time is relative to a time zone. Appending neither indicates
		//       that the time zone is unspecified.

		if !datetime.validate() {
			return Err(MtpError::new(MtpErrorKind::BadDateTime(
				"DateTime string contains invalid segments",
			)));
		}

		Ok(datetime)
	}
}

impl Display for DateTime {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{:04}", self.year)?;

		if let Some(month) = self.month {
			write!(f, "-{:02}", month)?;

			if let Some(day) = self.day {
				write!(f, "-{:02}", day)?;

				if let Some(hour) = self.hour {
					write!(f, "T{:02}", hour)?;

					if let Some(minute) = self.minute {
						write!(f, ":{:02}", minute)?;

						if let Some(second) = self.second {
							write!(f, ":{:02}", second)?;

							if let Some(decisecond) = self.decisecond {
								write!(f, ".{:01}", decisecond)?;
							}
						}
					}
				}
			}
		}

		Ok(())
	}
}

impl DekuReader<'_, Endian> for DateTime {
	fn from_reader_with_ctx<R>(
		reader: &mut Reader<'_, R>,
		endian: Endian,
	) -> Result<Self, DekuError>
	where
		R: Read + Seek,
	{
		PtpString::from_reader_with_ctx(reader, endian).and_then(|s| {
			DateTime::try_from(s).map_err(|e| DekuError::InvalidParam(Cow::from(e.to_string())))
		})
	}
}

impl DekuWriter<Endian> for DateTime {
	fn to_writer<W>(&self, writer: &mut Writer<W>, endian: Endian) -> Result<(), DekuError>
	where
		W: Write + Seek,
	{
		let ptp_str = PtpString::try_from(self.to_string())
			.map_err(|e| DekuError::InvalidParam(Cow::from(e.to_string())))?;
		ptp_str.to_writer(writer, endian)
	}
}

fn verify_field(field: Option<u8>, limit: u8, parent: Option<u8>) -> bool {
	if let Some(field) = field {
		return parent.is_some() && field <= limit;
	}
	return true; // Field does not exist, so it's valid
}

impl DateTime {
	fn validate(self) -> bool {
		if self.year > 9999
			|| !verify_field(self.month, 12, Some(self.year as u8))
			|| !verify_field(self.day, 31, self.month)
			|| !verify_field(self.hour, 23, self.day)
			|| !verify_field(self.minute, 59, self.hour)
			|| !verify_field(self.second, 59, self.minute)
			|| !verify_field(self.decisecond, 9, self.second)
		{
			return false;
		}

		true
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn datetime_parse_fromstr() {
		let datetime = DateTime::from_str("20240101T123456.7").unwrap();
		assert_eq!(datetime.year, 2024);
		assert_eq!(datetime.month, Some(1));
		assert_eq!(datetime.day, Some(1));
		assert_eq!(datetime.hour, Some(12));
		assert_eq!(datetime.minute, Some(34));
		assert_eq!(datetime.second, Some(56));
		assert_eq!(datetime.decisecond, Some(7));
	}

	#[test]
	fn datetime_parse_fromstr_no_decisecond() {
		let datetime = DateTime::from_str("20240101T123456").unwrap();
		assert_eq!(datetime.year, 2024);
		assert_eq!(datetime.month, Some(1));
		assert_eq!(datetime.day, Some(1));
		assert_eq!(datetime.hour, Some(12));
		assert_eq!(datetime.minute, Some(34));
		assert_eq!(datetime.second, Some(56));
		assert_eq!(datetime.decisecond, None);
	}

	#[test]
	fn datetime_parse_fromstr_no_time() {
		let datetime = DateTime::from_str("20240101").unwrap();
		assert_eq!(datetime.year, 2024);
		assert_eq!(datetime.month, Some(1));
		assert_eq!(datetime.day, Some(1));
		assert_eq!(datetime.hour, None);
		assert_eq!(datetime.minute, None);
		assert_eq!(datetime.second, None);
		assert_eq!(datetime.decisecond, None);
	}

	#[test]
	fn datetime_parse_fromstr_utc() {
		let datetime = DateTime::from_str("20240101T123456.7Z").unwrap();
		assert_eq!(datetime.year, 2024);
		assert_eq!(datetime.month, Some(1));
		assert_eq!(datetime.day, Some(1));
		assert_eq!(datetime.hour, None);
		assert_eq!(datetime.minute, None);
		assert_eq!(datetime.second, None);
		assert_eq!(datetime.decisecond, None);
	}

	#[test]
	fn datetime_parse_fromstr_relative_positive() {
		let datetime = DateTime::from_str("20240101T123456.7+1234").unwrap();
		assert_eq!(datetime.year, 2024);
		assert_eq!(datetime.month, Some(1));
		assert_eq!(datetime.day, Some(1));
		assert_eq!(datetime.hour, None);
		assert_eq!(datetime.minute, None);
		assert_eq!(datetime.second, None);
		assert_eq!(datetime.decisecond, None);
	}

	#[test]
	fn datetime_parse_fromstr_relative_negative() {
		let datetime = DateTime::from_str("20240101T123456.7-1234").unwrap();
		assert_eq!(datetime.year, 2024);
		assert_eq!(datetime.month, Some(1));
		assert_eq!(datetime.day, Some(1));
		assert_eq!(datetime.hour, None);
		assert_eq!(datetime.minute, None);
		assert_eq!(datetime.second, None);
		assert_eq!(datetime.decisecond, None);
	}
}
