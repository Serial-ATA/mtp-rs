use crate::communication::TransactionId;
use crate::device::properties::DevicePropertyCode;
use crate::device::storage::StorageId;
use crate::object::properties::ObjectPropertyCode;
use crate::object::{ObjectFormatCode, ObjectHandle};

use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek, Write};
use deku::{DekuError, DekuReader};

// Attempt to parse something, ignoring errors if it is missing
macro_rules! try_parse {
    ($reader:ident, $ty:ty, $ctx:expr) => {
        match <$ty>::from_reader_with_ctx($reader, $ctx) {
            Ok(v) => v,
            Err(DekuError::Incomplete(size)) if size.byte_size() == size_of::<$ty>() => {
                <$ty>::default()
            },
            Err(e) => return Err(e),
        }
    };
}

macro_rules! define_events {
	(
		$($events:tt)*
	) => {
		accumulate_events!(
			EVENTS_PARSER_ENUM: [
				pub enum EventsParser {}

				fn from(event: EventsParser) -> Event {
					match event {}
				}
			],
			EVENTS_ENUM: [ pub enum Events {} ],
			EVENT_CODE_ENUM: [ pub enum EventCode {} ],
			ALL_EVENT_CODES: [
				match code {}
				match self {}
			]
			$($events)*
		);
	}
}

