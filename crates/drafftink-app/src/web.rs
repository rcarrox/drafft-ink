//! WebAssembly entry point for the personal/local build.

use wasm_bindgen::prelude::*;

/// Compatibility URL parameters retained so legacy auto-join code compiles.
/// The personal/local build intentionally ignores collaboration parameters.
pub struct UrlParams {
    pub room: Option<String>,
    pub server: Option<String>,
}

/// Collaboration is disabled, so no room/server URL parameters are exposed.
pub fn get_url_params() -> UrlParams {
    UrlParams {
        room: None,
        server: None,
    }
}

/// Collaboration is disabled, so there is no WebSocket server URL.
pub fn get_server_url(_override_url: Option<&str>) -> Option<String> {
    None
}

/// Initialize and run the WASM application.
#[wasm_bindgen(start)]
pub async fn run_wasm() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).expect("Failed to initialize logger");
    log::info!("Starting DrafftInk local build (WASM)");

    // Collaboration/room URL parameters are intentionally ignored in this build.
    crate::App::run(None, None).await;
}
