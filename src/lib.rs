#![allow(dead_code)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

#[macro_use]
extern crate num_derive;

pub use callback::{CatchInvalidExt, WnfCallback};
pub use data::{WnfChangeStamp, WnfStampedData};
pub use error::{
    SecurityCreateError, WnfApplyError, WnfCreateError, WnfDeleteError, WnfInfoError, WnfQueryError, WnfSubscribeError,
    WnfTransformError, WnfUnsubscribeError, WnfUpdateError,
};
use security::SecurityDescriptor;
pub use state::{BorrowedWnfState, OwnedWnfState};
pub use state_name::{WnfDataScope, WnfStateName, WnfStateNameDescriptor, WnfStateNameLifetime};

mod bytes;
mod callback;
mod data;
mod error;
mod ntdll;
mod ntdll_sys;
mod raw_state;
mod security;
mod state;
mod state_name;
mod subscription;

// TODO move single vs. slice from methods to types?
// TODO apply: abstract over returning T vs. Option<T>?
// TODO restructure modules by query, update, subscribe etc.
// TODO wait (sync + async)
// TODO check Debug impls
