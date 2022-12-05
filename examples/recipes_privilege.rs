use tracing::info;
use tracing_subscriber::filter::LevelFilter;

fn main() {
    tracing_subscriber::fmt().with_max_level(LevelFilter::DEBUG).init();
    let has_privilege = wnf::can_create_permanent_shared_objects().expect("Failed to check privilege");
    info!(has_privilege);
}
