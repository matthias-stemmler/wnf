use std::sync::Arc;
use std::time::Duration;

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

    let state = Arc::new(OwnedState::<[u32]>::create_temporary().expect("Failed to create temporary state"));
    let state2 = Arc::clone(&state);

    state.set(&[]).expect("Failed to update state data");

    let handle = tokio::spawn(async move {
        info!("Waiting ...");
        let data = state2
            .wait_until_boxed_async(|data| data.len() > 1)
            .await
            .expect("Failed to wait for state update");
        info!(data = ?data, "State updated");
    });

    for i in 0..2 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        state
            .apply_boxed(|data| {
                let mut vec = data.into_vec();
                vec.push(i);
                vec.into_boxed_slice()
            })
            .expect("Failed to update state data");
    }

    handle.await.expect("Failed to join task");
}
