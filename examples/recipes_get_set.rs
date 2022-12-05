//! Using the `get` and `set` methods

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use wnf::OwnedState;

fn main() {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = OwnedState::<u32>::create_temporary().expect("Failed to create temporary state");

    let data = 42;
    state.set(&data).expect("Failed to set state data");

    let data = state.get().expect("Failed to get state data");
    info!(data);
}
