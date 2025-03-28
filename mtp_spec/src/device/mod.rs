use super::io::PtpIo;
use crate::communication::operation::{
	CloseSession, DeleteObject, GetDeviceInfo, GetNumObjects, GetObject, GetObjectHandles,
	GetObjectInfo, GetStorageIDs, GetStorageInfo, GetThumb, InitiateCapture, OpenSession,
	SendObject, SendObjectInfo,
};
use crate::communication::response::Response;
use crate::communication::{SessionId, TransactionId};
use crate::device::storage::id::StorageId;
use crate::object::info::ObjectInfo;
use crate::object::types::{ObjectFormatCode, ObjectHandle};

use alloc::vec::Vec;
use deku::DekuContainerWrite;

pub mod info;
pub mod property_describing;
pub mod storage;

/// An MTP responder device
///
/// This is a convenience trait to perform operations on a responder without having to interact with
/// [`operations`] directly.
///
/// [`operations`]: crate::communication::operations
pub trait Device: PtpIo
where
	<Self as PtpIo>::Error: From<crate::error::MtpError>,
{
	/// Send a [`GetDeviceInfo`] operation
	async fn get_device_info(
		&mut self,
		session_id: Option<SessionId>,
	) -> Result<Response<GetDeviceInfo>, <Self as PtpIo>::Error> {
		let transaction_id = if session_id.is_some() {
			self.next_transaction_id()
		} else {
			TransactionId::NONE
		};

		self.send_operation(
			GetDeviceInfo::new(transaction_id, session_id.unwrap_or(SessionId::NONE)),
			None,
		)
		.await
	}

	/// Send a [`OpenSession`] operation
	async fn open_session(
		&mut self,
	) -> Result<(Response<OpenSession>, SessionId), <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		let session_id = self.next_session_id();
		self.send_operation(OpenSession::new(transaction_id, session_id), None)
			.await
			.map(|res| (res, session_id))
	}

	/// Send a [`CloseSession`] operation
	async fn close_session(
		&mut self,
		session_id: SessionId,
	) -> Result<Response<CloseSession>, <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		self.send_operation(CloseSession::new(transaction_id, session_id), None)
			.await
	}

	/// Send a [`GetStorageIDs`] operation
	async fn get_storage_ids(
		&mut self,
		session_id: SessionId,
	) -> Result<Response<GetStorageIDs>, <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		self.send_operation(GetStorageIDs::new(transaction_id, session_id), None)
			.await
	}

	/// Send a [`GetStorageInfo`] operation
	async fn get_storage_info(
		&mut self,
		session_id: SessionId,
		storage: StorageId,
	) -> Result<Response<GetStorageInfo>, <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		self.send_operation(
			GetStorageInfo::new(transaction_id, session_id, storage),
			None,
		)
		.await
	}

	/// Send a [`GetNumObjects`] operation
	async fn get_num_objects(
		&mut self,
		session_id: SessionId,
		storage: StorageId,
		format: Option<ObjectFormatCode>,
		parent: Option<ObjectHandle>,
	) -> Result<Response<GetNumObjects>, <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		self.send_operation(
			GetNumObjects::new(transaction_id, session_id, storage, format, parent),
			None,
		)
		.await
	}

	/// Send a [`GetObjectHandles`] operation
	async fn get_object_handles(
		&mut self,
		session_id: SessionId,
		storage: StorageId,
		format: Option<ObjectFormatCode>,
		parent: Option<ObjectHandle>,
	) -> Result<Response<GetObjectHandles>, <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		self.send_operation(
			GetObjectHandles::new(transaction_id, session_id, storage, format, parent),
			None,
		)
		.await
	}

	/// Send a [`GetObjectInfo`] operation
	async fn get_object_info(
		&mut self,
		session_id: SessionId,
		object: ObjectHandle,
	) -> Result<Response<GetObjectInfo>, <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		self.send_operation(GetObjectInfo::new(transaction_id, session_id, object), None)
			.await
	}

	/// Send a [`GetObject`] operation
	async fn get_object(
		&mut self,
		session_id: SessionId,
		object: ObjectHandle,
	) -> Result<Response<GetObject>, <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		self.send_operation(GetObject::new(transaction_id, session_id, object), None)
			.await
	}

	/// Send a [`GetThumb`] operation
	async fn get_thumb(
		&mut self,
		session_id: SessionId,
		object: ObjectHandle,
	) -> Result<Response<GetThumb>, <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		self.send_operation(GetThumb::new(transaction_id, session_id, object), None)
			.await
	}

	/// Send a [`DeleteObject`] operation
	async fn delete_object(
		&mut self,
		session_id: SessionId,
		object: ObjectHandle,
		format: Option<ObjectFormatCode>,
	) -> Result<Response<DeleteObject>, <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		self.send_operation(
			DeleteObject::new(
				transaction_id,
				session_id,
				object,
				format.unwrap_or(ObjectFormatCode::Unknown(0)),
			),
			None,
		)
		.await
	}

	/// Send a [`SendObjectInfo`] operation
	async fn send_object_info(
		&mut self,
		session_id: SessionId,
		storage: Option<StorageId>,
		parent: Option<ObjectHandle>,
		object_info: ObjectInfo,
	) -> Result<Response<SendObjectInfo>, <Self as PtpIo>::Error> {
		let encoded_object_info = object_info
			.to_bytes()
			.map_err(Into::<crate::error::MtpError>::into)?;

		let transaction_id = self.next_transaction_id();
		self.send_operation(
			SendObjectInfo::new(transaction_id, session_id, storage, parent),
			Some(encoded_object_info),
		)
		.await
	}

	/// Send a [`SendObject`] operation
	async fn send_object<T>(
		&mut self,
		session_id: SessionId,
		object_data: T,
	) -> Result<Response<SendObject>, <Self as PtpIo>::Error>
	where
		T: Into<Vec<u8>>,
	{
		let transaction_id = self.next_transaction_id();
		self.send_operation(
			SendObject::new(transaction_id, session_id),
			Some(object_data.into()),
		)
		.await
	}

	/// Send a [`InitiateCapture`] operation
	async fn initiate_capture(
		&mut self,
		session_id: SessionId,
		storage: Option<StorageId>,
		format: Option<ObjectFormatCode>,
	) -> Result<Response<InitiateCapture>, <Self as PtpIo>::Error> {
		let transaction_id = self.next_transaction_id();
		self.send_operation(
			InitiateCapture::new(transaction_id, session_id, storage, format),
			None,
		)
		.await
	}
}
