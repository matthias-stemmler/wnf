#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]
#![deny(missing_debug_implementations)]
#![deny(clippy::missing_safety_doc)]
#![deny(clippy::undocumented_unsafe_blocks)]

#[cfg(not(windows))]
compile_error!("The `wnf` crate supports Windows only");

#[macro_use]
extern crate num_derive;

pub use bytes::*;
pub use data::{ChangeStamp, OpaqueData, StampedData};
pub use manage::{
    CreatableStateLifetime, StateCreation, TryIntoSecurityDescriptor, UnspecifiedLifetime, UnspecifiedScope,
    UnspecifiedSecurityDescriptor, MAXIMUM_STATE_SIZE,
};
pub use privilege::can_create_permanent_shared_objects;
pub use read::{Read, ReadError};
pub use security::{BoxedSecurityDescriptor, SecurityDescriptor};
pub use state::{AsState, BorrowedState, OwnedState};
pub use state_name::{
    DataScope, StateName, StateNameDescriptor, StateNameDescriptorFromStateNameError, StateNameFromDescriptorError,
    StateNameLifetime,
};
pub use subscribe::{DataAccessor, SeenChangeStamp, StateListener, Subscription};
pub use type_id::GUID;

mod apply;
mod bytes;
mod data;
mod info;
mod manage;
mod ntapi;
mod predicate;
mod privilege;
mod query;
mod read;
mod replace;
mod security;
mod state;
mod state_name;
mod subscribe;
mod type_id;
mod update;
mod util;
mod wait_async;
mod wait_blocking;

// TODO check Debug impls
// TODO check error messages for capitalization
// TODO consts for well-known states?
// TODO ZST tests
// TODO check which types are Send/Sync
// TODO trait impls: all for external types, only needed (+Debug) for internal (check generics!)
// TODO tests for error messages
// TODO minimal dependency versions
// TODO test on Windows 11
// TODO document what's not supported (kernel mode? event aggregation? meta subscriptions?)
// TODO real-life examples
// TODO const fn
// TODO configure clippy
// TODO CI
// TODO Which traits should be sealed?
// TODO unit tests (Miri?)
// TODO Safety comments
// TODO documentation
// TODO Wording: state vs. state name
// TODO Compare with ntapi crate
