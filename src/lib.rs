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
//! Note that while great care was taken in making these abstractions memory-safe, there cannot be a guarantee due to
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
//! A state can have an associated *type id*, which is a GUID that identifies the type of the data. While WNF does not
//! maintain a registry of types itself, it can ensure that state data are only updated if the correct type id is
//! provided. This can be useful if you maintain your own type registry or just want to avoid accidental updates with
//! invalid data.
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
//! There are two types in this crate that represent states: [`OwnedState<T>`] and [`BorrowedState<'_, T>`]. They work
//! in a similar way as the [`OwnedHandle`](std::os::windows::io::OwnedHandle) and
//! [`BorrowedHandle<'_>`](std::os::windows::io::BorrowedHandle) types from the standard library:
//! - An [`OwnedState<T>`] has no lifetime, does not implement [`Copy`] or [`Clone`] and deletes the represented state
//!   on drop
//! - A [`BorrowedState<'_, T>`] has a lifetime, implements [`Copy`] and [`Clone`] and does not delete the represented
//!   state on drop
//!
//! Note that copying/cloning a [`BorrowedState<'_, T>`] just copies the borrow, returning another borrow of the same
//! underlying state, rather than cloning the state itself.
//!
//! You can abstract over ownership (i.e. whether a state is owned or borrowed) through the [`AsState`] trait and its
//! [`as_state`](AsState::as_state) method. This trait is similar to the [`AsHandle`](std::os::windows::io::AsHandle)
//! trait in the standard library and is implemented by both [`OwnedState<T>`] and [`BorrowedState<'_, T>`] as well as
//! any type that derefs to one of these types. Calling [`as_state`](AsState::as_state) on an [`OwnedState<T>`]
//! borrows it as a [`BorrowedState<'_, T>`] while calling it on a [`BorrowedState<'_, T>`] just copies the borrow.
//!
//! You can obtain an instance of [`OwnedState<T>`] or [`BorrowedState<'_, T>`] in the following ways:
//! - Creating a new owned state through the [`StateCreation::create_owned`] method This is common for temporary states,
//!   for which there is the [`OwnedState::create_temporary`] shortcut method.
//! - Creating a new state and statically borrow it through the [`StateCreation::create_static`] method This is common
//!   for permanent or persistent states.
//! - Statically borrowing an existing state through the [`BorrowedState::from_state_name`] method This is common for
//!   well-known states.
//!
//! An owned state can be "leaked" as a statically borrowed state through the [`OwnedState::leak`] method, while a
//! borrowed state can be turned into an owned state through the [`BorrowedState::to_owned_state`] method.
//!
//! # Representing state data
//!
//! The types [`OwnedState<T>`] and [`BorrowedState<'_, T>`] are generic over a type `T` that describes the shape of the
//! data associated with the state.
//!
//! The state types themselves impose no trait bounds on the data type. However, in order for querying or updating state
//! data to be memory-safe, the data type needs to satisfy certain conditions:
//! - Querying state data as a `T` requires that any byte slice whose length is the size of `T` represent a valid `T` or
//!   that it can at least be checked at runtime whether it represents a valid `T`.
//! - Updating state data from a `T` requires that the type `T` contain no uninitialized (i.e. padding) bytes.
//!
//! These conditions cannot be checked at runtime and hence need to be encoded in the Rust type system.
//!
//! Note that querying state data as a `T` also requires that the size of the state data match the size of `T` in the
//! first place, but this condition can be checked at runtime. In fact, the data type can also be a slice type `[T]`, in
//! which case the size of the state data is required to be a multiple of the size of `T`.
//!
//! Defining how to properly encode the above conditions in the type system is part of the scope of the
//! [Project "safe transmute"](https://github.com/rust-lang/project-safe-transmute), which is still in
//! [RFC](https://github.com/jswrenn/project-safe-transmute/blob/rfc/rfcs/0000-safe-transmute.md#safe-transmute-rfc)
//! stage. However, there are various third-party crates that define (unsafe) traits encoding the above conditions,
//! among them being the [bytemuck](https://docs.rs/bytemuck/1/bytemuck) and
//! [zerocopy](https://docs.rs/zerocopy/0/zerocopy) crates. Both of them implement the appropriate traits for many
//! standard types and also provide macros to derive them for your own types (checking at compile-time whether a type
//! satisfies the necessary conditions), enabling you to avoid unsafe code in most cases.
//!
//! The [`wnf`](crate) crate does not have a hard dependency on any of these crates. Instead, it defines its own
//! (unsafe) traits that are modelled after the traits from the [bytemuck](https://docs.rs/bytemuck/1/bytemuck) crate
//! with the same names:
//! - [`AnyBitPattern`] and [`CheckedBitPattern`] encoding the requirements for querying state data
//! - [`NoUninit`] encoding the requirements for updating state data
//!
//! These traits are already implemented for many standard types. In case your code already makes use of the
//! [bytemuck](https://docs.rs/bytemuck/1/bytemuck) or [zerocopy](https://docs.rs/zerocopy/0/zerocopy) crate or you want
//! to take advantage of the derive macros provided by those crates, you can do the following:
//! - Enable the [`bytemuck_v1`] or [`zerocopy`] feature or the [`wnf`](crate) crate (producing a dependency on
//!   [bytemuck](https://docs.rs/bytemuck/1/bytemuck) v1, respectively [zerocopy](https://docs.rs/zerocopy/0/zerocopy))
//! - Implement the appropriate trait from one of these crates for your type, e.g. by using a derive macro
//! - Derive the corresponding trait from the [`wnf`](crate) crate using the [`derive_from_bytemuck_v1`] respectively
//!   [`derive_from_zerocopy`] macros. See the documentations of these macros for examples.
//! 
//! If you want to be able to support arbitrary state data without any restriction on the size (apart from the upper
//! bound of 4KB), you can always use a byte slice `[u8]` as the data type. In the rare case that you want to query a
//! state without caring about the data at all (e.g. if you want to check if you have the right permissions to query the
//! state), you can use the [`OpaqueData`] type.
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
compile_error!("the `wnf` crate supports Windows only");

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
