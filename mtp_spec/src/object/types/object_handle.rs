use crate::communication::Parameter;
use crate::object::types::ArrayEncodable;

use deku::ctx::Endian;
use deku::no_std_io::{Read, Seek, Write};
use deku::prelude::{Reader, Writer};
use deku::{DekuError, DekuRead, DekuReader, DekuWrite, DekuWriter};

/// Identifiers that provide a device- and session-unique consistent reference to a
/// logical object on a device.
///
/// Object handles are used in MTP transactions to reference a logical object on the device,
/// but do not necessarily reference actual data constructs on the device.
///
/// Object handles are only persistent within an MTP session; once a session has been re-opened, all
/// previous values shall be assumed to be invalid, and the contents of the Responder must be
/// re-enumerated if object handles are needed
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct ObjectHandle(u32);

impl Default for ObjectHandle {
    fn default() -> Self {
        Self::NONE
    }
}

impl ObjectHandle {
    /// Indicates the absense of an object handle
    ///
    /// This is used in both parameters (e.g. not assigning a parent to an object), and return types
    /// in contexts where missing objects are not errors.
    pub const NONE: Self = ObjectHandle(0);

    /// Indicates a selection of *all* objects in the context
    ///
    /// For example in the [`DeleteObject`] operation, this can be used to delete all objects on the
    /// responder.
    ///
    /// [`DeleteObject`]: crate::communication::operation::DeleteObject
    pub const ALL: Self = ObjectHandle(u32::MAX);

    /// In some contexts, we want to convert empty handles to `None` when parsing.
    #[allow(clippy::unnecessary_wraps)] // Used in parsing code, results are expected
    pub(crate) fn parse_optional(handle: ObjectHandle) -> Result<Option<ObjectHandle>, DekuError> {
        if handle == Self::NONE {
            Ok(None)
        } else {
            Ok(Some(handle))
        }
    }
}

impl From<u32> for ObjectHandle {
    fn from(value: u32) -> Self {
        ObjectHandle(value)
    }
}

impl From<ObjectHandle> for u32 {
    fn from(value: ObjectHandle) -> Self {
        value.0
    }
}

impl From<ObjectHandle> for Parameter {
    fn from(value: ObjectHandle) -> Self {
        Parameter::new(value.0)
    }
}

impl DekuReader<'_, Endian> for ObjectHandle {
    fn from_reader_with_ctx<R: Read + Seek>(
        reader: &mut Reader<R>,
        ctx: Endian,
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        u32::from_reader_with_ctx(reader, ctx).map(ObjectHandle)
    }
}

impl DekuWriter<Endian> for ObjectHandle {
    fn to_writer<W: Write + Seek>(
        &self,
        writer: &mut Writer<W>,
        _: Endian,
    ) -> Result<(), DekuError> {
        self.0.to_writer(writer, Endian::Little)
    }
}

// `ObjectHandle` is simply a `u32` wrapper
impl ArrayEncodable for ObjectHandle {}
