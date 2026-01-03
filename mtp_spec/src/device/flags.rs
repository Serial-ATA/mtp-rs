bitflags::bitflags! {
    /// Flags indicating the bugs/unexpected behaviors of MTP devices
    ///
    /// These flags match device flags of `libmtp` here: <https://sourceforge.net/p/libmtp/code/ci/master/tree/src/device-flags.h>
    #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct DeviceFlags: u32 {
        const BROKEN_MTP_GET_OBJECT_PROP_LIST_ALL = 0b0000_0001;
        const NO_RELEASE_INTERFACE = 0b0000_0010;
        const IGNORE_HEADER_ERRORS = 0b0000_0100;
        const BROKEN_SET_SAMPLE_DIMENSIONS = 0b0001_0000;
        const UNLOAD_DRIVER = 0b0010_0000;
        const BROKEN_MTP_GET_OBJECT_PROP_LIST = 0b0100_0000;
        const ALWAYS_PROBE_DESCRIPTOR = 0b1000_0000;
        const CANNOT_HANDLE_DATEMODIFIED = 0b0001_0000_0000;
        const OGG_IS_UNKNOWN = 0b0010_0000_0000;
        /// The playlist format is the Samsung SPL format v1.00, rather than a proper MTP playlist.
        const PLAYLIST_SPL_V1 = 0b0100_0000_0000;
        const NO_ZERO_READS = 0b1000_0000_0000;
        /// The playlist format is the Samsung SPL format v2.00, rather than a proper MTP playlist.
        const PLAYLIST_SPL_V2 = 0b0001_0000_0000_0000;
        /// The device needs unique filenames, no two files can be named the same string.
        const UNIQUE_FILENAMES = 0b0010_0000_0000_0000;
        const BROKEN_BATTERY_LEVEL = 0b0100_0000_0000_0000;
        /// The device may need additional time to respond, extend its timeout
        const LONG_TIMEOUT = 0b1000_0000_0000_0000;
        const PROPLIST_OVERRIDES_OI = 0b0001_0000_0000_0000_0000;
        const SAMSUNG_OFFSET_BUG = 0b0010_0000_0000_0000_0000;
        const FLAC_IS_UNKNOWN = 0b0100_0000_0000_0000_0000;
        const ONLY_7BIT_FILENAMES = 0b1000_0000_0000_0000_0000;
        const IRIVER_OGG_ALZHEIMER = 0b0001_0000_0000_0000_0000_0000;
        const BROKEN_SEND_OBJECT_PROP_LIST = 0b0010_0000_0000_0000_0000_0000;
        const BROKEN_SET_OBJECT_PROP_LIST = 0b1000_0000_0000_0000_0000_0000;
        const SWITCH_MODE_BLACKBERRY = 0b0001_0000_0000_0000_0000_0000_0000;
        const FORCE_RESET_ON_CLOSE = 0b0010_0000_0000_0000_0000_0000_0000;

        /// Bugs on all devices using the Android MTP stack
        const ANDROID_BUGS = Self::BROKEN_MTP_GET_OBJECT_PROP_LIST_ALL.bits()
        | Self::BROKEN_SET_OBJECT_PROP_LIST.bits()
        | Self::BROKEN_SEND_OBJECT_PROP_LIST.bits()
        | Self::UNLOAD_DRIVER.bits()
        | Self::LONG_TIMEOUT.bits()
        | Self::FORCE_RESET_ON_CLOSE.bits();

        /// Bugs on SONY NWZ Walkman players
        const SONY_NWZ_BUGS = Self::UNLOAD_DRIVER.bits()
        | Self::BROKEN_MTP_GET_OBJECT_PROP_LIST.bits()
        | Self::UNIQUE_FILENAMES.bits()
        | Self::FORCE_RESET_ON_CLOSE.bits();

        /// Bugs on devices using the Aricent MTP stack
        const ARICENT_BUGS = Self::IGNORE_HEADER_ERRORS.bits()
        | Self::BROKEN_SEND_OBJECT_PROP_LIST.bits()
        | Self::BROKEN_MTP_GET_OBJECT_PROP_LIST.bits();
    }
}
