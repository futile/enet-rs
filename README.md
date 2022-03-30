# High-Level, Rust-y Bindings for the ENet library

[![Documentation](https://docs.rs/enet/badge.svg)](https://docs.rs/enet)
[![Crates.io](https://img.shields.io/crates/v/enet.svg)](https://crates.io/crates/enet)
[![License](https://img.shields.io/crates/l/enet.svg)](https://github.com/futile/enet-rs)

This crate aims to provide high-level, rust-y bindings for the ENet library.
ENet is a networking library for games that builds on UDP,
offering optional reliability, congestion control, connection-orientation and
other related features. For more information, check out the
[ENet Website](http://enet.bespin.org).

## Status

For now, this library is **alpha**. It builds on the C-bindings for ENet,
the [enet-sys crate](https://github.com/ruabmbua/enet-sys). A lot of the
functionality is there, but not everything. Also, since ENet has
pretty unclear lifetime semantics, you might actually run into cases where
things crash. **In those cases, or when something is missing/not yet in the API,
open a bug report, and I will look into it as soon as possible.**

## Usage

To check what the latest released version is, check on
[https://crates.io/crates/enet](crates.io), or use `cargo add` from 
[cargo edit](https://github.com/killercup/cargo-edit) to automatically add a
dependency to the most recent version.

Installation is as simple as adding this to your `Cargo.toml`:

```toml
[dependencies]
enet = "0.3.0"
```

## Documentation & Examples

Documentation is available on https://docs.rs/enet or by running `cargo doc`.
An example server and client can be found in the `examples` directory.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
