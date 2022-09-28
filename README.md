# sim7000

This crate provides drivers for the SIM7000 series of chips. The current code implements enough commands to bring up a TCP connection and run GPS.

This crate runs on `no_std` and requires nightly Rust (see `rust-toolchain.toml`).


# Getting Started

This project relies on `probe-run`: https://github.com/knurling-rs/probe-run

Install it via:

`cargo install probe-run`

Add target toolchain:

`rustup target add thumbv7em-none-eabihf`

The following ENV VARS need to be set:

DEFMT_LOG=info

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
