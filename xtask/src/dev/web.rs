//! Web server development command with TypeScript hot-reload.

use std::path::Path;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use tokio::process::{Child, Command};
use tokio::sync::mpsc as tokio_mpsc;

use super::error::{DevError, Result};
use crate::prelude::*;

#[derive(Debug, clap::Args)]
pub struct WebOptions {
    /// Port to run the server on
    #[arg(long, short = 'p', default_value = "3000")]
    pub port: u16,

    /// Build in release mode
    #[arg(long)]
    pub release: bool,

    /// Disable TypeScript hot-reload
    #[arg(long)]
    pub no_hot_reload: bool,

    /// Disable browser auto-refresh on hot-reload
    #[arg(long)]
    pub no_auto_refresh: bool,
}

pub async fn run(opts: WebOptions, global: crate::Global) -> Result<()> {
    if !global.is_silent() {
        aprintln!("{} Starting web server on port {}...", p_b("🌐"), opts.port);
    }

    // Start the server process
    let mut server = start_server(&opts)?;

    // In release mode or with --no-hot-reload, just wait for server to exit
    if opts.release || opts.no_hot_reload {
        let status = server.wait().await?;
        if !status.success() {
            return Err(DevError::Io(std::io::Error::other("Server process failed")));
        }
        return Ok(());
    }

    // Hot-reload mode
    if !global.is_silent() {
        aprintln!(
            "{} Hot-reload enabled, watching crates/frontend/src/",
            p_y("👀")
        );
    }

    // Wait for server to be ready before starting watcher
    wait_for_server_ready(opts.port, global.is_silent()).await;

    // Set up file watcher with debouncing using tokio channel
    let (sync_tx, sync_rx) = std::sync::mpsc::channel::<
        std::result::Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>,
    >();
    let (async_tx, mut async_rx) = tokio_mpsc::channel::<()>(1);

    // Bridge sync notify events to async tokio channel
    let async_tx_clone = async_tx.clone();
    std::thread::spawn(move || {
        while let Ok(result) = sync_rx.recv() {
            if let Ok(events) = result {
                // Check if any events are actual modifications
                let has_changes = events
                    .iter()
                    .any(|e| matches!(e.kind, DebouncedEventKind::Any));
                if has_changes {
                    let _ = async_tx_clone.blocking_send(());
                }
            }
        }
    });

    let mut debouncer = new_debouncer(Duration::from_millis(500), sync_tx)?;

    let frontend_src = Path::new("crates/frontend/src");
    debouncer
        .watcher()
        .watch(frontend_src, RecursiveMode::Recursive)?;

    if !global.is_silent() {
        aprintln!("{} Ready for changes", p_g("✓"));
    }

    // Main event loop
    loop {
        tokio::select! {
            // Server process exited
            status = server.wait() => {
                match status {
                    Ok(s) if s.success() => break,
                    Ok(s) => {
                        if !global.is_silent() {
                            aprintln!("{} Server exited with status: {}", p_r("✗"), s);
                        }
                        break;
                    }
                    Err(e) => {
                        if !global.is_silent() {
                            aprintln!("{} Server error: {}", p_r("✗"), e);
                        }
                        break;
                    }
                }
            }

            // File change detected via async channel
            Some(()) = async_rx.recv() => {
                handle_file_change(&opts, &global).await;
            }
        }
    }

    Ok(())
}

/// Start the server process with DEV_MODE enabled.
fn start_server(opts: &WebOptions) -> Result<Child> {
    let mut args = vec!["run", "-p", "calendsync"];
    if opts.release {
        args.push("--release");
    }

    let mut cmd = Command::new("cargo");
    cmd.args(&args)
        .env("PORT", opts.port.to_string())
        .env("DEV_MODE", "1");

    // Pass auto-refresh setting to server
    if opts.no_auto_refresh {
        cmd.env("DEV_NO_AUTO_REFRESH", "1");
    }

    let child = cmd.spawn()?;

    Ok(child)
}

/// Wait for the server to be ready by polling the health endpoint.
/// Returns true if server is ready, false if timeout was reached.
async fn wait_for_server_ready(port: u16, silent: bool) -> bool {
    let client = reqwest::Client::new();
    let url = format!("http://localhost:{}/healthz", port);

    // Try for up to 30 seconds
    for _ in 0..60 {
        tokio::time::sleep(Duration::from_millis(500)).await;

        if client.get(&url).send().await.is_ok() {
            return true;
        }
    }

    // Warn user that server didn't respond
    if !silent {
        aprintln!(
            "{} Server not responding after 30s - continuing anyway",
            p_y("⚠")
        );
        aprintln!(
            "  Check server logs for errors (is something else using port {}?)",
            port
        );
    }

    false
}

/// Handle a file change event.
async fn handle_file_change(opts: &WebOptions, global: &crate::Global) {
    if !global.is_silent() {
        aprintln!("{} Change detected, rebuilding...", p_y("🔄"));
    }

    // Run bun build
    if let Err(e) = run_frontend_build().await {
        if !global.is_silent() {
            aprintln!("{} Build failed: {}", p_r("✗"), e);
        }
        return;
    }

    // Trigger reload
    if let Err(e) = trigger_reload(opts.port).await {
        if !global.is_silent() {
            aprintln!("{} Reload failed: {}", p_r("✗"), e);
        }
        return;
    }

    if !global.is_silent() {
        aprintln!("{} Reloaded!", p_g("✓"));
    }
}

/// Run the frontend build.
async fn run_frontend_build() -> Result<()> {
    let status = Command::new("bun")
        .args(["run", "build:dev"])
        .current_dir("crates/frontend")
        .status()
        .await?;

    if !status.success() {
        return Err(DevError::BuildFailed);
    }

    Ok(())
}

/// Trigger SSR pool reload via HTTP.
async fn trigger_reload(port: u16) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("http://localhost:{}/_dev/reload", port);

    let response = client.post(&url).send().await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(DevError::ReloadFailed(body));
    }

    Ok(())
}
