use super::io::PtpIo;
use crate::communication::operation::{
	CloseSession, CopyObject, DeleteObject, DevicePropCode, FormatStore, GetDeviceInfo,
	GetDevicePropDesc, GetDevicePropValue, GetNumObjects, GetObject, GetObjectHandles,
	GetObjectInfo, GetObjectPropDesc, GetObjectPropValue, GetObjectPropsSupported,
	GetPartialObject, GetStorageIDs, GetStorageInfo, GetThumb, InitiateCapture,
	InitiateOpenCapture, MoveObject, OpenSession, PowerDown, ResetDevice, ResetDevicePropValue,
	SelfTest, SelfTestType, SendObject, SendObjectInfo, SetDevicePropValue, SetObjectProtection,
	TerminateOpenCapture,
};
use crate::communication::response::Response;
use crate::communication::{SessionId, TransactionId};
use crate::device::storage::id::StorageId;
use crate::object::info::{ObjectInfo, ProtectionStatus};
use crate::object::types::{ObjectFormatCode, ObjectHandle};

use crate::device::storage::info::FilesystemType;
use crate::object::types::properties::{ObjectProperty, ObjectPropertyCode};
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
	fn get_device_info(
		&mut self,
		session_id: Option<SessionId>,
	) -> impl Future<Output = Result<Response<GetDeviceInfo>, <Self as PtpIo>::Error>> {
		async move {
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
	}

	/// Send a [`OpenSession`] operation
	fn open_session(
		&mut self,
	) -> impl Future<Output = Result<(Response<OpenSession>, SessionId), <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			let session_id = self.next_session_id();
			self.send_operation(OpenSession::new(transaction_id, session_id), None)
				.await
				.map(|res| (res, session_id))
		}
	}

	/// Send a [`CloseSession`] operation
	fn close_session(
		&mut self,
		session_id: SessionId,
	) -> impl Future<Output = Result<Response<CloseSession>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(CloseSession::new(transaction_id, session_id), None)
				.await
		}
	}

	/// Send a [`GetStorageIDs`] operation
	fn get_storage_ids(
		&mut self,
		session_id: SessionId,
	) -> impl Future<Output = Result<Response<GetStorageIDs>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(GetStorageIDs::new(transaction_id, session_id), None)
				.await
		}
	}

	/// Send a [`GetStorageInfo`] operation
	fn get_storage_info(
		&mut self,
		session_id: SessionId,
		storage: StorageId,
	) -> impl Future<Output = Result<Response<GetStorageInfo>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				GetStorageInfo::new(transaction_id, session_id, storage),
				None,
			)
			.await
		}
	}

	/// Send a [`GetNumObjects`] operation
	fn get_num_objects(
		&mut self,
		session_id: SessionId,
		storage: StorageId,
		format: Option<ObjectFormatCode>,
		parent: Option<ObjectHandle>,
	) -> impl Future<Output = Result<Response<GetNumObjects>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				GetNumObjects::new(transaction_id, session_id, storage, format, parent),
				None,
			)
			.await
		}
	}

	/// Send a [`GetObjectHandles`] operation
	fn get_object_handles(
		&mut self,
		session_id: SessionId,
		storage: StorageId,
		format: Option<ObjectFormatCode>,
		parent: Option<ObjectHandle>,
	) -> impl Future<Output = Result<Response<GetObjectHandles>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				GetObjectHandles::new(transaction_id, session_id, storage, format, parent),
				None,
			)
			.await
		}
	}

	/// Send a [`GetObjectInfo`] operation
	fn get_object_info(
		&mut self,
		session_id: SessionId,
		object: ObjectHandle,
	) -> impl Future<Output = Result<Response<GetObjectInfo>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(GetObjectInfo::new(transaction_id, session_id, object), None)
				.await
		}
	}

	/// Send a [`GetObject`] operation
	fn get_object(
		&mut self,
		session_id: SessionId,
		object: ObjectHandle,
	) -> impl Future<Output = Result<Response<GetObject>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(GetObject::new(transaction_id, session_id, object), None)
				.await
		}
	}

	/// Send a [`GetThumb`] operation
	fn get_thumb(
		&mut self,
		session_id: SessionId,
		object: ObjectHandle,
	) -> impl Future<Output = Result<Response<GetThumb>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(GetThumb::new(transaction_id, session_id, object), None)
				.await
		}
	}

	/// Send a [`DeleteObject`] operation
	fn delete_object(
		&mut self,
		session_id: SessionId,
		object: ObjectHandle,
		format: Option<ObjectFormatCode>,
	) -> impl Future<Output = Result<Response<DeleteObject>, <Self as PtpIo>::Error>> {
		async move {
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
	}

	/// Send a [`SendObjectInfo`] operation
	fn send_object_info(
		&mut self,
		session_id: SessionId,
		storage: Option<StorageId>,
		parent: Option<ObjectHandle>,
		object_info: ObjectInfo,
	) -> impl Future<Output = Result<Response<SendObjectInfo>, <Self as PtpIo>::Error>> {
		async move {
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
	}

	/// Send a [`SendObject`] operation
	fn send_object<T>(
		&mut self,
		session_id: SessionId,
		object_data: T,
	) -> impl Future<Output = Result<Response<SendObject>, <Self as PtpIo>::Error>>
	where
		T: Into<Vec<u8>>,
	{
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				SendObject::new(transaction_id, session_id),
				Some(object_data.into()),
			)
			.await
		}
	}

	/// Send a [`InitiateCapture`] operation
	fn initiate_capture(
		&mut self,
		session_id: SessionId,
		storage: Option<StorageId>,
		format: Option<ObjectFormatCode>,
	) -> impl Future<Output = Result<Response<InitiateCapture>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				InitiateCapture::new(transaction_id, session_id, storage, format),
				None,
			)
			.await
		}
	}

	/// Send a [`FormatStore`] operation
	fn format_store(
		&mut self,
		session_id: SessionId,
		storage: StorageId,
		fs: FilesystemType,
	) -> impl Future<Output = Result<Response<FormatStore>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				FormatStore::new(transaction_id, session_id, storage, fs),
				None,
			)
			.await
		}
	}

	/// Send a [`ResetDevice`] operation
	fn reset_device(
		&mut self,
		session_id: SessionId,
	) -> impl Future<Output = Result<Response<ResetDevice>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(ResetDevice::new(transaction_id, session_id), None)
				.await
		}
	}

	/// Send a [`SelfTest`] operation
	fn self_test(
		&mut self,
		session_id: SessionId,
		test_type: SelfTestType,
	) -> impl Future<Output = Result<Response<SelfTest>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(SelfTest::new(transaction_id, session_id, test_type), None)
				.await
		}
	}

	/// Send a [`SetObjectProtection`] operation
	fn set_object_protection(
		&mut self,
		session_id: SessionId,
		object: ObjectHandle,
		status: ProtectionStatus,
	) -> impl Future<Output = Result<Response<SetObjectProtection>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				SetObjectProtection::new(transaction_id, session_id, object, status),
				None,
			)
			.await
		}
	}

	/// Send a [`PowerDown`] operation
	fn power_down(
		&mut self,
		session_id: SessionId,
	) -> impl Future<Output = Result<Response<PowerDown>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(PowerDown::new(transaction_id, session_id), None)
				.await
		}
	}

	/// Send a [`GetDevicePropDesc`] operation
	fn get_device_prop_desc(
		&mut self,
		session_id: SessionId,
		code: DevicePropCode,
	) -> impl Future<Output = Result<Response<GetDevicePropDesc>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				GetDevicePropDesc::new(transaction_id, session_id, code),
				None,
			)
			.await
		}
	}

	/// Send a [`GetDevicePropValue`] operation
	fn get_device_prop_value(
		&mut self,
		session_id: SessionId,
		code: DevicePropCode,
	) -> impl Future<Output = Result<Response<GetDevicePropValue>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				GetDevicePropValue::new(transaction_id, session_id, code),
				None,
			)
			.await
		}
	}

	/// Send a [`SetDevicePropValue`] operation
	fn set_device_prop_value(
		&mut self,
		session_id: SessionId,
		code: DevicePropCode,
		value: Vec<u8>, // TODO
	) -> impl Future<Output = Result<Response<SetDevicePropValue>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				SetDevicePropValue::new(transaction_id, session_id, code),
				None,
			)
			.await
		}
	}

	/// Send a [`ResetDevicePropValue`] operation
	fn reset_device_prop_value(
		&mut self,
		session_id: SessionId,
		code: DevicePropCode,
	) -> impl Future<Output = Result<Response<ResetDevicePropValue>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				ResetDevicePropValue::new(transaction_id, session_id, code),
				None,
			)
			.await
		}
	}

	/// Send a [`TerminateOpenCapture`] operation
	fn terminate_open_capture(
		&mut self,
		session_id: SessionId,
		transaction_id: TransactionId,
	) -> impl Future<Output = Result<Response<TerminateOpenCapture>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				TerminateOpenCapture::new(transaction_id, session_id, transaction_id),
				None,
			)
			.await
		}
	}

	/// Send a [`MoveObject`] operation
	fn move_object(
		&mut self,
		session_id: SessionId,
		object: ObjectHandle,
		storage: StorageId,
		parent: Option<ObjectHandle>,
	) -> impl Future<Output = Result<Response<MoveObject>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				MoveObject::new(transaction_id, session_id, object, storage, parent),
				None,
			)
			.await
		}
	}

	/// Send a [`CopyObject`] operation
	fn copy_object(
		&mut self,
		session_id: SessionId,
		object: ObjectHandle,
		storage: StorageId,
		parent: Option<ObjectHandle>,
	) -> impl Future<Output = Result<Response<CopyObject>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				CopyObject::new(transaction_id, session_id, object, storage, parent),
				None,
			)
			.await
		}
	}

	/// Send a [`GetPartialObject`] operation
	fn get_partial_object(
		&mut self,
		session_id: SessionId,
		object: ObjectHandle,
		offset: u32,
		len: u32,
	) -> impl Future<Output = Result<Response<GetPartialObject>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				GetPartialObject::new(transaction_id, session_id, object, offset, len),
				None,
			)
			.await
		}
	}

	/// Send an [`InitiateOpenCapture`] operation
	fn initiate_open_capture(
		&mut self,
		session_id: SessionId,
		storage: Option<StorageId>,
		format: Option<ObjectFormatCode>,
	) -> impl Future<Output = Result<Response<InitiateOpenCapture>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				InitiateOpenCapture::new(transaction_id, session_id, storage, format),
				None,
			)
			.await
		}
	}

	/// Send a [`GetObjectPropsSupported`] operation
	fn get_object_props_supported(
		&mut self,
		session_id: SessionId,
		format: ObjectFormatCode,
	) -> impl Future<Output = Result<Response<GetObjectPropsSupported>, <Self as PtpIo>::Error>> {
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				GetObjectPropsSupported::new(transaction_id, session_id, format),
				None,
			)
			.await
		}
	}

	/// Send a [`GetObjectPropDesc`] operation
	fn get_object_prop_desc<T>(
		&mut self,
		session_id: SessionId,
		format: ObjectFormatCode,
	) -> impl Future<Output = Result<Response<GetObjectPropDesc<T>>, <Self as PtpIo>::Error>>
	where
		T: ObjectProperty,
	{
		async move {
			let transaction_id = self.next_transaction_id();
			self.send_operation(
				GetObjectPropDesc::<T>::new(transaction_id, session_id, format),
				None,
			)
			.await
		}
	}

	// TODO
	// /// Send a [`GetObjectPropValue`] operation
	// fn get_object_prop_value<T>(
	// 	&mut self,
	// 	session_id: SessionId,
	// 	object: ObjectHandle,
	// ) -> impl Future<Output = Result<Response<GetObjectPropValue<T>>, <Self as PtpIo>::Error>>
	// where
	// 	T: ObjectProperty,
	// {
	// 	async move {
	// 		let transaction_id = self.next_transaction_id();
	// 		self.send_operation(
	// 			GetObjectPropValue::<T>::new(transaction_id, session_id, object, T::CODE),
	// 			None,
	// 		)
	// 		.await
	// 	}
	// }
}
