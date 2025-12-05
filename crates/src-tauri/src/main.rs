//! CalendSync Tauri desktop entry point.
//!
//! This is the main entry point for desktop builds.
//! It simply calls the shared `run()` function from lib.rs.

// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    calendsync_tauri_lib::run()
}
