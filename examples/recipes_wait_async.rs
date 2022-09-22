use std::sync::Arc;
use std::time::Duration;

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;

use wnf::OwnedWnfState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::TRACE)
        .with_span_events(FmtSpan::ACTIVE)
        .with_thread_ids(true)
        .init();

    let state = Arc::new(OwnedWnfState::<u32>::create_temporary().expect("Failed to create temporary WNF state"));
    let state2 = Arc::clone(&state);

    let handle = tokio::spawn(async move {
        info!("Waiting ...");
        state2.wait_async().await.expect("Failed to wait for WNF state update");
        info!("State updated");
    });

    tokio::time::sleep(Duration::from_secs(3)).await;
    state.set(&0).expect("Failed to update WNF state data");
    handle.await.expect("Failed to join task");
}
