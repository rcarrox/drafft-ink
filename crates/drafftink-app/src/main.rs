//! Main application entry point (native).
//! Personal/local build: collaboration startup arguments are intentionally disabled.

#[cfg(feature = "native")]
fn main() {
    env_logger::init();
    log::info!("Starting DrafftInk local build");
    pollster::block_on(drafftink_app::App::run(None, None));
}

#[cfg(not(feature = "native"))]
fn main() {
    panic!("Native feature not enabled. Use `cargo run --features native`");
}
