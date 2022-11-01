use std::sync::Arc;
use std::thread;

use tracing::info;
use tracing_subscriber::filter::LevelFilter;

use wnf::OwnedState;

fn main() {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = Arc::new(OwnedState::<u32>::create_temporary().expect("Failed to create temporary state"));
    state.set(&0).expect("Failed to set state data");

    let mut handles = Vec::new();

    for _ in 0..2 {
        let state = Arc::clone(&state);

        handles.push(thread::spawn(move || {
            for _ in 0..5 {
                state
                    .apply(|v| v + 1)
                    .expect("Failed to apply transformation to state data");
            }
        }));
    }

    for handle in handles {
        handle.join().expect("Failed to join thread");
    }

    let data = state.get().expect("Failed to get state data");
    info!(data);
}
