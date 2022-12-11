//! Using the `replace` method

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

    const NUM_ITERATIONS: usize = 5;

    let mut handles = Vec::new();

    for i in 0..2 {
        let state = Arc::clone(&state);

        handles.push(thread::spawn(move || {
            for j in 0..NUM_ITERATIONS {
                let value = (1 + i * NUM_ITERATIONS + j) as u32;
                let previous_value = state.replace(&value).unwrap();
                info!(value, previous_value);
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
