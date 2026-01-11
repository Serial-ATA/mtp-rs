use super::error::{Error, UsbError};
use crate::communication::event::Event;
use crate::communication::operation::{DataDirection, DynOperation, SerializedOperation};
use crate::communication::response::{CODE_OK, Response, SuccessResponse};
use crate::communication::{SessionId, TransactionId};
use crate::device::{Device, DeviceFlags, PtpIo};
use crate::error::MtpError;

use std::io::Cursor;
use std::ops::BitOr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use bitflags::{Flag, Flags};
use deku::ctx::Endian;
use deku::reader::Reader;
use deku::writer::Writer;
use deku::{
    DekuContainerRead, DekuContainerWrite, DekuError, DekuRead, DekuReader, DekuWrite, DekuWriter,
};
use futures::{Stream, StreamExt};
use nusb::Endpoint;
use nusb::transfer::{Buffer, Bulk, In, Interrupt, Out};
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

const BASE_DEVICE_FLAGS_BITS: u32 = DeviceFlags::all().bits().count_ones();

bitflags::bitflags! {
    /// USB-specific device flags
    #[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
    pub struct UsbDeviceFlags: u32 {
        const NO_RELEASE_INTERFACE = 1 << BASE_DEVICE_FLAGS_BITS;
        const UNLOAD_DRIVER = 1 << (BASE_DEVICE_FLAGS_BITS + 1);
        /// The device requires an explicit USB reset after each connection
        const FORCE_RESET_ON_CLOSE = 3 << (BASE_DEVICE_FLAGS_BITS + 2);
        const ALWAYS_PROBE_DESCRIPTOR = 4 << (BASE_DEVICE_FLAGS_BITS + 3);
        const NO_ZERO_READS = 5 << (BASE_DEVICE_FLAGS_BITS + 4);
        /// The device provides garbage opcodes and [`TransactionId`]s in its packet headers
        const IGNORE_HEADER_ERRORS = 6 << (BASE_DEVICE_FLAGS_BITS + 5);
    }
}

/// A flag set combining [`DeviceFlags`] and [`UsbDeviceFlags`]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct UsbDeviceFlagSet {
    /// Base MTP device flags
    pub base: DeviceFlags,
    /// USB-specific device flags
    pub usb: UsbDeviceFlags,
}

impl UsbDeviceFlagSet {
    /// Bugs on all devices using the Android MTP stack
    pub const ANDROID_BUGS: Self = Self {
        base: DeviceFlags::from_bits_truncate(
            DeviceFlags::BROKEN_MTP_GET_OBJECT_PROP_LIST_ALL.bits()
                | DeviceFlags::BROKEN_SET_OBJECT_PROP_LIST.bits()
                | DeviceFlags::BROKEN_SEND_OBJECT_PROP_LIST.bits()
                | DeviceFlags::LONG_TIMEOUT.bits(),
        ),
        usb: UsbDeviceFlags::from_bits_truncate(
            UsbDeviceFlags::UNLOAD_DRIVER.bits() | UsbDeviceFlags::FORCE_RESET_ON_CLOSE.bits(),
        ),
    };

    /// Bugs on SONY NWZ Walkman players
    pub const SONY_NWZ_BUGS: Self = Self {
        base: DeviceFlags::from_bits_truncate(
            DeviceFlags::BROKEN_MTP_GET_OBJECT_PROP_LIST_ALL.bits()
                | DeviceFlags::UNIQUE_FILENAMES.bits(),
        ),
        usb: UsbDeviceFlags::from_bits_truncate(
            UsbDeviceFlags::UNLOAD_DRIVER.bits() | UsbDeviceFlags::FORCE_RESET_ON_CLOSE.bits(),
        ),
    };

    /// Bugs on devices using the Aricent MTP stack
    pub const ARICENT_BUGS: Self = Self {
        base: DeviceFlags::from_bits_truncate(
            DeviceFlags::BROKEN_SEND_OBJECT_PROP_LIST.bits()
                | DeviceFlags::BROKEN_MTP_GET_OBJECT_PROP_LIST.bits(),
        ),
        usb: UsbDeviceFlags::IGNORE_HEADER_ERRORS,
    };
}

impl Flags for UsbDeviceFlagSet {
    const FLAGS: &'static [Flag<Self>] = &[
        Flag::new("ANDROID_BUGS", Self::ANDROID_BUGS),
        Flag::new("SONY_NWZ_BUGS", Self::SONY_NWZ_BUGS),
        Flag::new("ARICENT_BUGS", Self::ARICENT_BUGS),
    ];
    type Bits = u32;

    fn all() -> Self {
        Self {
            base: DeviceFlags::all(),
            usb: UsbDeviceFlags::all(),
        }
    }

    fn bits(&self) -> Self::Bits {
        self.base.bits() | self.usb.bits()
    }

    fn from_bits_retain(bits: Self::Bits) -> Self {
        Self {
            base: DeviceFlags::from_bits_retain(bits),
            usb: UsbDeviceFlags::from_bits_retain(bits),
        }
    }
}

