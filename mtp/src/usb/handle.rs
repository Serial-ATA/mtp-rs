use super::error::UsbError;
use crate::error::Error;
use crate::usb::UsbDeviceFlags;
use std::io::Cursor;
use std::pin::Pin;

use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use deku::ctx::Endian;
use deku::reader::Reader;
use deku::{DekuContainerRead, DekuContainerWrite, DekuRead, DekuReader, DekuWrite};
use futures::{Stream, StreamExt};
use mtp_spec::communication::event::Event;
use mtp_spec::communication::operation::{DataDirection, DynOperation, SerializedOperation};
use mtp_spec::communication::response::{CODE_OK, Response, SuccessResponse};
use mtp_spec::communication::{SessionId, TransactionId};
use mtp_spec::device::{Device, PtpIo};
use mtp_spec::error::MtpError;
use nusb::transfer::{Queue, RequestBuffer};
use tokio::sync::Mutex;
use tokio::sync::broadcast::Sender;
use tokio_stream::wrappers::BroadcastStream;

#[derive(Copy, Clone, Debug)]
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
/// This implements [`Device`], which is how it should be interacted with primarily.
#[expect(dead_code)]
pub struct DeviceHandle {
    _device: nusb::Device,
    flags: UsbDeviceFlags,
    interface: nusb::Interface,
    endpoints: Endpoints,
    out_queue: Queue<Vec<u8>>,
    in_queue: Queue<RequestBuffer>,
    timeout: Duration,
    transaction_id: TransactionId,

    event_tx: Sender<Result<Event, Error>>,
    _events_task: tokio::task::JoinHandle<()>,
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

        let (event_tx, event_rx) = tokio::sync::broadcast::channel(100);
        let event_tx_clone = event_tx.clone();

        // TODO: Actually determine the endianness of the device
        let endian = Endian::Little;
        let events_task = tokio::task::spawn(async move {
            struct UsbEventStream {
                endian: Endian,
                endpoints: Endpoints,
                interrupt_queue: Arc<Mutex<Queue<RequestBuffer>>>,
            }

            impl Stream for UsbEventStream {
                type Item = Result<Event, crate::error::Error>;

                fn poll_next(
                    self: std::pin::Pin<&mut Self>,
                    cx: &mut std::task::Context<'_>,
                ) -> Poll<Option<Self::Item>> {
                    let buffer_size = self.endpoints.interrupt_buffer_size;

                    let Ok(mut interrupt_queue) = self.interrupt_queue.try_lock() else {
                        return Poll::Pending;
                    };

                    let pending = interrupt_queue.pending();
                    for _ in 0..(2usize.saturating_sub(pending)) {
                        interrupt_queue.submit(RequestBuffer::new(buffer_size));
                    }

                    match interrupt_queue.poll_next(cx) {
                        Poll::Ready(completion) => {
                            match UsbContainer::from_bytes((&completion.data, 0)) {
                                Ok((_, container)) => {
                                    let mut reader = Reader::new(Cursor::new(container.payload));

                                    Poll::Ready(Some(
                                        Event::from_reader_with_ctx(
                                            &mut reader,
                                            (self.endian, container.code),
                                        )
                                        .map_err(Into::into),
                                    ))
                                },
                                Err(e) => Poll::Ready(Some(Err(e.into()))),
                            }
                        },
                        Poll::Pending => Poll::Pending,
                    }
                }
            }

            let mut event_stream = UsbEventStream {
                endian,
                endpoints,
                interrupt_queue: Arc::new(Mutex::new(interrupt_queue)),
            };

            loop {
                match event_stream.next().await {
                    Some(event) => {
                        let _ = event_tx_clone.send(event);
                    },
                    None => {},
                }
            }
        });

        Self {
            _device: device,
            flags,
            interface,
            endpoints,
            out_queue,
            in_queue,
            timeout,
            transaction_id: TransactionId::new(1),

            event_tx,
            _events_task: events_task,
        }
    }
}

