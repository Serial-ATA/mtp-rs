use crate::communication::TransactionId;
use crate::device::storage::id::StorageId;
use crate::object::types::{ObjectFormatCode, ObjectHandle};

use deku::no_std_io::{Read, Seek, Write};
use deku::{DekuError, DekuReader};

macro_rules! define_events {
	(
		$($events:tt)*
	) => {
		accumulate_events!(
			EVENTS_PARSER_ENUM: [
				pub enum EventsParser {}

				fn code(&self) -> u16 {
					match self {}
				}

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

			fn code(&self) -> u16 {
				match self {
					$($parser_code_match_arms:tt)*
				}
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
		#[derive(Copy, Clone, Debug, PartialEq, Eq, deku::DekuRead)]
		#[repr(u16)]
		#[deku(
			id_type = "u16",
			id_endian = "little",
			ctx = "_endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Little"
		)]
		pub enum EventsParser {
			$($parser_variants)*
			#[deku(id_pat = "_", default)]
			Unknown {
				code: u16,
				param1: u32,
				param2: u32,
				param3: u32
			},
		}

		impl EventsParser {
			fn code(&self) -> u16 {
				match self {
					$($parser_code_match_arms)*
					Self::Unknown { code, param1, param2, param3 } => *code,
				}
			}
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
					_ => Event::Unknown { code: event.code(), param1: 0, param2: 0, param3: 0 }
				}
			}
		}

		#[derive(Copy, Clone, Debug, PartialEq, Eq)]
		#[repr(u16)]
		pub enum Event {
			$($variants)*
			VendorSpecific {
				code: u16,
				param1: u32,
				param2: u32,
				param3: u32
			},
			Unknown {
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

		#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
		#[repr(u16)]
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
				let code = u16::from_reader_with_ctx(reader, deku::ctx::Endian::Little)?;
				Ok(code.into())
			}
		}

		impl deku::DekuReader<'_, deku::ctx::Endian> for EventCode {
			fn from_reader_with_ctx<R: Read + Seek>(
				reader: &mut deku::reader::Reader<R>,
				_: deku::ctx::Endian,
			) -> Result<Self, DekuError> {
				let code = u16::from_reader_with_ctx(reader, deku::ctx::Endian::Little)?;
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
				code.to_writer(writer, deku::ctx::Endian::Little)
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

	// Event with 0 parameters
	(
		EVENTS_PARSER_ENUM: [
			pub enum EventsParser {
				$($parser_variants:tt)*
			}

			fn code(&self) -> u16 {
				match self {
					$($parser_code_match_arms:tt)*
				}
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
						__param3: u32
					},
				}

				fn code(&self) -> u16 {
					match self {
						$($parser_code_match_arms)*
						EventsParser::$name { .. } => $code,
					}
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

			fn code(&self) -> u16 {
				match self {
					$($parser_code_match_arms:tt)*
				}
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
						__param3: u32
					},
				}

				fn code(&self) -> u16 {
					match self {
						$($parser_code_match_arms)*
						EventsParser::$name { .. } => $code,
					}
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

			fn code(&self) -> u16 {
				match self {
					$($parser_code_match_arms:tt)*
				}
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
						__param3: u32
					},
				}

				fn code(&self) -> u16 {
					match self {
						$($parser_code_match_arms)*
						EventsParser::$name { .. } => $code,
					}
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
						$param: $ty,
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

			fn code(&self) -> u16 {
				match self {
					$($parser_code_match_arms:tt)*
				}
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

				fn code(&self) -> u16 {
					match self {
						$($parser_code_match_arms)*
						EventsParser::$name { .. } => $code,
					}
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
						$param: $ty,
						$param2: $ty2,
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
	/// It is strongly recommended to utilize USB cancelation functionality
	/// in preference to this protocol level cancelation. When an [`Initiator`]
	/// or [`Responder`] receives this event, it shall cancel the transaction
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
	/// to [`StorageId::DEFAULT_STORE`] and the initiator should retrieve a new list of StorageIDs using the
	/// [`GetStorageIDs`] operation.
	///
	/// [`GetStorageIDs`]: crate::communication::operations::GetStorageIDs
	pub struct StoreAdded {
		code: 0x4004,
		storage_id: StorageId,
	}

	/// A new store object has been added to the device.
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
		storage_id: u32,
	}

	/// The ObjectInfo dataset for a particular object has changed, and it should be requested again.
	pub struct ObjectInfoChanged {
		code: 0x4007,
		object: ObjectHandle,
	}

	/// The capabilities of the Responder have changed, and the [`DeviceInfo`] should be requested again.
	///
	/// This may be caused by the Responder going into or out of a sleep state, or by the Responder
	/// losing or gaining some functionality.
	pub struct DeviceInfoChanged {
		code: 0x4008,
	}

	/// The responder asks the initiator to initiate a [`GetObject`] operation on the specified handle.
	///
	/// This allows for push-mode to be enabled on devices that intrinsically use pull mode.
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

	/// The device is resettting, either manually of via the [`ResetDevice`] operation.
	///
	/// This is an indication that the session is about to be closed. All open sessions will receive
	/// this, except for the one that initiated the reset.
	///
	/// [`ResetDevice`]: crate::communication::operations::ResetDevice
	pub struct DeviceReset {
		code: 0x400B,
	}

	/// Information in the [`StorageInfo`] dataset for the specified store had changed.
	pub struct StorageInfoChanged {
		code: 0x400C,
		storage_id: StorageId,
	}

	/// A capture session, previously initiated by the [`InitiateCapture`] operation, is complete.
	///
	/// [`InitiateCapture`]: crate::communication::operations::InitiateCapture
	pub struct CaptureComplete {
		code: 0x400D,
		transaction: TransactionId,
	}

	/// The initiator must do anything necessary to update its knowledge of the responder.
	///
	/// This may include re-obtaining information such as individual datasets or ObjectHandle lists,
	/// or may even result in the session being closed and re-opened.
	pub struct UnreportedStatus {
		code: 0x400E,
	}

	/// An object property value on the Responder has changed, without that change being performed by the initiator
	pub struct ObjectPropChanged {
		code: 0xC801,
		object: ObjectHandle,
		prop_code: u32,
	}

	/// An object property description dataset has been updated, indicating some change on the device
	pub struct ObjectPropDescChanged {
		code: 0xC802,
		prop_code: u32,
		format: ObjectFormatCode
	}

	/// The references on an object have been updated
	pub struct ObjectReferencesChanged {
		code: 0xC803,
		object: ObjectHandle,
	}
}
