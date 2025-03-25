use super::io::PtpIo;
use crate::communication::operation::{CloseSession, GetDeviceInfo, OpenSession};
use crate::communication::response::Response;
use crate::communication::{operation, SessionId, TransactionId};

pub mod info;
pub mod property_describing;
pub mod storage;

pub trait Device: PtpIo {
	async fn get_device_info(
		&mut self,
		session_id: Option<SessionId>,
	) -> Result<Response<GetDeviceInfo>, <Self as PtpIo>::Error> {
		let transaction_id = if session_id.is_some() {
			self.next_transaction_id()
		} else {
			TransactionId::NONE
		};

		self.send_operation(GetDeviceInfo::new(
			transaction_id,
			session_id.unwrap_or(SessionId::NONE),
		))
		.await
	}

	async fn open_session(
		&mut self,
	) -> Result<(Response<OpenSession>, SessionId), <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		let session_id = self.next_session_id();
		self.send_operation(OpenSession::new(transaction_id, session_id))
			.await
			.map(|res| (res, session_id))
	}

	async fn close_session(
		&mut self,
		session_id: SessionId,
	) -> Result<Response<CloseSession>, <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		self.send_operation(CloseSession::new(transaction_id, session_id))
			.await
	}
}
