//! Using the `subscribe` method

use std::error::Error;

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use wnf::{DataAccessor, OwnedState, SeenChangeStamp};

const LAST_DATA: u32 = 10;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .with_span_events(FmtSpan::ACTIVE)
        .with_thread_ids(true)
        .init();

    let state = OwnedState::<u32>::create_temporary()?;
    state.set(&0)?;

    let (tx, rx) = crossbeam_channel::unbounded();

    let subscription = state.subscribe(
        move |accessor: DataAccessor<_>| {
            let (data, change_stamp) = accessor.query().unwrap().into_data_change_stamp();
            info!(data, ?change_stamp);
            tx.send(data).unwrap();
        },
        SeenChangeStamp::None,
    )?;

    for i in 1..=LAST_DATA {
        state.set(&i)?;
    }

    let mut receive_count = 0;

    for data in rx {
        receive_count += 1;
        if data == LAST_DATA {
            break;
        }
    }

    info!(update_count = LAST_DATA + 1, receive_count);

    subscription.unsubscribe()?;

    Ok(())
}
