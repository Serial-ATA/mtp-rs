//! Abstractions over MTP responder devices
//!
//! This module contains two key traits:
//!
//! * [`Device`]: Convenience trait providing simple methods for sending [`operations`](crate::communication::operation)
//! * [`PtpIo`]: The backing I/O interface used by [`Device`], implemented by higher-level crates
//!   providing transport implementations

use crate::communication::operation::{GetDeviceInfo, OpenSession};
use crate::communication::response::Response;
use crate::communication::{SessionId, TransactionId};
use crate::error::MtpError;

pub mod extensions;
mod flags;
pub use flags::*;
mod info;
pub use info::*;
mod io;
pub use io::*;
pub mod properties;
pub mod session;
pub mod storage;

/// An MTP responder device
///
/// This is a convenience trait to perform operations on a responder without having to interact with
/// [`operations`] directly.
///
/// [`operations`]: crate::communication::operation
pub trait Device: PtpIo + Sync {
    // === Property checking ===

    /// Get the [`DeviceFlags`] for this device
    ///
    /// These flags may be used behind-the-scenes for certain high-level operations.
    fn flags(&self) -> DeviceFlags;

    /// Check whether the device claims to support Android MTP extensions
    ///
    /// If this returns `true`, is *should* be safe to use the methods from [`AndroidDevice`].
    ///
    /// [`AndroidDevice`]: extensions::android::AndroidDevice
    fn is_android(
        &self,
    ) -> impl Future<Output = Result<bool, MtpError<<Self as PtpIo>::TransportError>>> {
        async move {
            let response = self.get_device_info().await?;

            let device_info = response.data.data;
            let extensions = device_info.mtp_extensions.to_string();

            Ok(extensions.contains("android.com"))
        }
    }

    // === Operation wrappers ===

    /// Send a [`GetDeviceInfo`] operation
    fn get_device_info(
        &self,
    ) -> impl Future<Output = Response<GetDeviceInfo, MtpError<<Self as PtpIo>::TransportError>>> + Send
    {
        async move {
            self.send_operation(OperationBundle::new(
                GetDeviceInfo::new(TransactionId::NONE, SessionId::NONE),
                None,
            )?)
            .await
        }
    }

    /// Send a [`OpenSession`] operation
    ///
    /// NOTE: This is just a raw operation wrapper. You likely want to use [`MtpSession::open()`] instead.
    ///
    /// [`MtpSession::open()`]: session::MtpSession
    fn open_session(
        &self,
    ) -> impl Future<
        Output = Result<
            (crate::communication::response::Empty, SessionId),
            MtpError<<Self as PtpIo>::TransportError>,
        >,
    > {
        async move {
            let transaction_id = self.next_transaction_id();
            let session_id = self.next_session_id();
            self.send_operation(OperationBundle::new(
                OpenSession::new(transaction_id, session_id),
                None,
            )?)
            .await
            .map(|res| (res.data, session_id))
        }
    }
}
