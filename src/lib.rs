#![allow(dead_code)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]
#![deny(missing_debug_implementations)]

#[macro_use]
extern crate num_derive;

pub use apply::{WnfApplyError, WnfTransformError};
pub use bytes::NoUninit;
pub use data::{WnfChangeStamp, WnfOpaqueData, WnfStampedData};
pub use info::WnfInfoError;
pub use manage::{WnfCreateError, WnfDeleteError};
pub use query::WnfQueryError;
pub use read::{WnfRead, WnfReadError};
pub use security::SecurityCreateError;
pub use state::{BorrowAsWnfState, BorrowedWnfState, OwnedWnfState};
pub use state_name::{WnfDataScope, WnfStateName, WnfStateNameDescriptor, WnfStateNameLifetime};
pub use subscribe::{WnfDataAccessor, WnfStateListener, WnfSubscribeError, WnfUnsubscribeError};
pub use update::WnfUpdateError;

mod apply;
mod bytes;
mod data;
mod info;
mod manage;
mod ntdll;
mod ntdll_sys;
mod predicate;
mod query;
mod read;
mod replace;
mod security;
mod state;
mod state_name;
mod subscribe;
mod update;
mod wait_async;
mod wait_blocking;

// TODO check Debug impls
// TODO builder pattern for creation, including max size and type_id
// TODO create permanent/persistent states
// TODO consts for well-known states?
// TODO ZST tests
// TODO check which types are Send/Sync
// TODO different subscribe variants with change_stamp 0/current/custom
// TODO consolidate errors
// TODO trait impls: all for external types, only needed (+Debug) for internal
// TODO naming: remove `Wnf` prefixes?
