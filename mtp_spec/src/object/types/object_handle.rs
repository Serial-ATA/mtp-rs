/// Identifiers that provide a device- and session-unique consistent reference to a
/// logical object on a device.
///
/// Object handles are used in MTP transactions to reference a logical object on the device,
/// but do not necessarily reference actual data constructs on the device.
///
/// Object handles are only persistent within an MTP session; once a session has been re-opened, all
/// previous values shall be assumed to be invalid, and the contents of the Responder must be
/// re-enumerated if object handles are needed
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[repr(transparent)]
pub struct ObjectHandle(u32);
