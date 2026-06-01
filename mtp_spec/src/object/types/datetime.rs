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

/// The timezone of a [`DateTime`]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Timezone {
    /// UTC
    Utc,
    /// A positive or negative offset from UTC
    Offset {
        /// The hour offset from UTC
        hour: i8,
        /// The minute offset from UTC
        minute: i8,
    },
}

impl Timezone {
    fn validate(self) -> bool {
        match self {
            Timezone::Utc => true,
            Timezone::Offset { hour, minute } => {
                hour.unsigned_abs() <= 14 && minute.unsigned_abs() <= 59
            },
        }
    }
}

/// An MTP date and time string
///
/// The format is `YYYYMMDDThhmmss.s`, where:
///
/// - `YYYY` is the year
/// - `MM` is the month
/// - `DD` is the day
/// - `hh` is the hour
/// - `mm` is the minute
/// - `ss` is the second
/// - `s` is the decisecond
///
/// It can optionally have `Z` appended to the end to indicate UTC, or `+/-hhmm` to indicate a timezone offset.
///
/// ## Usage
///
/// `DateTime` can be constructed in three ways:
///
/// 1. Manually
///
/// ```rust
/// use mtp_spec::object::types::DateTime;
///
/// let dt = DateTime {
///     year: 1984,
///     month: Some(1),
///     day: Some(2),
///     hour: Some(3),
///     minute: Some(4),
///     second: Some(5),
///     decisecond: Some(6),
/// };
/// ```
///
/// 2. From a `str`
/// ```rust
/// use mtp_spec::object::types::DateTime;
///
/// let dt: DateTime = "19840102T030405.6".parse().expect("valid DateTime");
/// ```
///
/// 3. From a [`PtpString`]
///
/// ```rust
/// use mtp_spec::object::types::{DateTime, PtpString};
/// use std::str::FromStr;
///
/// let ptp_string = PtpString::from_str("19840102T030405.6").expect("valid PtpString");
///
/// let dt: DateTime = ptp_string.try_into().expect("valid DateTime");
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Default)]
#[allow(missing_docs)]
pub struct DateTime {
    pub year: u16,
    pub month: Option<u8>,
    pub day: Option<u8>,
    pub hour: Option<u8>,
    pub minute: Option<u8>,
    pub second: Option<u8>,
    pub decisecond: Option<u8>,
    pub timezone: Option<Timezone>,
}

#[cfg(feature = "time")]
impl DateTime {
    /// Convert a [`DateTime`] to [`SystemTime`](std::time::SystemTime)
    ///
    /// This is useful for altering filesystem timestamps on the host-side.
    ///
    /// This will return `None` if `year < 1900`.
    pub fn as_systemtime(self) -> Option<std::time::SystemTime> {
        if self.year < 1900 {
            return None;
        }

        let mut tm = libc::tm {
            tm_sec: libc::c_int::from(self.second.unwrap_or(0)),
            tm_min: libc::c_int::from(self.minute.unwrap_or(0)),
            tm_hour: libc::c_int::from(self.hour.unwrap_or(0)),
            tm_mday: libc::c_int::from(self.day.unwrap_or(0)),
            tm_mon: libc::c_int::from(self.month.unwrap_or(0)),
            tm_year: libc::c_int::from(self.year),
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: -1,
            tm_gmtoff: 0,
            tm_zone: c"".as_ptr().cast(),
        };

        let time = unsafe { libc::mktime(&raw mut tm) };

        std::time::SystemTime::UNIX_EPOCH.checked_add(std::time::Duration::from_millis(time as u64))
    }

    /// Get the current system time as a `DateTime`
    pub fn now() -> DateTime {
        let duration = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::ZERO)
            .as_secs() as libc::time_t;

        let duration_ptr = &raw const duration;
        let tm = unsafe { libc::localtime(duration_ptr) };

        unsafe {
            Self {
                year: (*tm).tm_year as _,
                month: Some((*tm).tm_mon as _),
                day: Some((*tm).tm_mday as _),
                hour: Some((*tm).tm_hour as _),
                minute: Some((*tm).tm_min as _),
                second: Some((*tm).tm_sec as _),
                decisecond: None,
                // TODO: Could actually set from `tm_gmtoff`, but not needed so far
                timezone: None,
            }
        }
    }
}

impl DateTime {
    /// In many cases, empty date time strings are valid.
    pub(crate) fn parse_optional(string: PtpString) -> Result<Option<DateTime>, DekuError> {
        if string.is_empty() {
            return Ok(None);
        }

        match Self::try_from(string) {
            Ok(datetime) => Ok(Some(datetime)),
            Err(e) => Err(DekuError::Parse(e.to_string().into())),
        }
    }

