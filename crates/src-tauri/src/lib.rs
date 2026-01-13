//! CalendSync Tauri application library.
//!
//! This module contains the shared application logic for both desktop and mobile builds.
//! The `run()` function is the main entry point that configures and starts the Tauri app.

use std::sync::Mutex;

use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;
use tokio::sync::oneshot;
use tracing::info;

pub mod auth;
mod commands;
mod http;
mod sse;

/// Managed state for SSE connection.
///
/// Stores connection control and tracking state:
/// - `cancel_tx`: Sender to cancel the current SSE connection
/// - `last_event_id`: Last event ID received (for reconnection catch-up)
pub struct SseState {
    /// Sender to cancel the current SSE connection.
    pub cancel_tx: Mutex<Option<oneshot::Sender<()>>>,
    /// Last event ID received from the SSE stream.
    /// Used for reconnection to resume from last known position.
    pub last_event_id: Mutex<Option<String>>,
}

impl Default for SseState {
    fn default() -> Self {
        Self {
            cancel_tx: Mutex::new(None),
            last_event_id: Mutex::new(None),
        }
    }
}

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
        .manage(SseState::default())
        .invoke_handler(tauri::generate_handler![
            // Storage commands
            commands::get_session,
            commands::set_session,
            commands::clear_session,
            commands::get_last_calendar,
            commands::set_last_calendar,
            commands::clear_last_calendar,
            commands::open_oauth_login,
            // HTTP proxy commands
            commands::exchange_auth_code,
            commands::validate_session,
            commands::logout,
            commands::fetch_my_calendars,
            commands::fetch_entries,
            commands::fetch_entry,
            commands::create_entry,
            commands::update_entry,
            commands::delete_entry,
            commands::toggle_entry,
            // SSE commands
            commands::start_sse,
            commands::stop_sse,
            commands::get_last_event_id,
        ])
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

            // Register deep link schemes at runtime
            // - Windows/Linux: Always needed (no native bundle registration)
            // - macOS: Only needed in debug builds (production uses Info.plist)
            #[cfg(any(windows, target_os = "linux"))]
            {
                if let Err(e) = app.deep_link().register_all() {
                    tracing::warn!("Failed to register deep link schemes: {}", e);
                }
            }

            #[cfg(all(target_os = "macos", debug_assertions))]
            {
                if let Err(e) = app.deep_link().register_all() {
                    tracing::warn!("Failed to register deep link schemes on macOS dev: {}", e);
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
