#![allow(dead_code)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]
#![deny(missing_debug_implementations)]

#[macro_use]
extern crate num_derive;

pub use bytes::*;
pub use data::{WnfChangeStamp, WnfOpaqueData, WnfStampedData};
pub use manage::{UnspecifiedLifetime, UnspecifiedScope, WnfCreatableStateLifetime, WnfStateCreation};
pub use read::{WnfRead, WnfReadError};
pub use security::can_create_permanent_shared_objects;
pub use state::{AsWnfState, BorrowedWnfState, OwnedWnfState};
pub use state_name::{
    WnfDataScope, WnfStateName, WnfStateNameDescriptor, WnfStateNameDescriptorFromStateNameError,
    WnfStateNameFromDescriptorError, WnfStateNameLifetime,
};
pub use subscribe::{WnfDataAccessor, WnfSeenChangeStamp, WnfStampedStateListener, WnfStateListener, WnfSubscription};
pub use type_id::GUID;

mod apply;
mod bytes;
mod data;
mod info;
mod manage;
mod ntdll_sys;
mod predicate;
mod query;
mod read;
mod replace;
mod security;
mod state;
mod state_name;
mod subscribe;
mod type_id;
mod update;
mod wait_async;
mod wait_blocking;

// TODO check Debug impls
// TODO consts for well-known states?
// TODO ZST tests
// TODO check which types are Send/Sync
// TODO trait impls: all for external types, only needed (+Debug) for internal (check generics!)
// TODO tests for error messages
// TODO minimal dependency versions
// TODO scoped subscriptions (without 'static)
// TODO compatibility layer to external crates for security descriptors (windows-permissions, windows, winapi)
// TODO test on Windows 11
// TODO document what's not supported (kernel mode? event aggregation? meta subscriptions?)
// TODO impl CheckedBitPattern/NoUninit for FromPrimitive/ToPrimitive?
// TODO create with security descriptor
// TODO real-life examples
// TODO const fn
// TODO configure clippy
// TODO CI