    pub(crate) fn parse_ptp_string<R>(
        reader: &mut Reader<R>,
        ctx: Endian,
    ) -> Result<Self, DekuError>
    where
        R: Read + Seek,
    {
        let ptp_str = PtpString::from_reader_with_ctx(reader, ctx)?;
        TryInto::<DateTime>::try_into(ptp_str).map_err(|e| DekuError::Parse(e.to_string().into()))
    }
}

impl TryFrom<PtpString> for DateTime {
    type Error = DateTimeError;

    fn try_from(value: PtpString) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl TryFrom<&PtpString> for DateTime {
    type Error = DateTimeError;

    fn try_from(value: &PtpString) -> Result<Self, Self::Error> {
        Self::from_str(value.to_string().as_str())
    }
}

impl From<DateTime> for PtpString {
    fn from(value: DateTime) -> Self {
        // TODO: Could probably be more efficient
        value.to_string().parse().expect("should be valid")
    }
}

/// Errors that can occur while parsing a [`DateTime`]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DateTimeError {
    /// The string has a segment too short to be
    BadSegmentLength,
    /// A segment contains non-digit characters
    NonDigit,
    /// The string doesn't meet the minimum length of 4 characters
    TooShort,
    /// The string is missing a year (the only required field)
    MissingYear,
    /// The string has more content after the `day`, but is missing a time marker (`T`)
    MissingTimeMarker,
    /// The string has more content after the `second`, but is missing a decisecond marker (`.`)
    MissingDecisecondMarker,
    /// The string has a decisecond marker, but no digit
    MissingDecisecond,
    /// The string has a timezone indicator, but no information
    MissingTimezone,
    /// One or more fields are malformed (i.e. `month > 12`, `hour > 23`, etc.)
    FailedValidation,
}

impl Display for DateTimeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DateTimeError::BadSegmentLength => {
                f.write_str("A DateTime segment must be at least 2 characters long")
            },
            DateTimeError::NonDigit => f.write_str("A DateTime segment must contain only digits"),
            DateTimeError::TooShort => {
                f.write_str("A DateTime string must be at least 4 characters long")
            },
            DateTimeError::MissingYear => {
                f.write_str("A DateTime string must start with a 4-digit year")
            },
            DateTimeError::MissingTimeMarker => {
                f.write_str("Expected a 'T' marking the start of the time segment")
            },
            DateTimeError::MissingDecisecondMarker => {
                f.write_str("Expected a period marking the start of the decisecond segment")
            },
            DateTimeError::MissingDecisecond => f.write_str("Expected a decisecond digit"),
            DateTimeError::MissingTimezone => f.write_str("Expected a timezone offset"),
            DateTimeError::FailedValidation => {
                f.write_str("DateTime string contains invalid segments")
            },
        }
    }
}

impl FromStr for DateTime {
    type Err = DateTimeError;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        fn parse_int(s: &mut &str) -> Result<Option<u8>, DateTimeError> {
            if s.is_empty() {
                return Ok(None);
            }

            if s.len() < 2 {
                return Err(DateTimeError::BadSegmentLength);
            }

            let segment = &s[..2];
            *s = &s[2..];

            let Ok(ret) = segment.parse::<u8>() else {
                return Err(DateTimeError::NonDigit);
            };

            Ok(Some(ret))
        }

        if s.len() < 4 {
            return Err(DateTimeError::TooShort);
        }

        let year = &s[..4];
        s = &s[4..];

        let Ok(year) = year.parse() else {
            return Err(DateTimeError::MissingYear);
        };

        let mut datetime = DateTime {
            year,
            month: None,
            day: None,
            hour: None,
            minute: None,
            second: None,
            decisecond: None,
            timezone: None,
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

                return Err(DateTimeError::MissingTimeMarker);
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
                return Err(DateTimeError::MissingDecisecondMarker);
            }

            let Some(deciseconds) = remaining_chars.next() else {
                return Err(DateTimeError::MissingDecisecond);
            };
            datetime.decisecond = deciseconds.to_digit(10).map(|d| d as u8);

            match remaining_chars.next() {
                Some('Z') => datetime.timezone = Some(Timezone::Utc),
                Some('-') => {
                    let mut timezone = remaining_chars.as_str();
                    datetime.timezone = Some(Timezone::Offset {
                        hour: parse_int(&mut timezone)?
                            .ok_or(DateTimeError::BadSegmentLength)?
                            .cast_signed(),
                        minute: parse_int(&mut timezone)?
                            .ok_or(DateTimeError::BadSegmentLength)?
                            .cast_signed(),
                    })
                },
                Some('+') => {
                    let mut timezone = remaining_chars.as_str();
                    datetime.timezone = Some(Timezone::Offset {
                        hour: parse_int(&mut timezone)?.ok_or(DateTimeError::BadSegmentLength)?
                            as i8,
                        minute: parse_int(&mut timezone)?.ok_or(DateTimeError::BadSegmentLength)?
                            as i8,
                    })
                },
                Some(_) => return Err(DateTimeError::MissingTimezone),
                None => {},
            }
        }

        // TODO: This string can optionally be appended with a constant character “Z” to indicate UTC, or
        //       +/-hhmm to indicate that the time is relative to a time zone. Appending neither indicates
        //       that the time zone is unspecified.

        if !datetime.validate() {
            return Err(DateTimeError::FailedValidation);
        }

        Ok(datetime)
    }
}

