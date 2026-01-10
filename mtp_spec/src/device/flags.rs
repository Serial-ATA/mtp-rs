bitflags::bitflags! {
    /// Flags indicating the bugs/unexpected behaviors of MTP devices
    ///
    /// These flags match device flags of `libmtp` here: <https://sourceforge.net/p/libmtp/code/ci/master/tree/src/device-flags.h>
    #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct DeviceFlags: u32 {
        /// The [`GetObjectPropList`] operation doesn't support [`ObjectHandle::ALL`]
        ///
        /// This is different from [`Self::BROKEN_MTP_GET_OBJECT_PROP_LIST`], which indicates that [`GetObjectPropList`]
        /// is broken for *single* object queries.
        ///
        /// [`GetObjectPropList`]: crate::communication::operation::GetObjectPropList
        /// [`ObjectHandle::ALL`]: crate::object::types::ObjectHandle::ALL
        const BROKEN_MTP_GET_OBJECT_PROP_LIST_ALL = 0b0000_0001;
        const BROKEN_SET_SAMPLE_DIMENSIONS = 0b0000_0100;
        const BROKEN_MTP_GET_OBJECT_PROP_LIST = 0b0000_1000;
        /// The device claims to support writing the [`DateModified`] property, but writes will fail
        ///
        /// [`DateModified`]: crate::object::types::properties::DateModified
        const CANNOT_HANDLE_DATEMODIFIED = 0b1000_0000;
        const OGG_IS_UNKNOWN = 0b0001_0000_0000;
        /// The playlist format is the Samsung SPL format v1.00, rather than a proper MTP playlist.
        const PLAYLIST_SPL_V1 = 0b0010_0000_0000;
        /// The playlist format is the Samsung SPL format v2.00, rather than a proper MTP playlist.
        const PLAYLIST_SPL_V2 = 0b1000_0000_0000;
        /// The device needs unique filenames, no two files can be named the same string.
        const UNIQUE_FILENAMES = 0b0001_0000_0000_0000;
        /// The device doesn't support querying the [`BatteryLevel`] property
        ///
        /// [`BatteryLevel`]: crate::device::properties::BatteryLevel
        const BROKEN_BATTERY_LEVEL = 0b0010_0000_0000_0000;
        /// The device may need additional time to respond, extend its timeout
        const LONG_TIMEOUT = 0b0100_0000_0000_0000;
        const PROPLIST_OVERRIDES_OI = 0b1000_0000_0000_0000;
        const SAMSUNG_OFFSET_BUG = 0b0001_0000_0000_0000_0000;
        const FLAC_IS_UNKNOWN = 0b0010_0000_0000_0000_0000;
        const ONLY_7BIT_FILENAMES = 0b0100_0000_0000_0000_0000;
        const IRIVER_OGG_ALZHEIMER = 0b1000_0000_0000_0000_0000;
        const BROKEN_SEND_OBJECT_PROP_LIST = 0b0001_0000_0000_0000_0000_0000;
        const BROKEN_SET_OBJECT_PROP_LIST = 0b0010_0000_0000_0000_0000_0000;
        const SWITCH_MODE_BLACKBERRY = 0b0100_0000_0000_0000_0000_0000;
    }
}
