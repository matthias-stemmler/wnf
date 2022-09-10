#![allow(dead_code)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]
#![deny(missing_debug_implementations)]

#[macro_use]
extern crate num_derive;

pub use bytes::*;
pub use data::{WnfChangeStamp, WnfOpaqueData, WnfStampedData};
pub use manage::{UnspecifiedLifetime, UnspecifiedScope, WnfCreatableStateLifetime, WnfStateCreation};
pub use read::WnfRead;
pub use security::can_create_permanent_shared_objects;
pub use state::{AsWnfState, BorrowedWnfState, OwnedWnfState};
pub use state_name::{WnfDataScope, WnfStateName, WnfStateNameDescriptor, WnfStateNameLifetime};
pub use subscribe::{WnfDataAccessor, WnfStateListener, WnfSubscription};
pub use type_id::GUID;

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
mod type_id;
mod update;
mod wait_async;
mod wait_blocking;

// TODO check Debug impls
// TODO consts for well-known states?
// TODO ZST tests
// TODO check which types are Send/Sync
// TODO different subscribe variants with change_stamp 0/current/custom
// TODO trait impls: all for external types, only needed (+Debug) for internal
// TODO tests for error messages
// TODO minimal dependency versions
// TODO scoped subscriptions (without 'static)
// TODO compatibility layer to external crates for security descriptors (windows-permissions, windows, winapi)
// TODO crate-internal imports via module, not via crate::
// TODO test on Windows 11
// TODO document what's not supported (kernel mode? event aggregation? meta subscriptions?)
// TODO impl CheckedBitPattern/NoUninit for FromPrimitive/ToPrimitive?
// TODO create with security descriptor
// TODO real-life examples
// TODO const fn
// TODO use pub(crate) consistently
// TODO use AsRef instead of Borrow?
// TODO easier state borrowing from state name by accepting Into<WnfStateName>
