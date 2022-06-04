use std::sync::mpsc;

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;

use wnf::{OwnedWnfState, WnfChangeStamp};

const LAST_DATA: u32 = 10;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .with_span_events(FmtSpan::ACTIVE)
        .with_thread_ids(true)
        .init();

    let state = OwnedWnfState::create_temporary().expect("Failed to create temporary WNF state");
    state.set(0u32).expect("Failed to set WNF state data");

    let (tx, rx) = mpsc::channel();

    let subscription = state
        .subscribe(
            WnfChangeStamp::initial(),
            Box::new(move |data: Option<u32>, change_stamp: WnfChangeStamp| {
                info!(data, ?change_stamp);
                tx.send(data).expect("Failed to send data to mpsc channel");
            }),
        )
        .expect("Failed to subscribe to WNF state changes");

    for i in 1..=LAST_DATA {
        state.set(i).expect("Failed to set WNF state data");
    }

    let mut receive_count: usize = 0;

    for data in rx.into_iter() {
        receive_count += 1;
        if data == Some(LAST_DATA) {
            break;
        }
    }

    info!(update_count = LAST_DATA + 1, receive_count);

    subscription
        .unsubscribe()
        .expect("Failed to unsubscribe from WNF state changes");
}
