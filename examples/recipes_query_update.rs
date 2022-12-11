//! Using the `query` and `update` methods

use std::error::Error;

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use wnf::{ChangeStamp, OwnedState};

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = OwnedState::<u32>::create_temporary()?;

    let value = 42;
    let updated = state.update(&value, ChangeStamp::initial())?;
    info!(updated);

    let stamped_data = state.query()?;
    info!(data = stamped_data.data(), change_stamp = %stamped_data.change_stamp());

    let value = 42;
    let updated = state.update(&value, ChangeStamp::initial())?;
    info!(updated);

    Ok(())
}
