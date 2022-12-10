//! [![GitHub](https://img.shields.io/badge/GitHub-informational?logo=GitHub&labelColor=555555)](https://github.com/matthias-stemmler/wnf)
//! [![crates.io](https://img.shields.io/crates/v/wnf.svg)](https://crates.io/crates/wnf)
//! [![docs.rs](https://img.shields.io/docsrs/wnf)](https://docs.rs/wnf/latest/wnf/)
//! [![license](https://img.shields.io/crates/l/wnf.svg)](https://github.com/matthias-stemmler/wnf/blob/main/LICENSE-APACHE)
//! [![rustc 1.62+](https://img.shields.io/badge/rustc-1.62+-lightgrey.svg)](https://blog.rust-lang.org/2022/06/30/Rust-1.62.0.html)
//!
//! Safe Rust bindings for the Windows Notification Facility
//!
//! The *Windows Notification Facility (WNF)* is a registrationless publisher/subscriber mechanism that was introduced
//! in Windows 8 and forms an undocumented part of the Windows API.
//!
//! This crate provides safe Rust abstractions over (a part of) this API. If you are looking for raw bindings to the
//! API, take a look at the [`ntapi`](https://docs.rs/ntapi/latest/ntapi/) crate.
//!
//! Note that while great care was taken in making these abstractions memory safe, there cannot be a guarantee due to
//! the undocumented nature of the API.
//!
//! This is a Windows-only crate and will fail to compile on other platforms. If you target multiple platforms, it is
//! recommended that you declare it as a
//! [platform specific dependency](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies).
//!
//! # How WNF works
//!
//! WNF is built upon the core concept of a *state name*. Processes can publish to and subscribe to a state name,
//! represented by a 64-bit identifier. In this crate, in order to distinguish between such an identifier and the actual
//! operating system object it represents, we call the identifier a *state name*, while the underlying object will be
//! referred to as a *state*.
//!
//! A state can have different *lifetimes*:
//! - A *well-known state* is provisioned with the system and cannot be created or deleted.
//! - A *permanent* state can be created and stays alive even across system reboots until it is explicitly deleted.
//! - A *persistent* or *volatile* state can be created and stays alive until the next system reboot or until it is
//!   explicitly deleted.
//! - A *temporary* state can be created and stays alive until the system it was created from exits or until it is
//!   explicitly deleted.
//! For details, see [`StateLifetime`].
//!
//! A state has an associated payload, called the *state data* or *state value*, up to 4KB in size. Processes can query
//! and update these data and subscribe to changes of the data. Furthermore, a state has an associated *change stamp*,
//! which starts at zero when the state is created and increases by one on every update of the data.
//!
//! A state that lives across system reboots (i.e. with well-known or permanent lifetime) can be configured to persist
//! its data across reboots. Otherwise, the state itself stays alive but its data is reset on reboots.
//!
//! A state can have different *data scopes* that control whether it maintains multiple independent instances of its
//! data that are scoped in different ways. See [`DataScope`] for the available options.
//!
//! Access to a state is secured by a standard Windows Security Descriptor. In addition, creating a permanent or
//! persistent state or a state with "process" scope requires the `SeCreatePermanentPrivilege` privilege.
//!
//! The WNF mechanism, though officially undocumented, has been described by various sources. Its API is part of the
//! Windows Native API exposed through `ntdll.dll` and has been (partly) reverse engineered and described. For details,
//! refer to these sources:
//! - [A. Allievi et al.: Windows Internals, Part 2, 7th Edition](https://www.microsoftpressstore.com/store/windows-internals-part-2-9780135462331),
//!   p. 224ff.
//! - [Quarkslab's Blog: Playing with the Windows Notification Facility (WNF)](https://blog.quarkslab.com/playing-with-the-windows-notification-facility-wnf.html)
//! - [A. Ionescu, G. Viala: The Windows Notification Facility: Peeling the Onion of the Most Undocumented Kernel Attack
//!   Surface Yet](https://www.youtube.com/watch?v=MybmgE95weo), Talk at black hat USA 2018
//! - [A. Ionescu, G. Viala: WNF Utilities 4 Newbies (WNFUN)](https://github.com/ionescu007/wnfun), including a list of
//!   the names of well-known states
//!
//! # What this crate offers
//!
//! This crate provides memory-safe abstractions over most of the WNF API to accomplish these tasks:
//! - Create and delete a state
//! - Query information on a state
//! - Query and update state data
//! - Subscribe to state data
//!
//! Subscribing uses higher-level functions from `ntdll.dll` whose names start with `Rtl`, standing for *runtime
//! library*:
//! - `RtlSubscribeWnfStateChangeNotification`
//! - `RtlUnsubscribeWnfStateChangeNotification`
//!
//! The other featurs use more low-level functions from `ntdll.dll` whose names start with `Nt*`:
//! - `NtCreateWnfStateName`
//! - `NtDeleteWnfStateName`
//! - `NtQueryWnfStateNameInformation`
//! - `NtQueryWnfStateData`
//! - `NtUpdateWnfStateData`
//!
//! In addition, this crate provides some higher-level abstractions:
//! - Applying a transformation to state data
//! - Replacing state data
//! - Waiting for updates of state data (in both blocking and async variants)
//! - Waiting until state data satisfy a certain condition (in both blocking and async variants)
//!
//! The following WNF features are currently not supported:
//! - Subscriptions in meta-notification mode, i.e. subscribing to consumers becoming active or inactive or publishers
//!   terminating
//! - Event aggregation through the *Common Event Aggregator* to subscribe to updates of one out of multiple states
//! - Kernel mode
//!
//! # Representing states
//!
//! --Owned vs. borrowed, see OwnedHandle/BorrowedHandle--
//!
//! # Representing state data
//!
//! --Traits, Safe transmute, macros for bytemuck/zerocopy--
//!
//! # What to do with states
//!
//! ## Creating states
//! Note that state data should be initialized
//!
//! # Tracing
//!
//! --TODO--
//!
//! # Cargo features
//!
//! --TODO--
//!
//! # Stability
//!
//! Since this crate depends on the WNF API, which is undocumented and hence must be considered unstable, it will
//! probably stay on an unstable `0.x` version forever.
//!
//! # Minimum Supported Rust Version (MSRV) Policy
//!
//! The current MSRV of this crate is `1.62`.
//!
//! Increasing the MSRV of this crate is _not_ considered a breaking change. However, in such cases there will be at
//! least a minor version bump.
//!
//! Each version of this crate will support at least the four latest stable Rust versions at the time it is
//! published.

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(elided_lifetimes_in_paths)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]
#![deny(missing_abi)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(clippy::as_underscore)]
#![deny(clippy::cargo_common_metadata)]
#![deny(clippy::decimal_literal_representation)]
#![deny(clippy::derive_partial_eq_without_eq)]
#![deny(clippy::future_not_send)]
#![deny(clippy::missing_safety_doc)]
#![deny(clippy::non_send_fields_in_send_ty)]
#![deny(clippy::pedantic)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(rustdoc::bare_urls)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::invalid_codeblock_attributes)]
#![deny(rustdoc::invalid_rust_codeblocks)]
#![deny(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::private_intra_doc_links)]
#![allow(clippy::borrow_as_ptr)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::inline_always)]
#![allow(clippy::let_underscore_drop)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::ptr_as_ptr)]
#![allow(clippy::wildcard_imports)]

#[cfg(not(windows))]
compile_error!("The `wnf` crate supports Windows only");

#[macro_use]
extern crate num_derive;

mod apply;
mod bytes;
mod data;
mod info;
mod manage;
mod ntapi;
mod privilege;
mod query;
mod read;
mod replace;
mod security;
mod state;
mod state_name;
mod type_id;
mod update;
mod util;

#[cfg(any(feature = "wait_async", feature = "wait_blocking"))]
mod predicate;

#[cfg(feature = "subscribe")]
mod subscribe;

#[cfg(feature = "wait_async")]
mod wait_async;

#[cfg(feature = "wait_blocking")]
mod wait_blocking;

pub use bytes::*;
pub use data::*;
pub use manage::*;
pub use privilege::*;
pub use read::*;
pub use security::*;
pub use state::*;
pub use state_name::*;
#[cfg(feature = "subscribe")]
pub use subscribe::*;
pub use type_id::*;
#[cfg(feature = "wait_async")]
pub use wait_async::*;
