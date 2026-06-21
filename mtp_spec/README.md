# mtp_spec

[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/Serial-ATA/mtp-rs/ci.yml?branch=master&style=for-the-badge&logo=github)](https://github.com/Serial-ATA/mtp-rs/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/crates/d/mtp_spec?style=for-the-badge&logo=rust)](https://crates.io/crates/mtp)
[![Version](https://img.shields.io/crates/v/mtp_spec?style=for-the-badge&logo=rust)](https://crates.io/crates/mtp)
[![Documentation](https://img.shields.io/badge/docs.rs-mtp_spec-informational?style=for-the-badge&logo=read-the-docs)](https://docs.rs/mtp_spec/)

A `#![no_std]`-compatible full implementation of the MTP specification.

This crate provides all of the types in the MTP v1.1 specification as well as the *building blocks* for implementing
a transport layer. For an actual implementation of MTP over USB, see [`mtp`](https://crates.io/crates/mtp).

## License

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
  ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
