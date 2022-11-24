use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use wnf::OwnedState;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .with_span_events(FmtSpan::ACTIVE)
        .with_thread_ids(true)
        .init();

    let state = Arc::new(OwnedState::<u32>::create_temporary().expect("Failed to create temporary state"));
    let state2 = Arc::clone(&state);

    let handle = thread::spawn(move || {
        info!("Waiting ...");
        state2.wait_blocking().expect("Failed to wait for state update");
        info!("State updated");
    });

    thread::sleep(Duration::from_secs(3));
    state.set(&0).expect("Failed to update state data");
    handle.join().expect("Failed to join thread");
}