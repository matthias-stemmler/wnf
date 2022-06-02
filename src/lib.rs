#![allow(dead_code)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

#[macro_use]
extern crate num_derive;

pub use data::{WnfChangeStamp, WnfStampedData};
pub use error::{
    SecurityCreateError, WnfApplyError, WnfCreateError, WnfDeleteError, WnfInfoError, WnfQueryError, WnfSubscribeError,
    WnfTransformError, WnfUnsubscribeError, WnfUpdateError,
};
use security::SecurityDescriptor;
pub use state::{BorrowedWnfState, OwnedWnfState};
pub use state_name::{WnfDataScope, WnfStateName, WnfStateNameDescriptor, WnfStateNameLifetime};

mod bytes;
mod data;
mod error;
mod ntdll_sys;
mod raw_state;
mod security;
mod state;
mod state_name;
mod subscription;

// TODO allow specifying minimum change_stamp for subscribe
// TODO tracing
// TODO wrap API for querying state name information?
