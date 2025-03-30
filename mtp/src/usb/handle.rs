use super::error::UsbError;
use crate::usb::UsbDeviceFlags;

use std::task::Poll;
use std::time::Duration;

use deku::{DekuContainerRead, DekuContainerWrite, DekuRead, DekuWrite};
use futures::Stream;
use mtp_spec::communication::event::Event;
use mtp_spec::communication::operation::{DataDirection, DynOperation, SerializedOperation};
use mtp_spec::communication::response::{CODE_OK, Response, ResponseFlags, SuccessResponse};
use mtp_spec::communication::{SessionId, TransactionId};
use mtp_spec::device::{Device, PtpIo};
use mtp_spec::error::MtpError;
use nusb::transfer::{Queue, RequestBuffer};

pub(super) struct Endpoints {
	pub(super) bulk_in: u8,
	pub(super) bulk_in_buffer_size: usize,
	pub(super) bulk_out: u8,
	pub(super) bulk_out_buffer_size: usize,
	pub(super) interrupt: u8,
	pub(super) interrupt_buffer_size: usize,
}

/// A handle to an open USB device
///
/// This implements [`Device`], which is how this should be interacted with primarily.
pub struct DeviceHandle {
	_device: nusb::Device,
	flags: UsbDeviceFlags,
	interface: nusb::Interface,
	endpoints: Endpoints,
	out_queue: Queue<Vec<u8>>,
	in_queue: Queue<RequestBuffer>,
	interrupt_queue: Queue<RequestBuffer>,
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
		let interrupt_queue = interface.interrupt_in_queue(endpoints.interrupt);
		Self {
			_device: device,
			flags,
			interface,
			endpoints,
			out_queue,
			in_queue,
			interrupt_queue,
			timeout,
			transaction_id: TransactionId::new(1),
		}
	}
}

impl Stream for DeviceHandle {
	type Item = Result<Event, crate::error::MtpError>;

