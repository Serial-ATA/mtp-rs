use crate::communication::TransactionId;
use crate::device::storage::id::StorageId;
use crate::object::types::{ObjectFormatCode, ObjectHandle};

const fn counter<const N: usize>(_: [(); N]) -> usize {
	N
}

macro_rules! replace_expr {
	($_t:tt $sub:expr) => {
		$sub
	};
}

const MAX_PARAMETERS: usize = 3;

macro_rules! define_events {
	(
		$(
			$(#[$meta:meta])*
			pub struct $name:ident {
				code: $code:literal,
				$(
					$($param:ident: $ty:ty),+ $(,)?
				)?
			}
		)*
	) => {
		#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, deku::DekuRead, deku::DekuWrite)]
		#[repr(u16)]
		#[deku(
			id_type = "u16",
			id_endian = "little",
			ctx = "_endian: deku::ctx::Endian",
			ctx_default = "deku::ctx::Endian::Little"
		)]
		pub enum Event {
			$(
				#[deku(id = $code)]
				$name = $code,
			)*
			#[deku(id_pat = "o if (0xC000_u16..=0xC7FF_u16).contains(&o)")]
			VendorSpecific(u16),
			#[deku(id_pat = "_", default)]
			Unknown(u16),
		}

		$(
		const _: () = {
            $(
                if counter([$(replace_expr!($param ())),*]) > MAX_PARAMETERS {
                    panic!("Too many parameters");
                }
            )?
		};

		$(#[$meta])*
		pub struct $name {
			$(
                $(
                    pub $param: $ty,
                )*
            )?
		}
		)*
	}
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
