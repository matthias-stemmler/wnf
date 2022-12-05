//! Using the `query` and `update` methods

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use wnf::{ChangeStamp, OwnedState};

fn main() {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = OwnedState::<u32>::create_temporary().expect("Failed to create temporary state");

    let value = 42;
    let updated = state
        .update(&value, ChangeStamp::initial())
        .expect("Failed to set state data");
    info!(updated);

    let stamped_data = state.query().expect("Failed to get state data");
    info!(data = stamped_data.data(), change_stamp = %stamped_data.change_stamp());

    let value = 42;
    let updated = state
        .update(&value, ChangeStamp::initial())
        .expect("Failed to set state data");
    info!(updated);
}
