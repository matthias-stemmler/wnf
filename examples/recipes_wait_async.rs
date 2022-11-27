use std::sync::Arc;
use std::time::Duration;

use tokio::time;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;
use wnf::OwnedState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .with_span_events(FmtSpan::ACTIVE)
        .with_thread_ids(true)
        .init();

    let state = Arc::new(OwnedState::<u32>::create_temporary().expect("Failed to create temporary state"));
    let state2 = Arc::clone(&state);

    let handle = tokio::spawn(async move {
        info!("Waiting ...");

        time::timeout(Duration::from_secs(6), state2.wait_async())
            .await
            .expect("Waiting for state update timed out")
            .expect("Failed to wait for state update");

        info!("State updated");
    });

    time::sleep(Duration::from_secs(3)).await;
    state.set(&0).expect("Failed to update state data");
    handle.await.expect("Failed to join task");
}
