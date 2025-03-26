use crate::communication::operation::{DynOperation, Operation, SerializedOperation};
use crate::communication::{SessionId, TransactionId};

use crate::communication::response::Response;

pub trait PtpIo {
	type Error: core::error::Error;

	/// Get the next transaction ID
	///
	/// While not required, the resulting ID **should** be used in the next operation, to keep the
	/// transaction IDs monotonically increasing.
	#[must_use]
	fn next_transaction_id(&mut self) -> TransactionId;
	/// Get the next session ID
	///
	/// While not required, the resulting ID **should** be used in the next operation, to keep the
	/// session IDs monotonically increasing.
	#[must_use]
	fn next_session_id(&mut self) -> SessionId;

	/// Send the operation to the device and wait for a response
	async fn send_operation<O>(&mut self, operation: O) -> Result<Response<O>, Self::Error>
	where
		O: DynOperation,
		for<'a> SerializedOperation<'a>: From<&'a O>;
}
