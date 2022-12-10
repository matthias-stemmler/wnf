//! Subscribing on the battery discharge estimate

use std::io::{stdin, Read};

use wnf::{BorrowedState, DataAccessor, SeenChangeStamp, StateName};

const WNF_PO_DISCHARGE_ESTIMATE: StateName = StateName::from_opaque_value(0x41C6013DA3BC5075);

fn main() {
    let state = BorrowedState::<u64>::from_state_name(WNF_PO_DISCHARGE_ESTIMATE);

    let _subscription = state
        .subscribe(
            |accessor: DataAccessor<'_, _>| {
                let secs = accessor.get().expect("Failed to query state data");
                let hours = secs / 3600;
                let mins = (secs - hours * 3600) / 60;
                println!("Your battery will last for {hours}h{mins}m, press ENTER to exit ...");
            },
            SeenChangeStamp::None,
        )
        .expect("Failed to subscribe to state changes");

    stdin().read_exact(&mut [0u8]).unwrap();
}
