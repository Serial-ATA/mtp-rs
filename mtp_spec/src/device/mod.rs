use super::io::PtpIo;
use crate::communication::{operation, SessionId, TransactionId};
use deku::DekuContainerRead;

pub mod info;
pub mod property_describing;
pub mod storage;

pub trait Device: PtpIo {
	async fn get_device_info(
		&mut self,
		session_id: Option<SessionId>,
	) -> Result<info::DeviceInfo, <Self as PtpIo>::Error> {
		let transaction_id = if session_id.is_some() {
			self.next_transaction_id()
		} else {
			TransactionId::NONE
		};
		let response = self
			.send_operation(operation::GetDeviceInfo::new(
				transaction_id,
				session_id.unwrap_or(SessionId::NONE),
			))
			.await?;
		Ok(response.data)
	}

	async fn open_session(&mut self) -> Result<(), <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		let session_id = self.next_session_id();
		self.send_operation(operation::OpenSession::new(transaction_id, session_id))
			.await?;

		Ok(())
	}

	async fn close_session(&mut self, session_id: SessionId) -> Result<(), <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		self.send_operation(operation::CloseSession::new(transaction_id, session_id))
			.await?;

		Ok(())
	}
}
