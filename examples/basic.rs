use tracing::info;
use tracing_subscriber::filter::LevelFilter;

use wnf::OwnedWnfState;

fn main() {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = OwnedWnfState::create_temporary().expect("Failed to create temporary WNF state");

    let data: u32 = 42;
    state.set(data).expect("Failed to set WNF state data");

    let data: u32 = state.get().expect("Failed to get WNF state data");
    info!(data);
}
