use super::io::PtpIo;
use crate::communication::{operation, SessionId};
use crate::error::Result;

pub mod info;
pub mod property_describing;
pub mod storage;

pub trait Device: PtpIo {
	async fn open_session(&mut self) -> Result<()> {
		let transaction_id = self.next_transaction_id();
		let session_id = self.next_session_id();
		self.send_operation(operation::OpenSession::new(transaction_id, session_id))
			.await?;

		Ok(())
	}

	async fn close_session(&mut self, session_id: SessionId) -> Result<()> {
		let transaction_id = self.next_transaction_id();
		self.send_operation(operation::CloseSession::new(transaction_id, session_id))
			.await?;

		Ok(())
	}
}
