//! Creating and deleting states
//!
//! This example will elevate itself to run under the `LocalSystem` account.

use std::error::Error;
use std::io::{self, Read};

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use wnf::{CreatableStateLifetime, DataScope, StateCreation, StateNameDescriptor};

fn main() -> Result<(), Box<dyn Error>> {
    devutils::ensure_running_as_system()?;

    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = StateCreation::new()
        .lifetime(CreatableStateLifetime::Permanent { persist_data: true })
        .scope(DataScope::Machine)
        .type_id("1d942789-c358-46fc-a75d-2947c7a8fefa")
        .create_static()?;

    let exists = state.exists()?;
    info!(exists);

    let descriptor: StateNameDescriptor = state.state_name().try_into()?;
    info!(?descriptor);

    state.set(&0x11223344)?;
    let data = state.get()?;
    info!(data = %format!("{data:#10x}"));

    info!(
        "State: see registry at \
        Computer\\HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Notifications\\{:X}",
        state.state_name().opaque_value()
    );
    info!(
        "Data: see registry at \
        Computer\\HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Notifications\\Data\\{:X}",
        state.state_name().opaque_value()
    );
    info!("Press ENTER to delete the state");

    io::stdin().read_exact(&mut [0u8])?;

    state.delete()?;

    let exists = state.exists()?;
    info!(exists);

    Ok(())
}