impl BitOr<DeviceFlags> for UsbDeviceFlagSet {
    type Output = Self;

    fn bitor(self, rhs: DeviceFlags) -> Self::Output {
        Self {
            base: self.base | rhs,
            usb: self.usb,
        }
    }
}

impl BitOr<UsbDeviceFlags> for UsbDeviceFlagSet {
    type Output = Self;

    fn bitor(self, rhs: UsbDeviceFlags) -> Self::Output {
        Self {
            base: self.base,
            usb: self.usb | rhs,
        }
    }
}

impl BitOr<UsbDeviceFlagSet> for UsbDeviceFlagSet {
    type Output = Self;

    fn bitor(self, rhs: UsbDeviceFlagSet) -> Self::Output {
        Self {
            base: self.base | rhs.base,
            usb: self.usb | rhs.usb,
        }
    }
}

/// A handle to an open USB device
///
/// This implements [`Device`], which is how it should be interacted with primarily.
#[expect(dead_code)]
pub struct DeviceHandle {
    _device: nusb::Device,
    endian: Endian,
    flags: UsbDeviceFlagSet,
    interface: nusb::Interface,
    endpoints: Endpoints,
    out_queue: Endpoint<Bulk, Out>,
    in_queue: Endpoint<Bulk, In>,
    timeout: Duration,
    transaction_id: TransactionId,

    event_tx: Sender<Result<Event, Error>>,
    _events_task: tokio::task::JoinHandle<()>,
}

