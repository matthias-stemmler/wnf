#![allow(dead_code)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

#[macro_use]
extern crate num_derive;

use data::{WnfChangeStamp, WnfStampedData};
use security::SecurityDescriptor;

pub use error::{
    SecurityCreateError, WnfApplyError, WnfCreateError, WnfDeleteError, WnfInfoError, WnfQueryError, WnfSubscribeError,
    WnfTransformError, WnfUnsubscribeError, WnfUpdateError,
};
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
// TODO return info from apply about whether an update took place
