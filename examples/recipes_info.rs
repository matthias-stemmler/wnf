//! Using the `exists`, `subscribers_present` and `is_quiescent` methods

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use wnf::{BorrowedState, StateName, StateNameDescriptor};

const WNF_PO_ENERGY_SAVER_OVERRIDE: u64 = 0x41c6013da3bc3075;

fn main() {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state_name = StateName::from_opaque_value(WNF_PO_ENERGY_SAVER_OVERRIDE);

    let descriptor: StateNameDescriptor = state_name
        .try_into()
        .expect("failed to convert state name into descriptor");

    let state = BorrowedState::<u32>::from_state_name(state_name);

    let exists = state.exists().expect("failed to determine if state name exists");

    let subscribers_present = state
        .subscribers_present()
        .expect("failed to determine if state name has subscribers");

    let is_quiescent = state
        .is_quiescent()
        .expect("failed to determine if state name is quiescent");

    info!(?descriptor, exists, subscribers_present, is_quiescent);
}
