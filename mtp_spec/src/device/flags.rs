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
        /// The device can't handle modifications to the [`RepresentativeSampleWidth`] and [`RepresentativeSampleHeight`] properties
        ///
        /// [`RepresentativeSampleWidth`]: crate::object::properties::RepresentativeSampleWidth
        /// [`RepresentativeSampleHeight`]: crate::object::properties::RepresentativeSampleHeight
        const BROKEN_SET_SAMPLE_DIMENSIONS = 0b0000_0100;
        /// The [`GetObjectPropList`] operation doesn't work correctly in some way
        ///
        /// [`GetObjectPropList`]: crate::communication::operation::GetObjectPropList
        const BROKEN_MTP_GET_OBJECT_PROP_LIST = 0b0000_1000;
        /// The device claims to support writing the [`DateModified`] property, but writes will fail
        ///
        /// [`DateModified`]: crate::object::types::properties::DateModified
        const CANNOT_HANDLE_DATEMODIFIED = 0b1000_0000;
        /// The device supports OGG files, but needs to be forced to accept them
        ///
        /// Devices with this flag:
        ///
        /// * Do not explicitly claim support for OGG files
        /// * Report the format of OGG objects as [`ObjectFormatCode::Undefined`]
        /// * Need the format code to be overwritten to [`ObjectFormatCode::Undefined`] when sending/modifying
        ///   OGG objects
        ///
        /// [`ObjectFormatCode::Undefined`]: crate::object::ObjectFormatCode::Undefined
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
        /// The device returns bad data in the [`GetObjectInfo`] operation, use [`GetObjectPropList`] instead
        ///
        /// [`GetObjectInfo`]: crate::communication::operation::GetObjectInfo
        /// [`GetObjectPropList`]: crate::communication::operation::GetObjectPropList
        const PROPLIST_OVERRIDES_OI = 0b1000_0000_0000_0000;
        /// Same as [`DeviceFlags::OGG_IS_UNKNOWN`], but for FLAC files
        const FLAC_IS_UNKNOWN = 0b0010_0000_0000_0000_0000;
        /// The device only supports ASCII filenames, despite the spec requiring ISO 10646 support
        const ONLY_7BIT_FILENAMES = 0b0100_0000_0000_0000_0000;
        /// Unlike [`DeviceFlags::OGG_IS_UNKNOWN`], the device claims OGG support, but sometimes forgets
        /// to set the [`ObjectFormatCode`] correctly
        ///
        /// [`ObjectFormatCode`]: crate::object::ObjectFormatCode
        const IRIVER_OGG_ALZHEIMER = 0b1000_0000_0000_0000_0000;
        /// The [`SendObjectPropList`] operation doesn't work correctly in some way
        ///
        /// [`SendObjectPropList`]: crate::communication::operation::SendObjectPropList
        const BROKEN_SEND_OBJECT_PROP_LIST = 0b0001_0000_0000_0000_0000_0000;
        /// The [`SetObjectPropList`] operation doesn't work correctly in some way
        ///
        /// [`SetObjectPropList`]: crate::communication::operation::SetObjectPropList
        const BROKEN_SET_OBJECT_PROP_LIST = 0b0010_0000_0000_0000_0000_0000;
        /// Special flag for BlackBerry devices to attempt a switch from USB mass storage to MTP mode
        const SWITCH_MODE_BLACKBERRY = 0b0100_0000_0000_0000_0000_0000;
    }
}
