use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use wnf::{BorrowedWnfState, WnfStateName, WnfStateNameDescriptor};

const WNF_PO_ENERGY_SAVER_OVERRIDE: WnfStateName = WnfStateName::from_opaque_value(0x41c6013da3bc3075);

fn main() {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let descriptor: WnfStateNameDescriptor = WNF_PO_ENERGY_SAVER_OVERRIDE
        .try_into()
        .expect("Failed to convert state name into descriptor");

    let state = BorrowedWnfState::<u32>::from_state_name(WNF_PO_ENERGY_SAVER_OVERRIDE);

    let exists = state.exists().expect("Failed to determine if WNF state name exists");

    let subscribers_present = state
        .subscribers_present()
        .expect("Failed to determine if WNF state name has subscribers");

    let is_quiescent = state
        .is_quiescent()
        .expect("Failed to determine if WNF state name is quiescent");

    info!(?descriptor, exists, subscribers_present, is_quiescent);
}
