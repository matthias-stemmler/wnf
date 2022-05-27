#![allow(dead_code)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

#[macro_use]
extern crate num_derive;

use data::{WnfChangeStamp, WnfStampedData};
use error::WnfCreateError;
use security::SecurityDescriptor;

pub use state::{BorrowedWnfState, OwnedWnfState};
pub use state_name::{WnfDataScope, WnfStateName, WnfStateNameDescriptor, WnfStateNameLifetime};

mod buffer;
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
// TODO maybe extract trait similar to FromBuffer also for query?
// TODO tracing