impl DeviceHandle {
    pub(super) fn new(
        device: nusb::Device,
        flags: UsbDeviceFlagSet,
        interface: nusb::Interface,
        endpoints: Endpoints,
    ) -> Result<Self, Error> {
        let timeout = if flags.base.contains(DeviceFlags::LONG_TIMEOUT) {
            Duration::from_millis(60000)
        } else {
            Duration::from_millis(20000)
        };

        let out_queue = interface
            .endpoint::<Bulk, Out>(endpoints.bulk_out)
            .map_err(|e| Arc::new(e.into()))?;
        let in_queue = interface
            .endpoint::<Bulk, In>(endpoints.bulk_in)
            .map_err(|e| Arc::new(e.into()))?;
        let interrupt_queue = interface
            .endpoint::<Interrupt, In>(endpoints.interrupt)
            .map_err(|e| Arc::new(e.into()))?;

        let (event_tx, event_rx) = tokio::sync::broadcast::channel(100);
        let event_tx_clone = event_tx.clone();

        // TODO: Actually determine the endianness of the device.
        //       Not that important, since (seemingly) all devices use LE, but would be nice to verify.
        let endian = Endian::Little;
        let events_task = tokio::task::spawn(async move {
            struct UsbEventStream {
                endian: Endian,
                endpoints: Endpoints,
                interrupt_queue: Arc<Mutex<Endpoint<Interrupt, In>>>,
            }

            impl Stream for UsbEventStream {
                type Item = Result<Event, crate::error::Error<Arc<UsbError>>>;

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
                        interrupt_queue.submit(Buffer::new(buffer_size));
                    }

                    match interrupt_queue.poll_next_complete(cx) {
                        Poll::Ready(completion) => {
                            match UsbContainer::from_bytes((&completion.buffer, 0)) {
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

        Ok(Self {
            _device: device,
            endian,
            flags,
            interface,
            endpoints,
            out_queue,
            in_queue,
            timeout,
            transaction_id: TransactionId::new(1),

            event_tx,
            _events_task: events_task,
        })
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
    type TransportError = Arc<UsbError>;
    type Error = Error;
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
        self.endian
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
    ) -> Response<O, MtpError<Self::TransportError>>
    where
        O: DynOperation,
        for<'a> SerializedOperation<'a>: From<&'a O>,
    {
        async fn send(
            data: Vec<u8>,
            queue: &mut Endpoint<Bulk, Out>,
            buffer_size: usize,
            timeout: Duration,
        ) -> Result<(), Arc<UsbError>> {
            let data_len = data.len();

            let mut transfers = 1;
            queue.submit(Buffer::from(data));
            if data_len.is_multiple_of(buffer_size) {
                queue.submit(Buffer::new(0));
                transfers += 1;
            }

            for _ in 0..transfers {
                let completion = tokio::time::timeout(timeout, queue.next_complete())
                    .await
                    .map_err(|_| Arc::new(UsbError::Timeout))?;
                completion.status.map_err(|e| Arc::new(UsbError::from(e)))?;
            }

            Ok(())
        }

        // Phase 1: Command
        let command_buf;
        let op = operation.encode();
        {
            log::debug!("Sending operation of type: {:#X}", op.code());

            let command_container = UsbContainer::new(
                ContainerType::Command,
                op.code(),
                op.transaction_id(),
                op.encode_parameters(self.endian())?,
            );

            command_buf = command_container.encode(self.endian())?;
        }

        send(
            command_buf,
            &mut self.out_queue,
            self.endpoints.bulk_out_buffer_size,
            self.timeout,
        )
        .await
        .map_err(MtpError::Transport)?;

        // Phase 2: Data (if applicable)
        let mut responder_data = None;
        match O::DATA_DIRECTION {
            Some(DataDirection::InitiatorToResponder) => {
                let data_container = UsbContainer::new(
                    ContainerType::Data,
                    op.code(),
                    op.transaction_id(),
                    data.expect("data should exist"),
                );

                send(
                    data_container.encode(self.endian())?,
                    &mut self.out_queue,
                    self.endpoints.bulk_out_buffer_size,
                    self.timeout,
                )
                .await
                .map_err(MtpError::Transport)?;
            },
            Some(DataDirection::ResponderToInitiator) => {
                let data_phase = get_data_from_responder(self).await?;

                // Error was returned
                if data_phase.type_ == ContainerType::Response {
                    let err = O::decode_err(&data_phase.payload, self.endian(), data_phase.code)?;
                    return Err(MtpError::Protocol(err.into()));
                }

                responder_data = Some(data_phase.payload);
            },
            // No data phase
            _ => {},
        }

        // Phase 3: Response
        log::debug!("Attempting to get response");

        let response_raw = next_packet(self).await.map_err(MtpError::Transport)?;

        let (_, response) = UsbContainer::from_bytes((&response_raw, 0))?;

        if response.code != CODE_OK {
            let err = O::decode_err(&response.payload, self.endian(), response.code)?;
            return Err(MtpError::Protocol(err.into()));
        }

        match responder_data {
            Some(data) => {
                let data = O::decode_data(&data, self.endian())?;
                Ok(SuccessResponse {
                    data,
                    transaction_id: response.transaction_id,
                })
            },
            None => {
                // This case will only ever be hit for `()` anyway. The data we give it will
                // never be read.
                let data = O::decode_data(&[], self.endian())?;
                Ok(SuccessResponse {
                    data,
                    transaction_id: response.transaction_id,
                })
            },
        }
    }
}

impl Device for DeviceHandle {
    fn flags(&self) -> DeviceFlags {
        self.flags.base
    }
}

#[derive(PartialEq, Debug, Copy, Clone, DekuRead, DekuWrite)]
#[repr(u16)]
#[deku(
    id_type = "u16",
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
enum ContainerType {
    Undefined = 0x0000,
    Command = 0x0001,
    Data = 0x0002,
    Response = 0x0003,
    Event = 0x0004,
}

const USB_CONTAINER_HEADER_SIZE: u32 =
    (size_of::<u32>() + size_of::<u16>() + size_of::<u16>() + size_of::<TransactionId>()) as u32;

#[repr(C)]
#[derive(DekuRead, DekuWrite)]
#[deku(
    endian = "endian",
    ctx = "endian: deku::ctx::Endian",
    ctx_default = "deku::ctx::Endian::Big"
)]
struct UsbContainer {
    #[deku(assert = "*length >= USB_CONTAINER_HEADER_SIZE")]
    length: u32,
    type_: ContainerType,
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

    fn encode(&self, endian: Endian) -> Result<Vec<u8>, DekuError> {
        let mut cur = Cursor::new(Vec::new());
        let mut writer = Writer::new(&mut cur);
        self.to_writer(&mut writer, endian)?;

        Ok(cur.into_inner())
    }
}

async fn get_data_from_responder(
    handle: &mut DeviceHandle,
) -> Result<UsbContainer, MtpError<Arc<UsbError>>> {
    log::trace!("Attempting to get data from responder");

    let data_phase_raw = next_packet(handle).await.map_err(MtpError::Transport)?;
    let (_, mut data_phase) = UsbContainer::from_bytes((&data_phase_raw, 0))?;

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
            let data = next_packet(handle).await.map_err(MtpError::Transport)?;
            let Some(r) = remaining.checked_sub(data.len() as u32) else {
                return Err(MtpError::Transport(Arc::new(UsbError::TooMuchData)));
            };
            remaining = r;

            // Grow the first packet
            data_phase.payload.extend(data);
        }
    }

    Ok(data_phase)
}

async fn next_packet(handle: &mut DeviceHandle) -> Result<Vec<u8>, Arc<UsbError>> {
    let pending = handle.in_queue.pending();
    for _ in 0..(2usize.saturating_sub(pending)) {
        handle
            .in_queue
            .submit(Buffer::new(handle.endpoints.bulk_in_buffer_size));
    }

    log::trace!("Waiting for next packet");
    let completion = tokio::time::timeout(handle.timeout, handle.in_queue.next_complete())
        .await
        .map_err(|_| Arc::new(UsbError::Timeout))?;
    completion.status.map_err(|e| Arc::new(UsbError::from(e)))?;

    Ok(completion.buffer.into_vec())
}
