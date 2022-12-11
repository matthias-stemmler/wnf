//! Using the `wait_until_blocking` method

use std::error::Error;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use wnf::OwnedState;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .with_span_events(FmtSpan::ACTIVE)
        .with_thread_ids(true)
        .init();

    let state = Arc::new(OwnedState::<u32>::create_temporary()?);
    let state2 = Arc::clone(&state);

    state.set(&0)?;

    let handle = thread::spawn(move || {
        info!("Waiting ...");

        let data = state2
            .wait_until_blocking(|data| *data > 1, Duration::from_secs(6))
            .unwrap();

        info!(data, "State updated");
    });

    for i in 1..3 {
        thread::sleep(Duration::from_secs(1));
        state.set(&i)?;
    }

    handle.join().unwrap();

    Ok(())
}
