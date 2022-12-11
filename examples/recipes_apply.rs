//! Using the `apply` method

use std::error::Error;
use std::sync::Arc;
use std::thread;

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use wnf::OwnedState;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = Arc::new(OwnedState::<u32>::create_temporary()?);
    state.set(&0)?;

    let mut handles = Vec::new();

    for _ in 0..2 {
        let state = Arc::clone(&state);

        handles.push(thread::spawn(move || {
            for _ in 0..5 {
                state.apply(|value| value + 1).unwrap();
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let data = state.get()?;
    info!(data);

    Ok(())
}
