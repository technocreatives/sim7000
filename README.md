# sim7000_async

This crate provides an [embassy][embassy] driver for the SIM7000 series of modem chips.

*If you are looking for the old synchronous sim7000 driver, it has been archived [here][old-driver].*

**Supported features**:
- [X] TCP connections
- [X] GPS
- [ ] UDP connections
- [ ] SMS
- [ ] A bunch more the other things that the SIM7000 supports

This crate runs on `no_std` and, like embassy, requires nightly Rust (see `rust-toolchain.toml`).
See the `samples` directory for examples.

[embassy]: https://embassy.dev/
[old-driver]: https://github.com/technocreatives/sim7000/tree/old-sync-driver

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
