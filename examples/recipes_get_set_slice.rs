//! Using the `get` and `set` methods with slices

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use wnf::OwnedState;

fn main() {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = OwnedState::<[u32]>::create_temporary().expect("failed to create temporary state");

    let data = vec![1, 2, 3, 4, 5];
    state.set(&data).expect("failed to set state data");

    let data = state.get_boxed().expect("failed to get state data");
    info!(?data);
}
