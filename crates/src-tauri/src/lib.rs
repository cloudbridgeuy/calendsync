//! CalendSync Tauri application library.
//!
//! This module contains the shared application logic for both desktop and mobile builds.
//! The `run()` function is the main entry point that configures and starts the Tauri app.

use tauri::Manager;

/// Main application entry point.
///
/// This function is called from:
/// - `main.rs` for desktop builds
/// - Mobile runtime for iOS builds (via `#[tauri::mobile_entry_point]`)
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .setup(|app| {
            // Open devtools automatically in debug builds for easier debugging
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
