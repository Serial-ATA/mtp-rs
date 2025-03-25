use super::error::UsbError;
use crate::usb::UsbDeviceFlags;

use deku::{DekuContainerRead, DekuRead};
use mtp_spec::communication::operation::{Operation, SerializedOperation};
use mtp_spec::communication::response::{Response, ResponseFlags, SuccessResponse, CODE_OK};
use mtp_spec::communication::{SessionId, TransactionId};
use mtp_spec::device::Device;
use mtp_spec::error::MtpError;
use mtp_spec::io::PtpIo;
use nusb::transfer::{Queue, RequestBuffer};
use std::io::{Cursor, Write};
use std::time::Duration;

pub(super) struct Endpoints {
	pub(super) bulk_in: u8,
	pub(super) bulk_in_buffer_size: usize,
	pub(super) bulk_out: u8,
	pub(super) bulk_out_buffer_size: usize,
	pub(super) interrupt: u8,
}

pub struct DeviceHandle {
	device: nusb::Device,
	flags: UsbDeviceFlags,
	interface: nusb::Interface,
	endpoints: Endpoints,
	out_queue: Queue<Vec<u8>>,
	in_queue: Queue<RequestBuffer>,
	timeout: Duration,
	transaction_id: TransactionId,
}

impl DeviceHandle {
	pub(super) fn new(
		device: nusb::Device,
		flags: UsbDeviceFlags,
		interface: nusb::Interface,
		endpoints: Endpoints,
	) -> Self {
		let timeout = if flags.contains(UsbDeviceFlags::LONG_TIMEOUT) {
			Duration::from_millis(60000)
		} else {
			Duration::from_millis(20000)
		};

		let out_queue = interface.bulk_out_queue(endpoints.bulk_out);
		let in_queue = interface.bulk_in_queue(endpoints.bulk_in);
		Self {
			device,
			flags,
			interface,
			endpoints,
			out_queue,
			in_queue,
			timeout,
			transaction_id: TransactionId::new(1),
		}
	}
}

struct OperationSerializer<'a>(SerializedOperation<'a>);

impl OperationSerializer<'_> {
	fn to_bytes(&self) -> Result<Vec<u8>, crate::error::MtpError> {
		const CONTAINER_TYPE: u16 = 0x0001; // Command
		const HEADER_SIZE: u32 = (size_of::<u32>() + size_of::<u16>()) as u32;

		let container_length = HEADER_SIZE + self.0.size() as u32;

		let buf = vec![0; container_length as usize];
		let mut cursor = Cursor::new(buf);

		cursor.write_all(&container_length.to_le_bytes())?;
		cursor.write_all(&CONTAINER_TYPE.to_le_bytes())?;
		cursor.write_all(&self.0.to_bytes()?)?;

		Ok(cursor.into_inner())
	}
}

impl PtpIo for DeviceHandle {
	type Error = crate::error::MtpError;

	fn next_transaction_id(&mut self) -> TransactionId {
		let next = self.transaction_id;
		self.transaction_id = self.transaction_id.next();
		next
	}

	fn next_session_id(&mut self) -> SessionId {
		SessionId::new(1)
	}

	async fn send_operation<O>(&mut self, operation: O) -> Result<Response<O>, Self::Error>
	where
		O: Operation,
		for<'a> SerializedOperation<'a>: From<&'a O>,
	{
		let buf;
		{
			let op = OperationSerializer(operation.encode());
			buf = op.to_bytes()?;
		}

		let buf_len = buf.len();

		self.out_queue.submit(buf);

		if buf_len % self.endpoints.bulk_out_buffer_size == 0 {
			self.out_queue.submit(Vec::new());
		}

		let completion = tokio::time::timeout(self.timeout, self.out_queue.next_complete()).await?;
		completion.status.map_err(UsbError::from)?;

		let phases = get_response::<<O as Operation>::Response>(self).await?;

		if phases.response.code != CODE_OK {
			let err = O::decode_err(&phases.response.data, phases.response.code)?;
			return Ok(Response::Err(err));
		}

		match phases.data {
			Some(data) => {
				let data = O::decode_data(&data.data)?;
				Ok(Response::Ok(SuccessResponse {
					data,
					transaction_id: phases.response.transaction_id,
				}))
			},
			None => {
				// This case will only ever be hit for `()` anyway. The data we give it will
				// never be read.
				let data = O::decode_data(&[])?;
				Ok(Response::Ok(SuccessResponse {
					data,
					transaction_id: phases.response.transaction_id,
				}))
			},
		}
	}
}

impl Device for DeviceHandle {}

#[derive(DekuRead, PartialEq, Debug, Copy, Clone)]
#[repr(u16)]
#[deku(id_type = "u16", endian = "little")]
enum ContainerType {
	Undefined = 0x0000,
	Command = 0x0001,
	Data = 0x0002,
	Response = 0x0003,
	Event = 0x0004,
}

const USB_CONTAINER_HEADER_SIZE: u32 =
	(size_of::<u32>() + size_of::<u16>() + size_of::<u16>() + size_of::<TransactionId>()) as u32;

#[derive(DekuRead)]
struct UsbContainer {
	#[deku(assert = "*length >= USB_CONTAINER_HEADER_SIZE", endian = "little")]
	length: u32,
	type_: ContainerType,
	#[deku(endian = "little")]
	code: u16,
	transaction_id: TransactionId,
	#[deku(read_all)]
	data: Vec<u8>,
}

struct Phases {
	data: Option<UsbContainer>,
	response: UsbContainer,
}

async fn get_response<T: ResponseFlags>(
	handle: &mut DeviceHandle,
) -> Result<Phases, crate::error::MtpError> {
	let next_phase = next_packet(handle).await?;

	// From the MTP 1.1 spec appendix: Splitting the Header and Data during the Data Phase
	//
	// "An MTP responder may overcome this by separating the header from the payload and
	// sending/receiving it in a short packet preceding the payload. Devices that choose to do
	// this must always manage these packets consistently. That is, all data phases (all USB data
	// transfers where the ContainerType = 0x0002) must have a single packet containing 12
	// bytes, which has only the header which is followed by the payload beginning with a new
	// packet."
	//
	// In short, we may get a single header packet before the real data.
	if T::EXPECTS_DATA && next_phase.data.is_empty() && next_phase.type_ == ContainerType::Response
	{
		let data_container = next_packet(handle).await?;

		return Ok(Phases {
			data: Some(data_container),
			response: next_phase,
		});
	}

	if next_phase.type_ != ContainerType::Data {
		if T::EXPECTS_DATA && next_phase.code == CODE_OK {
			todo!("Error, device didn't provide data");
		}

		return Ok(Phases {
			data: None,
			response: next_phase,
		});
	}

	let response = next_packet(handle).await?;
	Ok(Phases {
		data: Some(next_phase),
		response,
	})
}

async fn next_packet(handle: &mut DeviceHandle) -> Result<UsbContainer, crate::error::MtpError> {
	let pending = handle.in_queue.pending();
	for _ in 0..(2usize.saturating_sub(pending)) {
		handle
			.in_queue
			.submit(RequestBuffer::new(handle.endpoints.bulk_in_buffer_size));
	}

	let completion = tokio::time::timeout(handle.timeout, handle.in_queue.next_complete()).await?;
	completion.status.map_err(UsbError::from)?;

	let (_, container) = UsbContainer::from_bytes((&completion.data, 0)).map_err(MtpError::from)?;
	Ok(container)
}
