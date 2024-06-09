# Safe Rust bindings for the Windows Notification Facility

[![GitHub](https://img.shields.io/badge/GitHub-informational?logo=GitHub&labelColor=555555)](https://github.com/matthias-stemmler/wnf)
[![crates.io](https://img.shields.io/crates/v/wnf.svg)](https://crates.io/crates/wnf)
[![docs.rs](https://img.shields.io/docsrs/wnf)](https://docs.rs/wnf/latest/wnf/)
[![license](https://img.shields.io/crates/l/wnf.svg)](https://github.com/matthias-stemmler/wnf/blob/main/LICENSE-APACHE)
[![rustc 1.70+](https://img.shields.io/badge/rustc-1.70+-lightgrey.svg)](https://blog.rust-lang.org/2023/06/01/Rust-1.70.0.html)

The _Windows Notification Facility (WNF)_ is a registrationless publisher/subscriber mechanism that was introduced in
Windows 8 and forms an undocumented part of the Windows API.

This crate provides safe Rust abstractions over (a part of) this API. If you are looking for raw bindings to the API,
take a look at the [`ntapi`](https://docs.rs/ntapi/latest/ntapi/) crate.

Note that while great care was taken in making these abstractions memory-safe, there cannot be a guarantee due to the
undocumented nature of the API.

## Installation

This crate is available on [crates.io](https://crates.io/crates/wnf). In order to use it, add this to the `dependencies`
table of your `Cargo.toml`:

```toml
[dependencies]
wnf = "0.2.0"
```

Some functionality of this crate is only available if the corresponding
[feature flags](https://doc.rust-lang.org/cargo/reference/features.html) are enabled. For example, in order to enable
the `subscribe` feature:

```toml
[dependencies]
wnf = { version = "0.2.0", features = ["subscribe"] }
```

This is a Windows-only crate and will fail to compile on other platforms. If you target multiple platforms, it is
recommended that you declare it as a
[platform specific dependency](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies):

```toml
[target.'cfg(windows)'.dependencies]
wnf = "0.2.0"
```

## Usage

For a detailed explanation on how to use this crate, see the [crate documentation](https://docs.rs/wnf/latest/wnf/).

For examples, see the [examples](examples) folder.

## Minimum Supported Rust Version (MSRV) Policy

The current MSRV of this crate is `1.70`.

Increasing the MSRV of this crate is _not_ considered a breaking change.
However, in such cases there will be at least a minor version bump. Each version
of this crate will support at least the four latest stable Rust versions at the
time it is published.

## Changelog

See [CHANGELOG.md](CHANGELOG.md)

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  https://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
