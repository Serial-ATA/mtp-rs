use crate::communication::operation;
use super::io::PtpIo;
use crate::error::Result;

pub mod info;
pub mod property_describing;
pub mod storage;

pub trait Device: PtpIo {
	async fn open_session(&self) -> Result<()> {
		self.send_operation(operation::OpenSession::new(1))
		
		Ok(())
	}
}
