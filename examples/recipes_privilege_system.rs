//! Checking for privileges when run as system
//!
//! This example will elevate itself to run under the `LocalSystem` account.

use tracing::info;
use tracing_subscriber::filter::LevelFilter;

fn main() {
    devutils::ensure_running_as_system().expect("Failed to run as system");

    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();
    let has_privilege = wnf::can_create_permanent_shared_objects().expect("Failed to check privilege");
    info!(has_privilege);
}
