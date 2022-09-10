//! To run this, you need the
//! [`SeCreatePermanentPrivilege`](https://docs.microsoft.com/en-us/windows/security/threat-protection/security-policy-settings/create-permanent-shared-objects)
//! privilege
//!
//! Steps to achieve this:
//! 1. Download PsExec from [PsTools](https://download.sysinternals.com/files/PSTools.zip)
//! 2. Build the example using `cargo build --example manage`
//! 3. Run a shell as Administrator, then run `psexec -s %cd%\target\debug\examples\manage.exe`
//!
//! This runs the example under the `LocalSystem` account, which always has the required privilege implicitly

use std::io::{self, Read};
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use wnf::{WnfCreatableStateLifetime, WnfDataScope, WnfStateCreation, WnfStateNameDescriptor};

fn main() {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();

    let state = WnfStateCreation::new()
        .lifetime(WnfCreatableStateLifetime::Permanent { persist_data: true })
        .scope(WnfDataScope::Machine)
        .type_id("1d942789-c358-46fc-a75d-2947c7a8fefa")
        .create_static()
        .expect("Failed to create WNF state");

    let exists = state.exists().expect("Failed to determine if WNF state name exists");
    info!(exists);

    let descriptor: WnfStateNameDescriptor = state
        .state_name()
        .try_into()
        .expect("Failed to convert state name into descriptor");
    info!(?descriptor);

    state.set(0x11223344).expect("Failed to set WNF state data");
    let data = state.get().expect("Failed to get WNF state data");
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
    info!("Press any key to delete the state");

    io::stdin().read(&mut [0u8]).unwrap();

    state.delete().expect("Failed to delete WNF state");

    let exists = state.exists().expect("Failed to determine if WNF state name exists");
    info!(exists);
}