	fn poll_next(
		mut self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Option<Self::Item>> {
		let buffer_size = self.endpoints.interrupt_buffer_size;

		let pending = self.interrupt_queue.pending();
		for _ in 0..(2usize.saturating_sub(pending)) {
			self.interrupt_queue.submit(RequestBuffer::new(buffer_size));
		}

		match self.interrupt_queue.poll_next(cx) {
			Poll::Ready(completion) => match Event::try_from(completion.data) {
				Ok(event) => Poll::Ready(Some(Ok(event))),
				Err(e) => Poll::Ready(Some(Err(e.into()))),
			},
			Poll::Pending => Poll::Pending,
		}
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

	async fn __send_operation<O>(
		&mut self,
		operation: O,
		data: Option<Vec<u8>>,
	) -> Result<Response<O>, Self::Error>
	where
		O: DynOperation,
		for<'a> SerializedOperation<'a>: From<&'a O>,
	{
		async fn send(
			data: Vec<u8>,
			queue: &mut Queue<Vec<u8>>,
			buffer_size: usize,
			timeout: Duration,
		) -> Result<(), crate::error::MtpError> {
			let data_len = data.len();

			let mut transfers = 1;
			queue.submit(data);
			if data_len % buffer_size == 0 {
				queue.submit(Vec::new());
				transfers += 1;
			}

			for _ in 0..transfers {
				let completion = tokio::time::timeout(timeout, queue.next_complete()).await?;
				completion.status.map_err(UsbError::from)?;
			}

			Ok(())
		}

		// Phase 1: Command
		let command_buf;
		{
			let op = operation.encode();
			log::debug!("Sending operation of type: {}", op.code());

			let command_container = UsbContainer::new(
				ContainerType::Command,
				op.code(),
				op.transaction_id(),
				op.encode_parameters()?,
			);
			command_buf = command_container
				.to_bytes()
				.map_err(Into::<MtpError>::into)?;
		}

		send(
			command_buf,
			&mut self.out_queue,
			self.endpoints.bulk_out_buffer_size,
			self.timeout,
		)
		.await?;

		// Phase 2: Data (if applicable)
		let mut responder_data = None;
		match O::DATA_DIRECTION {
			Some(DataDirection::InitiatorToResponder) => {
				send(
					data.expect("data should exist"),
					&mut self.out_queue,
					self.endpoints.bulk_out_buffer_size,
					self.timeout,
				)
				.await?;
			},
			Some(DataDirection::ResponderToInitiator) => {
				let data_phase =
					get_data_from_responder::<<O as DynOperation>::Response>(self).await?;
				responder_data = Some(data_phase.payload);
			},
			// No data phase
			_ => {},
		}

		// Phase 3: Response
		log::debug!("Attempting to get response");

		let response_raw = next_packet(self).await?;

		let (_, response_container) =
			UsbContainer::from_bytes((&response_raw, 0)).map_err(MtpError::from)?;
		let response = response_container;

		if response.code != CODE_OK {
			let err = O::decode_err(&response.payload, response.code)?;
			return Ok(Response::Err(err));
		}

		match responder_data {
			Some(data) => {
				let data = O::decode_data(&data)?;
				Ok(Response::Ok(SuccessResponse {
					data,
					transaction_id: response.transaction_id,
				}))
			},
			None => {
				// This case will only ever be hit for `()` anyway. The data we give it will
				// never be read.
				let data = O::decode_data(&[])?;
				Ok(Response::Ok(SuccessResponse {
					data,
					transaction_id: response.transaction_id,
				}))
			},
		}
	}
}

impl Device for DeviceHandle {}

#[derive(PartialEq, Debug, Copy, Clone, DekuRead, DekuWrite)]
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

#[derive(DekuRead, DekuWrite)]
struct UsbContainer {
	#[deku(assert = "*length >= USB_CONTAINER_HEADER_SIZE", endian = "little")]
	length: u32,
	type_: ContainerType,
	#[deku(endian = "little")]
	code: u16,
	transaction_id: TransactionId,
	#[deku(read_all)]
	payload: Vec<u8>,
}

impl UsbContainer {
	fn new(ty: ContainerType, code: u16, transaction_id: TransactionId, payload: Vec<u8>) -> Self {
		Self {
			length: USB_CONTAINER_HEADER_SIZE + payload.len() as u32,
			type_: ty,
			code,
			transaction_id,
			payload,
		}
	}
}

async fn get_data_from_responder<T: ResponseFlags>(
	handle: &mut DeviceHandle,
) -> Result<UsbContainer, crate::error::MtpError> {
	log::trace!("Attempting to get data from responder");

	let data_phase_raw = next_packet(handle).await?;
	let (_, mut data_phase) =
		UsbContainer::from_bytes((&data_phase_raw, 0)).map_err(MtpError::from)?;

	if data_phase.type_ != ContainerType::Data {
		todo!("Error, responder didn't provide data")
	}

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
	let len_without_header = data_phase.length - USB_CONTAINER_HEADER_SIZE;
	if len_without_header > data_phase.payload.len() as u32 {
		let mut remaining = len_without_header - data_phase.payload.len() as u32;

		log::trace!(
			"Device is buffering the data, received {}/{} bytes",
			data_phase.payload.len(),
			len_without_header
		);

		while remaining > 0 {
			let data = next_packet(handle).await?;
			let Some(r) = remaining.checked_sub(data.len() as u32) else {
				todo!("Error, device sent too much data");
			};
			remaining = r;

			// Grow the first packet
			data_phase.payload.extend(data);
		}
	}

	Ok(data_phase)
}

async fn next_packet(handle: &mut DeviceHandle) -> Result<Vec<u8>, crate::error::MtpError> {
	let pending = handle.in_queue.pending();
	for _ in 0..(2usize.saturating_sub(pending)) {
		handle
			.in_queue
			.submit(RequestBuffer::new(handle.endpoints.bulk_in_buffer_size));
	}

	log::trace!("Waiting for next packet");
	let completion = tokio::time::timeout(handle.timeout, handle.in_queue.next_complete()).await?;
	completion.status.map_err(UsbError::from)?;

	Ok(completion.data)
}
