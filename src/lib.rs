#![allow(dead_code)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

#[macro_use]
extern crate num_derive;

pub use apply::{WnfApplyError, WnfTransformError};
pub use callback::{CatchInvalidExt, WnfCallback};
pub use data::{WnfChangeStamp, WnfStampedData};
pub use info::WnfInfoError;
pub use manage::{WnfCreateError, WnfDeleteError};
pub use query::WnfQueryError;
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

// TODO apply: abstract over returning T vs. Option<T>?
// TODO wait (sync + async)
// TODO check Debug impls
// TODO builder pattern for creation, including max size
// TODO create permanent/persistent states
// TODO consts for well-known states?
// TODO ZST tests
