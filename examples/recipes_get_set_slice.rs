//! Using the `get` and `set` methods with slices

use std::error::Error;

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use wnf::OwnedState;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = OwnedState::<[u32]>::create_temporary()?;

    let data = vec![1, 2, 3, 4, 5];
    state.set(&data)?;

    let data = state.get_boxed()?;
    info!(?data);

    Ok(())
}
