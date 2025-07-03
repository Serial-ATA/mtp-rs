//! MTP specification implementation
//!
//! This crate defines all of the datatypes, operations, responses, and events defined by the MTP
//! specification. It is fully `#[no_std]` compatible and is intended to be used by higher-level crates
//! providing transport implementations.
//!
//! See the [`mtp`] crate for an implementation of MTP over USB.
//!
//! [`mtp`]: https://crates.io/crates/mtp
//!
//! ## Features
//!
//! * `time` - Adds [`DateTime::as_systemtime()`] to convert [`DateTime`] into [`SystemTime`] (*enabled by default*)
//!     * NOTE: This enables `std`
//!
//! [`DateTime`]: object::types::DateTime
//! [`SystemTime`]: std::time::SystemTime

#![cfg_attr(not(feature = "time"), no_std)]

extern crate alloc;

pub mod communication;
pub mod device;
pub mod error;
pub mod object;
