//! Creating and deleting states
//!
//! This example will elevate itself to run under the `LocalSystem` account.

use std::io::{self, Read};

use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use wnf::{CreatableStateLifetime, DataScope, StateCreation, StateNameDescriptor};

fn main() {
    devutils::ensure_running_as_system().expect("Failed to run as system");

    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = StateCreation::new()
        .lifetime(CreatableStateLifetime::Permanent { persist_data: true })
        .scope(DataScope::Machine)
        .type_id("1d942789-c358-46fc-a75d-2947c7a8fefa")
        .create_static()
        .expect("Failed to create state");

    let exists = state.exists().expect("Failed to determine if state name exists");
    info!(exists);

    let descriptor: StateNameDescriptor = state
        .state_name()
        .try_into()
        .expect("Failed to convert state name into descriptor");
    info!(?descriptor);

    state.set(&0x11223344).expect("Failed to set state data");
    let data = state.get().expect("Failed to get state data");
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

    io::stdin().read_exact(&mut [0u8]).unwrap();

    state.delete().expect("Failed to delete state");

    let exists = state.exists().expect("Failed to determine if state name exists");
    info!(exists);
}
