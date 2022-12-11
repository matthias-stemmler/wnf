//! Subscribing on the battery discharge estimate

use std::error::Error;
use std::io::{stdin, Read};

use wnf::{BorrowedState, DataAccessor, SeenChangeStamp, StateName};

const WNF_PO_DISCHARGE_ESTIMATE: StateName = StateName::from_opaque_value(0x41C6013DA3BC5075);

fn main() -> Result<(), Box<dyn Error>> {
    let state = BorrowedState::<u64>::from_state_name(WNF_PO_DISCHARGE_ESTIMATE);

    let _subscription = state.subscribe(
        |accessor: DataAccessor<'_, _>| {
            let secs = accessor.get().unwrap();
            let hours = secs / 3600;
            let mins = (secs - hours * 3600) / 60;
            println!("Your battery will last for {hours}h{mins}m, press ENTER to exit ...");
        },
        SeenChangeStamp::None,
    )?;

    stdin().read_exact(&mut [0u8])?;
    Ok(())
}
