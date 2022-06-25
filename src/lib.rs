#![allow(dead_code)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]
#![deny(missing_debug_implementations)]

#[macro_use]
extern crate num_derive;

pub use apply::{WnfApplyError, WnfTransformError, WnfTransformResult};
pub use callback::{WnfCallbackMaybeInvalid, WnfCallbackOnResult};
pub use data::{WnfChangeStamp, WnfStampedData};
pub use info::WnfInfoError;
pub use manage::{WnfCreateError, WnfDeleteError};
pub use query::WnfQueryError;
pub use read::WnfReadError;
pub use security::SecurityCreateError;
pub use state::{BorrowedWnfState, OwnedWnfState};
pub use state_name::{WnfDataScope, WnfStateName, WnfStateNameDescriptor, WnfStateNameLifetime};
pub use subscribe::{WnfSubscribeError, WnfUnsubscribeError};
pub use update::WnfUpdateError;

mod apply;
mod bytes;
mod callback;
mod data;
mod info;
mod manage;
mod ntdll;
mod ntdll_sys;
mod query;
mod read;
mod security;
mod state;
mod state_name;
mod subscribe;
mod update;

// TODO Use callback varargs mechanism for apply
// TODO different api for .catch_invalid
// TODO implement replace in terms of apply

// TODO wait (sync + async)
// TODO check Debug impls
// TODO builder pattern for creation, including max size and type_id
// TODO create permanent/persistent states
// TODO consts for well-known states?
// TODO ZST tests
