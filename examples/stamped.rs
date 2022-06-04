use tracing::info;
use tracing_subscriber::filter::LevelFilter;

use wnf::{OwnedWnfState, WnfChangeStamp, WnfStampedData};

fn main() {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = OwnedWnfState::create_temporary().expect("Failed to create temporary WNF state");

    let value: u32 = 42;
    let updated = state
        .update(value, WnfChangeStamp::initial())
        .expect("Failed to set WNF state data");
    info!(updated);

    let stamped_data: WnfStampedData<u32> = state.query().expect("Failed to get WNF state data");
    info!(data = stamped_data.data(), change_stamp = %stamped_data.change_stamp());

    let value: u32 = 42;
    let updated = state
        .update(value, WnfChangeStamp::initial())
        .expect("Failed to set WNF state data");
    info!(updated);
}
