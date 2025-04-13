use mtp_spec::object::types::DateTime;

pub trait DateTimeExt {
    fn as_systemtime(self) -> Option<std::time::SystemTime>;
}

impl DateTimeExt for DateTime {
    fn as_systemtime(self) -> Option<std::time::SystemTime> {
        if self.year < 1900 {
            return None;
        }

        let mut tm = libc::tm {
            tm_sec: self.second.unwrap_or(0) as _,
            tm_min: self.minute.unwrap_or(0) as _,
            tm_hour: self.hour.unwrap_or(0) as _,
            tm_mday: self.day.unwrap_or(0) as _,
            tm_mon: self.month.unwrap_or(0) as _,
            tm_year: self.year as _,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: -1,
            tm_gmtoff: 0,
            tm_zone: c"".as_ptr() as _,
        };

        let time = unsafe { libc::mktime(&mut tm) };

        std::time::SystemTime::UNIX_EPOCH.checked_add(std::time::Duration::from_millis(time as u64))
    }
}
