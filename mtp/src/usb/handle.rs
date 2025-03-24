use super::error::UsbError;
use crate::usb::UsbDeviceFlags;

use std::io::{Cursor, Write};
use std::time::Duration;

use mtp_spec::communication::operation::{Operation, SerializedOperation};
use mtp_spec::communication::{SessionId, TransactionId};
use mtp_spec::device::Device;
use mtp_spec::io::PtpIo;
use nusb::transfer::{Queue, RequestBuffer};

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

	fn get_data(&self) {
		todo!()
	}

	async fn get_response(&mut self) -> Result<Vec<u8>, Self::Error> {
		let pending = self.in_queue.pending();
		for _ in 0..(4usize.saturating_sub(pending)) {
			self.in_queue
				.submit(RequestBuffer::new(self.endpoints.bulk_in_buffer_size));
		}

		let completion = tokio::time::timeout(self.timeout, self.in_queue.next_complete()).await?;
		completion.status.map_err(UsbError::from)?;

		Ok(completion.data)
	}

	fn next_transaction_id(&mut self) -> TransactionId {
		TransactionId::new(1)
	}

	fn next_session_id(&mut self) -> SessionId {
		SessionId::new(1)
	}

	async fn send_operation<O>(
		&mut self,
		operation: O,
	) -> Result<<O as Operation>::Response, Self::Error>
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
		dbg!(completion.data);

		let data = self.get_response().await?;
		let response = O::decode_response(&data)?;
		Ok(response)
	}
}

impl Device for DeviceHandle {}
