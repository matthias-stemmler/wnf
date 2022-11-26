//! [![GitHub](https://img.shields.io/badge/GitHub-informational?logo=GitHub&labelColor=555555)](https://github.com/matthias-stemmler/wnf)
//! [![crates.io](https://img.shields.io/crates/v/wnf.svg)](https://crates.io/crates/wnf)
//! [![docs.rs](https://img.shields.io/docsrs/wnf)](https://docs.rs/wnf/latest/wnf/)
//! [![license](https://img.shields.io/crates/l/wnf.svg)](https://github.com/matthias-stemmler/wnf/blob/main/LICENSE-APACHE)
//! [![rustc 1.62+](https://img.shields.io/badge/rustc-1.62+-lightgrey.svg)](https://blog.rust-lang.org/2022/06/30/Rust-1.62.0.html)
//!
//! Safe Rust bindings for the Windows Notification Facility
//!
//! --Intro text--
//! --This=safe bindings, for raw bindings -> ntapi--
//! --Safe=best effort--
//! --Windows only--
//!
//! # What is WNF
//!
//! --TODO--
//! --In particular, what can you do with a state--
//! --What's not supported: Kernel mode, event aggregation?, meta subscriptions?--
//! --Glossary: State vs. state name--
//!
//! # Representing states
//!
//! --Owned vs. borrowed, see OwnedHandle/BorrowedHandle--
//!
//! # Representing state data
//!
//! --Traits, Safe transmute, macros for bytemuck/zerocopy--
//!
//! # Tracing
//!
//! --TODO--
//!
//! # Cargo features
//!
//! --TODO--
//!
//! # Resources
//!
//! --TODO--
//! --YouTube, Blog posts, Book, Other GitHub projects including list of well-known states--
//!
//! # Stability
//!
//! --TODO--
//!
//! # Minimum Supported Rust Version (MSRV) Policy
//!
//! The current MSRV of this crate is `1.62`.
//!
//! Increasing the MSRV of this crate is _not_ considered a breaking change. However, in such cases there will be at
//! least a minor version bump.
//!
//! Each version //! of this crate will support at least the four latest stable Rust versions at the time it is
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
