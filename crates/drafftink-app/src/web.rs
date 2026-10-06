//! WebAssembly entry point for the personal/local build.

use wasm_bindgen::prelude::*;

/// Initialize and run the WASM application.
#[wasm_bindgen(start)]
pub async fn run_wasm() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).expect("Failed to initialize logger");
    log::info!("Starting DrafftInk local build (WASM)");

    // Collaboration/room URL parameters are intentionally ignored in this build.
    crate::App::run(None, None).await;
}
