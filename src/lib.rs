#![allow(dead_code)]
#![deny(improper_ctypes)]
#![deny(improper_ctypes_definitions)]

#[macro_use]
extern crate num_derive;

use data::{WnfChangeStamp, WnfStampedData};
use error::WnfCreateError;
use security::SecurityDescriptor;

pub use pod::Pod;
pub use state::{BorrowedWnfState, OwnedWnfState};
pub use state_name::{WnfDataScope, WnfStateName, WnfStateNameDescriptor, WnfStateNameLifetime};

mod data;
mod error;
mod ntdll_sys;
mod pod;
mod raw_state;
mod security;
mod state;
mod state_name;
mod subscription;

// TODO allow specifying minimum change_stamp for subscribe
// TODO maybe extract trait similar to FromBuffer also for query?
// TODO tracing

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use crate::state::OwnedWnfState;

    use super::*;

    #[test]
    fn test() {
        let state = OwnedWnfState::<u32>::create_temporary().unwrap().leak();

        let _handle = state
            .subscribe(Box::new(|data: Option<WnfStampedData<&u32>>| {
                println!("{data:?}");
            }))
            .unwrap();

        state.set(&100).unwrap();

        let join_handle = thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(500));
            state.apply(|x| x + 1).unwrap();
        });

        println!("GET: {:?}", state.get());

        join_handle.join().unwrap();
    }
}
