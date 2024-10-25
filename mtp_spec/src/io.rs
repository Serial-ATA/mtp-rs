use crate::communication::operation::Operation;
use crate::communication::{SessionId, TransactionId};
use crate::error::Result;

pub trait PtpIo {
	fn get_data(&self);
	fn get_response(&self);

	fn next_transaction_id(&mut self) -> TransactionId;
	fn next_session_id(&mut self) -> SessionId;

	async fn send_operation<O>(&self, operation: O) -> Result<()>
	where
		O: Into<Operation>;
}
