use tracing::info;
use tracing_subscriber::filter::LevelFilter;

use wnf::{OwnedWnfState, WnfChangeStamp};

fn main() {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = OwnedWnfState::<u32>::create_temporary().expect("Failed to create temporary WNF state");

    let value = 42;
    let updated = state
        .update(&value, WnfChangeStamp::initial())
        .expect("Failed to set WNF state data");
    info!(updated);

    let stamped_data = state.query().expect("Failed to get WNF state data");
    info!(data = stamped_data.data(), change_stamp = %stamped_data.change_stamp());

    let value = 42;
    let updated = state
        .update(&value, WnfChangeStamp::initial())
        .expect("Failed to set WNF state data");
    info!(updated);
}