pub struct EventStream {
    recv: BroadcastStream<Result<Event, Error>>,
}

impl Stream for EventStream {
    type Item = Result<Event, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.recv.poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(event))) => Poll::Ready(Some(event)),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e.into()))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl PtpIo for DeviceHandle {
    type Error = crate::error::Error;
    type EventStream = EventStream;

    fn next_transaction_id(&mut self) -> TransactionId {
        let next = self.transaction_id;
        self.transaction_id = self.transaction_id.next();
        next
    }

    fn next_session_id(&mut self) -> SessionId {
        SessionId::new(1)
    }

    #[inline]
    fn endian(&self) -> Endian {
        // TODO: Needs to be provided from some global context
        Endian::Little
    }

    fn event_stream(&self) -> Self::EventStream {
        EventStream {
            recv: self.event_tx.subscribe().into(),
        }
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
        ) -> Result<(), crate::error::Error> {
            let data_len = data.len();

            let mut transfers = 1;
            queue.submit(data);
            if data_len % buffer_size == 0 {
                queue.submit(Vec::new());
                transfers += 1;
            }

            for _ in 0..transfers {
                let completion = tokio::time::timeout(timeout, queue.next_complete())
                    .await
                    .map_err(|_| UsbError::Timeout)?;
                completion.status.map_err(UsbError::from)?;
            }

            Ok(())
        }

        // Phase 1: Command
        let command_buf;
        {
            let op = operation.encode();
            log::debug!("Sending operation of type: {:#X}", op.code());

            let command_container = UsbContainer::new(
                ContainerType::Command,
                op.code(),
                op.transaction_id(),
                op.encode_parameters(self.endian())?,
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
                let data_phase = get_data_from_responder(self).await?;

                // Error was returned
                if data_phase.type_ == ContainerType::Response {
                    let err = O::decode_err(&data_phase.payload, self.endian(), data_phase.code)?;
                    return Ok(Response::Err(err));
                }

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
            let err = O::decode_err(&response.payload, self.endian(), response.code)?;
            return Ok(Response::Err(err));
        }

        match responder_data {
            Some(data) => {
                let data = O::decode_data(&data, self.endian())?;
                Ok(Response::Ok(SuccessResponse {
                    data,
                    transaction_id: response.transaction_id,
                }))
            },
            None => {
                // This case will only ever be hit for `()` anyway. The data we give it will
                // never be read.
                let data = O::decode_data(&[], self.endian())?;
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

async fn get_data_from_responder(
    handle: &mut DeviceHandle,
) -> Result<UsbContainer, crate::error::Error> {
    log::trace!("Attempting to get data from responder");

    let data_phase_raw = next_packet(handle).await?;
    let (_, mut data_phase) =
        UsbContainer::from_bytes((&data_phase_raw, 0)).map_err(MtpError::from)?;

    if data_phase.type_ == ContainerType::Response {
        if data_phase.code == CODE_OK {
            todo!("Error, responder didn't provide data")
        }

        return Ok(data_phase);
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
                return Err(UsbError::TooMuchData.into());
            };
            remaining = r;

            // Grow the first packet
            data_phase.payload.extend(data);
        }
    }

    Ok(data_phase)
}

async fn next_packet(handle: &mut DeviceHandle) -> Result<Vec<u8>, crate::error::Error> {
    let pending = handle.in_queue.pending();
    for _ in 0..(2usize.saturating_sub(pending)) {
        handle
            .in_queue
            .submit(RequestBuffer::new(handle.endpoints.bulk_in_buffer_size));
    }

    log::trace!("Waiting for next packet");
    let completion = tokio::time::timeout(handle.timeout, handle.in_queue.next_complete())
        .await
        .map_err(|_| UsbError::Timeout)?;
    completion.status.map_err(UsbError::from)?;

    Ok(completion.data)
}
