//! Using the `wait_until_async` method

use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use tokio::time;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use wnf::OwnedState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .with_span_events(FmtSpan::ACTIVE)
        .with_thread_ids(true)
        .init();

    let state = Arc::new(OwnedState::<u32>::create_temporary()?);
    let state2 = Arc::clone(&state);

    state.set(&0)?;

    let handle = tokio::spawn(async move {
        info!("Waiting ...");

        let data = time::timeout(Duration::from_secs(6), state2.wait_until_async(|data| *data > 1))
            .await
            .unwrap()
            .unwrap();

        info!(data, "State updated");
    });

    for i in 1..3 {
        time::sleep(Duration::from_secs(1)).await;
        state.set(&i)?;
    }

    handle.await?;

    Ok(())
}
