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

impl DateTime {
	fn validate(self) -> bool {
		fn verify_field(field: Option<u8>, limit: u8, parent: Option<u8>) -> bool {
			if let Some(field) = field {
				return parent.is_some() && field <= limit;
			}
			return true; // Field does not exist, so it's valid
		}

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