macro_rules! accumulate_events {
	// Base case, no more events
	(
		EVENTS_PARSER_ENUM: [
			pub enum EventsParser {
				$($parser_variants:tt)*
			}

			fn from(event: EventsParser) -> Event {
				match event {
					$($parser_match_arms:tt)*
				}
			}
		],

		EVENTS_ENUM: [
			pub enum Events {
				$($variants:tt)*
			}
		],

		EVENT_CODE_ENUM: [
			pub enum EventCode {
				$($code_variants:tt)*
			}
		],

		ALL_EVENT_CODES: [
			match code {
				$($code_match_arms:tt)*
			}

			match self {
				$($code_match_arms_inverse:tt)*
			}
		]
	) => {
		/// This exists to provide the conversion step of determining whether an event is
		/// [`Event::VendorSpecific`] or [`Event::Unknown`]
		#[derive(Copy, Clone, Debug, PartialEq, Eq, deku::DekuRead)]
		#[repr(u16)]
		#[deku(
			id_type = "u16",
			id_endian = "endian",
			ctx = "endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Big"
		)]
		pub(super) enum EventsParser {
			$($parser_variants)*
			#[deku(id_pat = "_", default)]
			Unknown {
				code: u16,
				param1: u32,
				param2: u32,
				param3: u32
			},
		}

		impl From<EventsParser> for Event {
			fn from(event: EventsParser) -> Self {
				match event {
					$($parser_match_arms)*
					EventsParser::Unknown {
						code,
						param1,
						param2,
						param3
					} if (0x6000_u16..=0x63FF_u16).contains(&code) => Event::VendorSpecific { code, param1, param2, param3 },
					EventsParser::Unknown { code, param1, param2, param3 } => Event::Unknown { code, param1, param2, param3 }
				}
			}
		}

		// special impl for specifying the code in the ctx.
		// USB containers specify the code for us outside of the normal event dataset
		impl DekuReader<'_, (Endian, u16)> for EventsParser {
			fn from_reader_with_ctx<R: Read + Seek>(
				reader: &mut deku::reader::Reader<R>,
				(endian, code): (Endian, u16),
			) -> Result<Self, DekuError> {
				match code {
					0x4000..=0x4fff | 0xc800..=0xcfff => accumulate_events!(@PARSE_WITH_CODE reader, endian, code, $($parser_variants)*),
					_ => {
						let param1 = u32::from_reader_with_ctx(reader, endian)?;
						let param2 = u32::from_reader_with_ctx(reader, endian)?;
						let param3 = u32::from_reader_with_ctx(reader, endian)?;

						Ok(EventsParser::Unknown { code, param1, param2, param3 })
					}
				}
			}
		}

		/// A notification of an event, from either party
		///
		/// Events differ from operations, in that they need no acknowledgement or action.
		///
		/// They will only ever occur in an open session.
		///
		/// See [`PtpIo`](crate::device::PtpIo) for information on how to poll for events.
		#[derive(Copy, Clone, Debug, PartialEq, Eq)]
		#[repr(u16)]
		pub enum Event {
			$($variants)*
			/// Some vendor-specific event
			///
			/// Note that the event may or may not use all of the parameters. Unused parameters should be `0`.
			#[allow(missing_docs)]
			VendorSpecific {
				/// The event code
				code: u16,
				param1: u32,
				param2: u32,
				param3: u32
			},
			/// Any unknown event
			///
			/// This falls outside of the [`Self::VendorSpecific`] range, which may indicate
			/// a faulty device.
			///
			/// Note that the event may or may not use all of the parameters. Unused parameters should be `0`.
			#[allow(missing_docs)]
			Unknown {
				/// The event code
				code: u16,
				param1: u32,
				param2: u32,
				param3: u32
			}
		}

		impl deku::DekuReader<'_, ()> for Event {
			fn from_reader_with_ctx<R: Read + Seek>(
				reader: &mut deku::reader::Reader<R>,
				_: (),
			) -> Result<Self, DekuError> {
				let event = EventsParser::from_reader_with_ctx(reader, ())?;
				Ok(event.into())
			}
		}

		impl deku::DekuReader<'_, (Endian, u16)> for Event {
			fn from_reader_with_ctx<R: Read + Seek>(
				reader: &mut deku::reader::Reader<R>,
				ctx: (Endian, u16),
			) -> Result<Self, DekuError> {
				let event = EventsParser::from_reader_with_ctx(reader, ctx)?;
				Ok(event.into())
			}
		}

		/// The code for an [`Event`]
		#[repr(u16)]
		#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
		#[allow(missing_docs)]
		pub enum EventCode {
			$($code_variants)*
			VendorSpecific(u16),
			Unknown(u16),
		}

		impl From<u16> for EventCode {
			fn from(code: u16) -> Self {
				match code {
					$($code_match_arms)*
					_ if (0x6000_u16..=0x63FF_u16).contains(&code) => EventCode::VendorSpecific(code),
					_ => EventCode::Unknown(code)
				}
			}
		}

		impl From<EventCode> for u16 {
			fn from(code: EventCode) -> Self {
				match code {
					$($code_match_arms_inverse)*
					EventCode::VendorSpecific(code) => code,
					EventCode::Unknown(code) => code
				}
			}
		}

		impl deku::DekuReader<'_, ()> for EventCode {
			fn from_reader_with_ctx<R: Read + Seek>(
				reader: &mut deku::reader::Reader<R>,
				_: (),
			) -> Result<Self, DekuError> {
				let code = u16::from_reader_with_ctx(reader, deku::ctx::Endian::Big)?;
				Ok(code.into())
			}
		}

		impl deku::DekuReader<'_, deku::ctx::Endian> for EventCode {
			fn from_reader_with_ctx<R: Read + Seek>(
				reader: &mut deku::reader::Reader<R>,
				endian: deku::ctx::Endian,
			) -> Result<Self, DekuError> {
				let code = u16::from_reader_with_ctx(reader, endian)?;
				Ok(code.into())
			}
		}

		impl deku::DekuWriter<()> for EventCode {
			fn to_writer<W: Write + Seek>(
				&self,
				writer: &mut deku::writer::Writer<W>,
				_: (),
			) -> Result<(), DekuError> {
				let code = u16::from(*self);
				code.to_writer(writer, deku::ctx::Endian::Big)
			}
		}

		impl deku::DekuWriter<deku::ctx::Endian> for EventCode {
			fn to_writer<W: Write + Seek>(
				&self,
				writer: &mut deku::writer::Writer<W>,
				endian: deku::ctx::Endian,
			) -> Result<(), DekuError> {
				let code = u16::from(*self);
				code.to_writer(writer, endian)
			}
		}
	};

	(@PARSE_WITH_CODE
		$reader:ident,
		$endian:ident,
		$code:ident,

		$(
			#[deku(id = $event_code:literal)]
			$(#[$_meta:meta])*
			$variant:ident {
				$param1:ident: $param1_ty:ty,
				$param2:ident: $param2_ty:ty,
				$param3:ident: $param3_ty:ty,
			},
		)*
	) => {
		match $code {
			$(
			$event_code => {
				let param1 = try_parse!($reader, $param1_ty, $endian);
				let param2 = try_parse!($reader, $param2_ty, $endian);
				let param3 = try_parse!($reader, $param3_ty, $endian);

				Ok(EventsParser::$variant { $param1: param1, $param2: param2, $param3: param3 })
			}
			)*
			_ => {
				let param1 = try_parse!($reader, u32, $endian);
				let param2 = try_parse!($reader, u32, $endian);
				let param3 = try_parse!($reader, u32, $endian);

				Ok(EventsParser::Unknown { code: $code, param1, param2, param3 })
			}
		}
	};

	// Event with 0 parameters
	(
		EVENTS_PARSER_ENUM: [
			pub enum EventsParser {
				$($parser_variants:tt)*
			}

			fn from(event: EventsParser) -> Event {
				match event {
					$($parser_match_arms:tt)*
				}
			}
		],

		EVENTS_ENUM: [
			pub enum Events {
				$($variants:tt)*
			}
		],

		EVENT_CODE_ENUM: [
			pub enum EventCode {
				$($code_variants:tt)*
			}
		],

		ALL_EVENT_CODES: [
			match code {
				$($code_match_arms:tt)*
			}

			match self {
				$($code_match_arms_inverse:tt)*
			}
		]

		$(#[$meta:meta])*
		pub struct $name:ident {
			code: $code:literal $(,)?
		}

		$($rest:tt)*
	) => {
		accumulate_events!(
			EVENTS_PARSER_ENUM: [
				pub enum EventsParser {
					$($parser_variants)*
					#[deku(id = $code)]
					$(#[$meta])*
					$name {
						__param1: u32,
						__param2: u32,
						__param3: u32,
					},
				}

				fn from(event: EventsParser) -> Event {
					match event {
						$($parser_match_arms)*
						EventsParser::$name { .. } => Event::$name,
					}
				}
			],

			EVENTS_ENUM: [
				pub enum Events {
					$($variants)*
					$(#[$meta])*
					$name,
				}
			],

			EVENT_CODE_ENUM: [
				pub enum EventCode {
					$($code_variants)*
					$name = $code,
				}
			],

			ALL_EVENT_CODES: [
				match code {
					$($code_match_arms)*
					$code => EventCode::$name,
				}

				match self {
					$($code_match_arms_inverse)*
					EventCode::$name => $code,
				}
			]

			$($rest)*
		);
	};

	// Event with 1 parameter
	(
		EVENTS_PARSER_ENUM: [
			pub enum EventsParser {
				$($parser_variants:tt)*
			}

			fn from(event: EventsParser) -> Event {
				match event {
					$($parser_match_arms:tt)*
				}
			}
		],

		EVENTS_ENUM: [
			pub enum Events {
				$($variants:tt)*
			}
		],

		EVENT_CODE_ENUM: [
			pub enum EventCode {
				$($code_variants:tt)*
			}
		],

		ALL_EVENT_CODES: [
			match code {
				$($code_match_arms:tt)*
			}

			match self {
				$($code_match_arms_inverse:tt)*
			}
		]

		$(#[$meta:meta])*
		pub struct $name:ident {
			code: $code:literal,
			$param:ident: $ty:ty $(,)?
		}

		$($rest:tt)*
	) => {
		accumulate_events!(
			EVENTS_PARSER_ENUM: [
				pub enum EventsParser {
					$($parser_variants)*
					#[deku(id = $code)]
					$(#[$meta])*
					$name {
						$param: $ty,
						__param2: u32,
						__param3: u32,
					},
				}

				fn from(event: EventsParser) -> Event {
					match event {
						$($parser_match_arms)*
						EventsParser::$name { $param, .. } => Event::$name {
							$param,
						},
					}
				}
			],

			EVENTS_ENUM: [
				pub enum Events {
					$($variants)*
					$(#[$meta])*
					$name {
						#[allow(missing_docs)]
						$param: $ty,
					},
				}
			],

			EVENT_CODE_ENUM: [
				pub enum EventCode {
					$($code_variants)*
					$name = $code,
				}
			],

			ALL_EVENT_CODES: [
				match code {
					$($code_match_arms)*
					$code => EventCode::$name,
				}

				match self {
					$($code_match_arms_inverse)*
					EventCode::$name => $code,
				}
			]

			$($rest)*
		);
	};

	// Event with 2 parameters
	(
		EVENTS_PARSER_ENUM: [
			pub enum EventsParser {
				$($parser_variants:tt)*
			}

			fn from(event: EventsParser) -> Event {
				match event {
					$($parser_match_arms:tt)*
				}
			}
		],

		EVENTS_ENUM: [
			pub enum Events {
				$($variants:tt)*
			}
		],

		EVENT_CODE_ENUM: [
			pub enum EventCode {
				$($code_variants:tt)*
			}
		],

		ALL_EVENT_CODES: [
			match code {
				$($code_match_arms:tt)*
			}

			match self {
				$($code_match_arms_inverse:tt)*
			}
		]

		$(#[$meta:meta])*
		pub struct $name:ident {
			code: $code:literal,
			$param:ident: $ty:ty,
			$param2:ident: $ty2:ty $(,)?
		}

		$($rest:tt)*
	) => {
		accumulate_events!(
			EVENTS_PARSER_ENUM: [
				pub enum EventsParser {
					$($parser_variants)*
					#[deku(id = $code)]
					$(#[$meta])*
					$name {
						$param: $ty,
						$param2: $ty2,
						__param3: u32,
					},
				}

				fn from(event: EventsParser) -> Event {
					match event {
						$($parser_match_arms)*
						EventsParser::$name { $param, $param2, .. } => Event::$name {
							$param,
							$param2,
						},
					}
				}
			],

			EVENTS_ENUM: [
				pub enum Events {
					$($variants)*
					$(#[$meta])*
					$name {
						#[allow(missing_docs)]
						$param: $ty,
						#[allow(missing_docs)]
						$param2: $ty2,
					},
				}
			],

			EVENT_CODE_ENUM: [
				pub enum EventCode {
					$($code_variants)*
					$name = $code,
				}
			],

			ALL_EVENT_CODES: [
				match code {
					$($code_match_arms)*
					$code => EventCode::$name,
				}

				match self {
					$($code_match_arms_inverse)*
					EventCode::$name => $code,
				}
			]

			$($rest)*
		);
	};

	// Event with 3 parameters
	(
		EVENTS_PARSER_ENUM: [
			pub enum EventsParser {
				$($parser_variants:tt)*
			}

			fn from(event: EventsParser) -> Event {
				match event {
					$($parser_match_arms:tt)*
				}
			}
		],

		EVENTS_ENUM: [
			pub enum Events {
				$($variants:tt)*
			}
		],

		EVENT_CODE_ENUM: [
			pub enum EventCode {
				$($code_variants:tt)*
			}
		],

		ALL_EVENT_CODES: [
			match code {
				$($code_match_arms:tt)*
			}

			match self {
				$($code_match_arms_inverse:tt)*
			}
		]

		$(#[$meta:meta])*
		pub struct $name:ident {
			code: $code:literal,
			$param:ident: $ty:ty,
			$param2:ident: $ty2:ty,
			$param3:ident: $ty3:ty $(,)?
		}

		$($rest:tt)*
	) => {
		accumulate_events!(
			EVENTS_PARSER_ENUM: [
				pub enum EventsParser {
					$($parser_variants)*
					#[deku(id = $code)]
					$(#[$meta])*
					$name {
						$param: $ty,
						$param2: $ty2,
						$param3: $ty3
					},
				}

				fn from(event: EventsParser) -> Event {
					match event {
						$($parser_match_arms)*
						EventsParser::$name { $param, $param2, $param3, .. } => Event::$name {
							$param,
							$param2,
							$param3,
						},
					}
				}
			],

			EVENTS_ENUM: [
				pub enum Events {
					$($variants)*
					$(#[$meta])*
					$name {
						#[allow(missing_docs)]
						$param: $ty,
						#[allow(missing_docs)]
						$param2: $ty2,
						#[allow(missing_docs)]
						$param3: $ty3
					},
				}
			],
			EVENT_CODE_ENUM: [
				pub enum EventCode {
					$($code_variants)*
					$name = $code,
				}
			],

			ALL_EVENT_CODES: [
				match code {
					$($code_match_arms)*
					$code => EventCode::$name,
				}

				match self {
					$($code_match_arms_inverse)*
					EventCode::$name => $code,
				}
			]

			$($rest)*
		);
	};
}

define_events! {
    /// This event code is undefined, and is not used.
    pub struct Undefined {
        code: 0x4000,
    }

    /// A transaction has been cancelled.
    ///
    /// When an initiator or responder receives this event, it shall cancel the transaction
    /// identified by the [`TransactionId`] in the event dataset. If the transaction
    /// has already completed, this event shall be ignored.
    pub struct CancelTransaction {
        code: 0x4001,
    }

    /// A new data object has been added to the device.
    ///
    /// Note that in the event that multiple objects have been added, each object
    /// shall be reported with a separate `ObjectAdded` event.
    pub struct ObjectAdded {
        code: 0x4002,
        object_handle: ObjectHandle,
    }

    /// A data object has been removed to the device.
    ///
    /// Note that in the event that multiple objects have been removed, each object
    /// shall be reported with a separate `ObjectRemoved` event.
    pub struct ObjectRemoved {
        code: 0x4003,
        object_handle: ObjectHandle,
    }

    /// A new store object has been added to the device.
    ///
    /// If the new store contains more than one logical store, then the first parameter shall be set
    /// to [`StorageId::DEFAULT_STORE`] and the initiator should retrieve a new list of [`StorageId`]s using the
    /// [`GetStorageIDs`] operation.
    ///
    /// [`GetStorageIDs`]: crate::communication::operation::GetStorageIDs
    pub struct StoreAdded {
        code: 0x4004,
        storage_id: StorageId,
    }

    /// The indicated stores are no longer available.
    ///
    /// If the store that has been removed is only a single logical store within a physical store, the
    /// entire [`StorageId`] shall be sent, which indicates that any other logical stores on that physical
    /// store are still available.
    ///
    /// If the physical store and all logical stores upon it are removed
    /// (for example, removal of an ejectable media device that contains multiple partitions), the first
    /// parameter shall contain the PhysicalStorageID in the most significant sixteen bits, with the least significant sixteen bits set to 0xFFFF.
    pub struct StoreRemoved {
        code: 0x4005,
        storage_id: StorageId,
    }

    /// A property changed on the device due to something external to this session.
    pub struct DevicePropChanged {
        code: 0x4006,
        prop_code: DevicePropertyCode,
    }

    /// The [`ObjectInfo`] dataset for a particular object has changed, and it should be requested again.
    ///
    /// [`ObjectInfo`]: crate::object::info::ObjectInfo
    pub struct ObjectInfoChanged {
        code: 0x4007,
        object: ObjectHandle,
    }

    /// The capabilities of the Responder have changed, and the [`DeviceInfo`] should be requested again.
    ///
    /// This may be caused by the Responder going into or out of a sleep state, or by the Responder
    /// losing or gaining some functionality.
    ///
    /// [`DeviceInfo`]: crate::device::info::DeviceInfo
    pub struct DeviceInfoChanged {
        code: 0x4008,
    }

    /// The responder asks the initiator to initiate a [`GetObject`] operation on the specified handle.
    ///
    /// This allows for push-mode to be enabled on devices that intrinsically use pull mode.
    ///
    /// [`GetObject`]: crate::communication::operation::GetObject
    pub struct RequestObjectTransfer {
        code: 0x4009,
        object: ObjectHandle,
    }

    /// The specified storage has become full.
    ///
    /// Any multi-object capture that may be occurring shall retain the objects that were written to
    /// a store before the store became full.
    pub struct StoreFull {
        code: 0x400A,
        storage_id: StorageId,
    }

    /// The device is resetting, either manually of via the [`ResetDevice`] operation.
    ///
    /// This is an indication that the session is about to be closed. All open sessions will receive
    /// this, except for the one that initiated the reset.
    ///
    /// [`ResetDevice`]: crate::communication::operation::ResetDevice
    pub struct DeviceReset {
        code: 0x400B,
    }

    /// Information in the [`StorageInfo`] dataset for the specified store had changed.
    ///
    /// [`StorageInfo`]: crate::device::storage::info::StorageInfo
    pub struct StorageInfoChanged {
        code: 0x400C,
        storage_id: StorageId,
    }

    /// A capture session, previously initiated by the [`InitiateCapture`] operation, is complete.
    ///
    /// [`InitiateCapture`]: crate::communication::operation::InitiateCapture
    pub struct CaptureComplete {
        code: 0x400D,
        transaction: TransactionId,
    }

    /// The initiator must do anything necessary to update its knowledge of the responder.
    ///
    /// This may include re-obtaining information such as individual datasets or [`ObjectHandle`] lists,
    /// or may even result in the session being closed and re-opened.
    pub struct UnreportedStatus {
        code: 0x400E,
    }

    /// An object property value on the Responder has changed, without that change being performed by the initiator
    pub struct ObjectPropChanged {
        code: 0xC801,
        object: ObjectHandle,
        prop_code: ObjectPropertyCode,
    }

    /// An object property description dataset has been updated, indicating some change on the device
    pub struct ObjectPropDescChanged {
        code: 0xC802,
        prop_code: ObjectPropertyCode,
        format: ObjectFormatCode
    }

    /// The references on an object have been updated
    pub struct ObjectReferencesChanged {
        code: 0xC803,
        object: ObjectHandle,
    }
}
