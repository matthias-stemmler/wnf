use std::sync::Arc;
use std::thread;

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use wnf::OwnedState;

fn main() {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = Arc::new(OwnedState::<u32>::create_temporary().expect("Failed to create temporary state"));
    state.set(&0).expect("Failed to set state data");

    const NUM_ITERATIONS: usize = 5;

    let mut handles = Vec::new();

    for i in 0..2 {
        let state = Arc::clone(&state);

        handles.push(thread::spawn(move || {
            for j in 0..NUM_ITERATIONS {
                let value = (1 + i * NUM_ITERATIONS + j) as u32;
                let previous_value = state.replace(&value).expect("Failed to replace state data");
                info!(value, previous_value);
            }
        }));
    }

    for handle in handles {
        handle.join().expect("Failed to join thread");
    }

    let data = state.get().expect("Failed to get state data");
    info!(data);
}
