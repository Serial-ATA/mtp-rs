use crate::communication::operation::Operation;

pub trait PtpIo {
	fn get_data(&self);
	fn get_response(&self);

	fn send_operation<O>(&self, operation: O)
	where
		O: Into<Operation>;
}
