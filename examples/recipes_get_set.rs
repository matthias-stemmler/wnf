//! Using the `get` and `set` methods

use std::error::Error;

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use wnf::OwnedState;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = OwnedState::<u32>::create_temporary()?;

    let data = 42;
    state.set(&data)?;

    let data = state.get()?;
    info!(data);

    Ok(())
}
