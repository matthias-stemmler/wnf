use tracing::info;
use tracing_subscriber::filter::LevelFilter;

use wnf::OwnedWnfState;

fn main() {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = OwnedWnfState::create_temporary().expect("Failed to create temporary WNF state");

    let data: Vec<u32> = vec![1, 2, 3, 4, 5];
    state.set_slice(data).expect("Failed to set WNF state data");

    let data: Box<[u32]> = state.get_slice().expect("Failed to get WNF state data");
    info!(?data);
}
