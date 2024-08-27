use crate::object::types::ObjectHandle;

const fn counter<const N: usize>(_: [(); N]) -> usize {
	N
}

macro_rules! replace_expr {
	($_t:tt $sub:expr) => {
		$sub
	};
}

const MAX_PARAMETERS: usize = 3;

macro_rules! define_event {
	(
		$(#[$meta:meta])*
		pub struct $name:ident {
			code: $code:literal,
			$(
                $($param:ident: $ty:ty),+ $(,)?
            )?
		}
	) => {
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
                    $param: $ty,
                ),*
            )?
		}

		impl $name {
			const CODE: u16 = $code;

            $(
                $(
                    paste::paste! {
                        pub fn [<get_ $param>](self) -> $ty {
                            self.$param
                        }
                    }
                )*
            )?
		}
	}
}

define_event! {
    /// This event code is undefined, and is not used.
    pub struct Undefined {
        code: 0x4000,
    }
}

define_event! {
    /// This event is used to initiate the cancellation of a transaction.
    ///
    /// It is strongly recommended to utilize USB cancelation functionality
    /// in preference to this protocol level cancelation. When an [`Initiator`]
    /// or [`Responder`] receives this event, it shall cancel the transaction
    /// identified by the [`TransactionId`] in the event dataset. If the transaction
    /// has already completed, this event shall be ignored.
    pub struct CancelTransaction {
        code: 0x4001,
    }
}

define_event! {
    /// This event indicates that a new data object has been added to the device.
    ///
    /// The handle of the new object can be retreived with [`ObjectAdded::get_object_handle`].
    ///
    /// Note that in the event that multiple objects have been added, each object
    /// shall be reported with a separate `ObjectAdded` event.
    pub struct ObjectAdded {
        code: 0x4002,
        object_handle: ObjectHandle,
    }
}