impl Display for DateTime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:04}", self.year)?;

        if let Some(month) = self.month {
            write!(f, "{:02}", month)?;

            if let Some(day) = self.day {
                write!(f, "{:02}", day)?;

                if let Some(hour) = self.hour {
                    write!(f, "T{:02}", hour)?;

                    if let Some(minute) = self.minute {
                        write!(f, "{:02}", minute)?;

                        if let Some(second) = self.second {
                            write!(f, "{:02}", second)?;

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

impl DekuReader<'_, ()> for DateTime {
    fn from_reader_with_ctx<R>(reader: &mut Reader<R>, _: ()) -> Result<Self, DekuError>
    where
        R: Read + Seek,
    {
        DateTime::from_reader_with_ctx(reader, Endian::Big)
    }
}

impl DekuReader<'_, Endian> for DateTime {
    fn from_reader_with_ctx<R>(reader: &mut Reader<R>, ctx: Endian) -> Result<Self, DekuError>
    where
        R: Read + Seek,
    {
        PtpString::from_reader_with_ctx(reader, ctx).and_then(|s| {
            DateTime::try_from(s).map_err(|e| DekuError::InvalidParam(Cow::from(e.to_string())))
        })
    }
}

impl DekuWriter for DateTime {
    fn to_writer<W>(&self, writer: &mut Writer<W>, _: ()) -> Result<(), DekuError>
    where
        W: Write + Seek,
    {
        DateTime::to_writer(self, writer, Endian::Big)
    }
}

impl DekuWriter<Endian> for DateTime {
    fn to_writer<W>(&self, writer: &mut Writer<W>, ctx: Endian) -> Result<(), DekuError>
    where
        W: Write + Seek,
    {
        if !self.validate() {
            return Err(DekuError::InvalidParam(Cow::from("DateTime is invalid")));
        }

        let ptp_str = PtpString::from_str(&self.to_string())
            .map_err(|e| DekuError::InvalidParam(Cow::from(e.to_string())))?;
        ptp_str.to_writer(writer, ctx)
    }
}

fn verify_field(field: Option<u8>, min: u8, max: u8, parent_exists: bool) -> bool {
    if let Some(val) = field {
        return parent_exists && val >= min && val <= max;
    }
    true // Field does not exist, so it's valid
}

impl DateTime {
    fn validate(self) -> bool {
        if self.year > 9999
            || !verify_field(self.month, 1, 12, true)
            || !verify_field(self.day, 1, 31, self.month.is_some())
            || !verify_field(self.hour, 0, 23, self.day.is_some())
            || !verify_field(self.minute, 0, 59, self.hour.is_some())
            || !verify_field(self.second, 0, 59, self.minute.is_some())
            || !verify_field(self.decisecond, 0, 9, self.second.is_some())
            || !self
                .timezone
                .is_none_or(|tz| self.hour.is_some() && tz.validate())
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
        assert_eq!(datetime.hour, Some(12));
        assert_eq!(datetime.minute, Some(34));
        assert_eq!(datetime.second, Some(56));
        assert_eq!(datetime.decisecond, Some(7));
    }

    #[test]
    fn datetime_parse_fromstr_relative_positive() {
        let datetime = DateTime::from_str("20240101T123456.7+1234").unwrap();
        assert_eq!(datetime.year, 2024);
        assert_eq!(datetime.month, Some(1));
        assert_eq!(datetime.day, Some(1));
        assert_eq!(datetime.hour, Some(12));
        assert_eq!(datetime.minute, Some(34));
        assert_eq!(datetime.second, Some(56));
        assert_eq!(datetime.decisecond, Some(7));
    }

    #[test]
    fn datetime_parse_fromstr_relative_negative() {
        let datetime = DateTime::from_str("20240101T123456.7-1234").unwrap();
        assert_eq!(datetime.year, 2024);
        assert_eq!(datetime.month, Some(1));
        assert_eq!(datetime.day, Some(1));
        assert_eq!(datetime.hour, Some(12));
        assert_eq!(datetime.minute, Some(34));
        assert_eq!(datetime.second, Some(56));
        assert_eq!(datetime.decisecond, Some(7));
    }
}
