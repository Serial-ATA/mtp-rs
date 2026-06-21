//! High-level pure Rust implementation of MTP
//!
//! This crate provides the transport layer for the [`mtp_spec`] type definitions.
//!
//! ## Important Terminology
//!
//! * **Initiator**: Your host device, the one initiating operations through this crate
//! * **Responder**: The device that is connected to the initiator, responding to operations (e.g., Your Android phone)
//! * **PTP**: Picture Transfer Protocol, the predecessor to MTP
//!
//! ## What is MTP?
//!
//! Media Transfer Protocol (**MTP**) is a generic protocol allowing lots of different mobile devices
//! to primarily send and receive files, alongside some [other operations] whose availability depends
//! on the device.
//!
//! *For more details, check out the [Wikipedia Article]*
//!
//! [other operations]: communication::operation
//! [Wikipedia Article]: https://en.wikipedia.org/wiki/Media_Transfer_Protocol
//!
//! ## Performance Considerations
//!
//! MTP can be a very inefficient protocol at times, so it is important to fetch data only when
//! needed and **cache** anything possible!
//!
//! ### Object Fetching
//!
//! MTP makes few assumptions about the structure of the device's filesystem and instead operates
//! on [`ObjectHandle`]s, which can have varying levels of [`ObjectInfo`]. Unfortunately, the design makes it
//! *very* slow to build a file tree, because:
//!
//! * A device can send objects in any order, so parents may come after their children
//! * Devices (barring any special [vendor extensions]) will only be able to send objects one-by-one
//!
//! [`ObjectHandle`]: object::ObjectHandle
//! [`ObjectInfo`]: object::ObjectInfo
//!
//! ### Synchronous Nature
//!
//! The protocol is **fully synchronous**, meaning if you send an operation, you must wait for the response (or abort)
//! before being able to send another one.
//!
//! ## Backends
//!
//! Theoretically, MTP can be used over other transports (e.g. Wi-Fi), but for now this crate only
//! supports USB transport. See the [`usb`] module.
//!
//! ## Features
//!
//! * `usb` - The [`usb`] transport backend (*enabled by default*)
//! * `fs` - Enables [`high_level::fs`] for easy filesystem operations (mkdir, rename, etc.) (*enabled by default*)
//! * `time` - Enables the `time` feature of [`mtp_spec`], see the `mtp_spec` crate docs (*enabled by default*)
//!
//! ## Logging
//!
//! `mtp` (and [`mtp_spec`]) use the [`log`] crate to log debug and error information.
//!
//! [`log`]: https://docs.rs/log
//!
//! ## Examples
//!
//! See the [`usb`] module for an example

pub mod error;
#[cfg(feature = "usb")]
pub mod usb;

pub use mtp_spec::*;
pub mod high_level;

#[cfg(feature = "examples")]
pub mod example_utils;
