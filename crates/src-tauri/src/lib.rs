//! CalendSync Tauri application library.
//!
//! This module contains the shared application logic for both desktop and mobile builds.
//! The `run()` function is the main entry point that configures and starts the Tauri app.

use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;
use tracing::info;

pub mod auth;

/// Main application entry point.
///
/// This function is called from:
/// - `main.rs` for desktop builds
/// - Mobile runtime for iOS builds (via `#[tauri::mobile_entry_point]`)
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            // Open devtools automatically in debug builds for easier debugging
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }

            // Check if the app was launched via a deep link
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                info!("App launched with deep link URLs: {:?}", urls);
                for url in urls {
                    auth::handle_deep_link(app.handle(), url.as_str());
                }
            }

            // Register handler for deep links received while app is running
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                let urls = event.urls();
                info!("Received deep link event: {:?}", urls);
                for url in urls {
                    auth::handle_deep_link(&handle, url.as_str());
                }
            });

            // Register deep link schemes at runtime (needed for development on Windows/Linux)
            #[cfg(any(windows, target_os = "linux"))]
            {
                if let Err(e) = app.deep_link().register_all() {
                    tracing::warn!("Failed to register deep link schemes: {}", e);
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
