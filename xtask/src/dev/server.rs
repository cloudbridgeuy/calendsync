//! Development server command with TypeScript hot-reload.
//!
//! This module implements the `cargo xtask dev server` command, which manages:
//! - Storage backends (inmemory, sqlite, dynamodb)
//! - Cache backends (memory, redis)
//! - Container orchestration for DynamoDB and Redis
//! - TypeScript hot-reload for frontend development
//! - HTTP-based data seeding
//!
//! ## Execution Flow
//!
//! 1. **Option Resolution**: Parse CLI args, detect container runtime
//! 2. **Container Management**: Start required containers (DynamoDB/Redis)
//! 3. **Infrastructure Setup**: Deploy DynamoDB table, ensure SQLite directory
//! 4. **Server Execution**: Build and run with correct features
//! 5. **Seeding and Cleanup**: Seed data via HTTP, cleanup containers on exit

use std::path::Path;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use tokio::process::{Child, Command};
use tokio::sync::mpsc as tokio_mpsc;

use super::containers::{self, Cache, ContainerPorts, ContainerRuntime, ContainerSpec, Storage};
use super::error::{DevError, Result};
use super::seed;
use crate::prelude::*;

#[derive(Debug, clap::Args)]
pub struct ServerOptions {
    /// Port to run the server on
    #[arg(long, short = 'p', default_value = "3000", env = "PORT")]
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

    /// Storage backend: inmemory (default), sqlite, dynamodb
    #[arg(long, default_value = "inmemory", env = "CALENDSYNC_STORAGE")]
    pub storage: Storage,

    /// Cache backend: memory (default), redis
    #[arg(long, default_value = "memory", env = "CALENDSYNC_CACHE")]
    pub cache: Cache,

    /// Use podman instead of docker
    #[arg(long, env = "CALENDSYNC_PODMAN")]
    pub podman: bool,

    /// Remove existing volumes before starting containers
    #[arg(long)]
    pub flush: bool,

    /// Seed the database with demo data via HTTP after startup
    #[arg(long)]
    pub seed: bool,

    /// Keep containers running on error (default: stop containers)
    #[arg(long)]
    pub keep_containers: bool,

    /// Open browser to calendar URL after seeding (macOS only)
    #[arg(long)]
    pub open: bool,
}

pub async fn run(opts: ServerOptions, global: crate::Global) -> Result<()> {
    // =========================================================================
    // Stage 1: Option Resolution
    // =========================================================================
    let features = containers::cargo_features(opts.storage, opts.cache);
    let required = containers::required_containers(opts.storage, opts.cache);

    if !global.is_silent() {
        aprintln!(
            "{} Starting development server on port {}",
            p_b("🌐"),
            opts.port
        );
        aprintln!(
            "   Storage: {}, Cache: {}, Features: {}",
            format!("{:?}", opts.storage).to_lowercase(),
            format!("{:?}", opts.cache).to_lowercase(),
            p_y(&features)
        );
    }

    // Detect container runtime if containers are needed
    let runtime = if !required.is_empty() {
        Some(containers::detect_runtime(opts.podman).await?)
    } else {
        None
    };

    // Track started containers for cleanup
    let mut started_containers: Vec<&'static ContainerSpec> = Vec::new();

    // Track discovered container ports
    let mut container_ports = ContainerPorts::default();

    // =========================================================================
    // Stage 2: Container Management
    // =========================================================================
    if let Some(runtime) = runtime {
        let verbose = global.is_verbose();

        // Handle --flush: remove and recreate volume directories
        if opts.flush {
            for spec in &required {
                if !global.is_silent() {
                    aprintln!(
                        "{} Flushing volume directory for {}...",
                        p_y("🗑"),
                        spec.name
                    );
                }
                containers::flush_volume_directory(spec, verbose)?;
            }
        }

        // Start required containers
        for spec in &required {
            if !global.is_silent() {
                aprintln!("{} Starting {}...", p_b("🐳"), spec.name);
            }

            match containers::start_container(runtime, spec, verbose).await {
                Ok(()) => {
                    started_containers.push(spec);
                }
                Err(e) => {
                    // Cleanup on error unless --keep-containers
                    if !opts.keep_containers {
                        cleanup_containers(runtime, &started_containers, &global).await;
                    }
                    return Err(e);
                }
            }

            // Query the actual host port assigned by Docker/Podman
            let actual_port = match containers::get_container_port(
                runtime, spec.name, spec.port, verbose,
            )
            .await
            {
                Ok(port) => port,
                Err(e) => {
                    if !opts.keep_containers {
                        cleanup_containers(runtime, &started_containers, &global).await;
                    }
                    return Err(e);
                }
            };

            // Store the discovered port
            if spec.name == containers::DYNAMODB_SPEC.name {
                container_ports.dynamodb = Some(actual_port);
            } else if spec.name == containers::REDIS_SPEC.name {
                container_ports.redis = Some(actual_port);
            }

            if !global.is_silent() {
                aprintln!(
                    "   Container port: {} -> localhost:{}",
                    spec.port,
                    actual_port
                );
            }

            // Wait for health
            if !global.is_silent() {
                aprintln!("{} Waiting for {} to be healthy...", p_b("⏳"), spec.name);
            }

            match containers::wait_for_health(runtime, spec, actual_port, Duration::from_secs(30))
                .await
            {
                Ok(()) => {
                    if !global.is_silent() {
                        aprintln!("{} {} is ready", p_g("✓"), spec.name);
                    }
                }
                Err(e) => {
                    if !opts.keep_containers {
                        cleanup_containers(runtime, &started_containers, &global).await;
                    }
                    return Err(e);
                }
            }
        }
    }

    // =========================================================================
    // Stage 3: Infrastructure Setup
    // =========================================================================
    // For DynamoDB: deploy table schema
    if opts.storage == Storage::Dynamodb {
        if !global.is_silent() {
            aprintln!("{} Deploying DynamoDB table schema...", p_b("📦"));
        }

        let dynamodb_port = container_ports.dynamodb.unwrap_or(8000);
        let deploy_result = deploy_dynamodb_table(dynamodb_port).await;
        if let Err(e) = deploy_result {
            if !opts.keep_containers {
                if let Some(runtime) = runtime {
                    cleanup_containers(runtime, &started_containers, &global).await;
                }
            }
            return Err(DevError::Io(std::io::Error::other(format!(
                "DynamoDB table deployment failed: {e}"
            ))));
        }

        if !global.is_silent() {
            aprintln!("{} DynamoDB table ready", p_g("✓"));
        }
    }

    // For SQLite: ensure data directory exists
    if opts.storage == Storage::Sqlite {
        let data_dir = Path::new(".local/data");
        if !data_dir.exists() {
            std::fs::create_dir_all(data_dir)?;
            if !global.is_silent() {
                aprintln!("{} Created SQLite data directory", p_g("✓"));
            }
        }
    }

    // =========================================================================
    // Stage 4: Server Execution
    // =========================================================================
    let mut server = match start_server_with_features(&opts, &features, &container_ports) {
        Ok(child) => child,
        Err(e) => {
            if !opts.keep_containers {
                if let Some(runtime) = runtime {
                    cleanup_containers(runtime, &started_containers, &global).await;
                }
            }
            return Err(e);
        }
    };

    // In release mode or with --no-hot-reload, handle simplified flow
    if opts.release || opts.no_hot_reload {
        // Wait for server to be ready for seeding
        if opts.seed {
            let base_url = format!("http://localhost:{}", opts.port);
            if !global.is_silent() {
                aprintln!("{} Waiting for server to be ready...", p_b("⏳"));
            }

            if let Err(e) = seed::wait_for_server(&base_url, Duration::from_secs(60)).await {
                if !opts.keep_containers {
                    if let Some(runtime) = runtime {
                        cleanup_containers(runtime, &started_containers, &global).await;
                    }
                }
                let _ = server.kill().await;
                return Err(e);
            }

            // Seed via HTTP
            match seed::seed_via_http(&base_url, global.is_silent()).await {
                Ok(calendar_id) => {
                    let calendar_url = format!("{}/calendar/{}", base_url, calendar_id);
                    if !global.is_silent() {
                        aprintln!("{} Seeding complete!", p_g("✓"));
                        aprintln!("   Calendar URL: {}", p_b(&calendar_url));
                    }

                    // Open browser if requested
                    if opts.open {
                        open_browser(&calendar_url, global.is_silent()).await;
                    }
                }
                Err(e) => {
                    aprintln!("{} Seeding failed: {}", p_r("✗"), e);
                    // Continue running server even if seeding fails
                }
            }
        } else if opts.open {
            // --open without --seed: warn the user
            if !global.is_silent() {
                aprintln!(
                    "{} --open requires --seed to create a calendar to open",
                    p_y("⚠")
                );
            }
        }

        let status = server.wait().await?;
        if !opts.keep_containers {
            if let Some(runtime) = runtime {
                cleanup_containers(runtime, &started_containers, &global).await;
            }
        }
        if !status.success() {
            return Err(DevError::Io(std::io::Error::other("Server process failed")));
        }
        return Ok(());
    }

    // =========================================================================
    // Stage 5: Hot-reload mode with seeding
    // =========================================================================
    if !global.is_silent() {
        aprintln!(
            "{} Hot-reload enabled, watching crates/frontend/src/",
            p_y("👀")
        );
    }

    // Wait for server to be ready before starting watcher
    let base_url = format!("http://localhost:{}", opts.port);
    wait_for_server_ready(opts.port, global.is_silent()).await;

    // Seed if requested
    if opts.seed {
        if !global.is_silent() {
            aprintln!("{} Seeding database...", p_b("🌱"));
        }

        match seed::seed_via_http(&base_url, global.is_silent()).await {
            Ok(calendar_id) => {
                let calendar_url = format!("{}/calendar/{}", base_url, calendar_id);
                if !global.is_silent() {
                    aprintln!("{} Seeding complete!", p_g("✓"));
                    aprintln!("   Calendar URL: {}", p_b(&calendar_url));
                }

                // Open browser if requested
                if opts.open {
                    open_browser(&calendar_url, global.is_silent()).await;
                }
            }
            Err(e) => {
                aprintln!("{} Seeding failed: {}", p_r("✗"), e);
                // Continue running server even if seeding fails
            }
        }
    } else if opts.open {
        // --open without --seed: warn the user
        if !global.is_silent() {
            aprintln!(
                "{} --open requires --seed to create a calendar to open",
                p_y("⚠")
            );
        }
    }

    // Track current asset filenames for change detection
    let mut current_css = get_current_css_filename();
    let mut current_server_js = get_current_server_js_filename();
    let mut current_client_js = get_current_client_js_filename();

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
                if let Some(assets) = handle_file_change(
                    &opts,
                    &global,
                    current_css.as_deref(),
                    current_server_js.as_deref(),
                    current_client_js.as_deref(),
                ).await {
                    current_css = Some(assets.css);
                    current_server_js = Some(assets.server_js);
                    current_client_js = Some(assets.client_js);
                }
            }
        }
    }

    // Cleanup containers on exit
    if !opts.keep_containers {
        if let Some(runtime) = runtime {
            cleanup_containers(runtime, &started_containers, &global).await;
        }
    } else if !started_containers.is_empty() && !global.is_silent() {
        aprintln!("{} Containers left running (--keep-containers)", p_y("⚠"));
    }

    Ok(())
}

/// Clean up started containers.
async fn cleanup_containers(
    runtime: ContainerRuntime,
    containers: &[&'static ContainerSpec],
    global: &crate::Global,
) {
    let verbose = global.is_verbose();
    for spec in containers {
        if !global.is_silent() {
            aprintln!("{} Stopping {}...", p_b("🐳"), spec.name);
        }
        let _ = containers::stop_container(runtime, spec.name, verbose).await;
    }
}

/// Deploy DynamoDB table using cargo xtask dynamodb deploy.
///
/// The `port` parameter specifies the actual DynamoDB Local port (discovered at runtime).
async fn deploy_dynamodb_table(port: u16) -> std::result::Result<(), String> {
    let endpoint_url = format!("http://localhost:{}", port);

    let status = Command::new("cargo")
        .args([
            "xtask",
            "dynamodb",
            "deploy",
            "--force",
            "--table-name",
            "calendsync",
        ])
        .env("AWS_ENDPOINT_URL", &endpoint_url)
        .env("AWS_REGION", "us-east-1")
        .env("AWS_ACCESS_KEY_ID", "test")
        .env("AWS_SECRET_ACCESS_KEY", "test")
        .status()
        .await
        .map_err(|e| e.to_string())?;

    if !status.success() {
        return Err("cargo xtask dynamodb deploy failed".to_string());
    }

    Ok(())
}

/// Start the server process with specific features and environment variables.
///
/// The `container_ports` parameter provides the actual ports discovered after starting containers.
fn start_server_with_features(
    opts: &ServerOptions,
    features: &str,
    container_ports: &ContainerPorts,
) -> Result<Child> {
    let mut args = vec![
        "run",
        "-p",
        "calendsync",
        "--no-default-features",
        "--features",
        features,
    ];
    if opts.release {
        args.push("--release");
    }

    let mut cmd = Command::new("cargo");
    cmd.args(&args);

    // Set all environment variables from the configuration
    let env_vars =
        containers::environment_variables(opts.storage, opts.cache, opts.port, container_ports);
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

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

/// Opens the browser to the specified URL (macOS only).
///
/// Waits briefly to ensure the page is ready before opening.
async fn open_browser(url: &str, silent: bool) {
    // Brief delay to ensure the page is ready
    tokio::time::sleep(Duration::from_millis(500)).await;

    if !silent {
        aprintln!("{} Opening browser...", p_b("🌐"));
    }

    // macOS-specific: use `open` command
    match Command::new("open").arg(url).spawn() {
        Ok(_) => {
            if !silent {
                aprintln!("{} Browser opened: {}", p_g("✓"), url);
            }
        }
        Err(e) => {
            if !silent {
                aprintln!("{} Failed to open browser: {}", p_y("⚠"), e);
                aprintln!("   Open manually: {}", url);
            }
        }
    }
}

/// Tracked asset filenames for change detection.
struct AssetFilenames {
    css: String,
    server_js: String,
    client_js: String,
}

/// Handle a file change event.
/// Returns the new asset filenames if the reload was successful.
async fn handle_file_change(
    opts: &ServerOptions,
    global: &crate::Global,
    prev_css: Option<&str>,
    prev_server_js: Option<&str>,
    prev_client_js: Option<&str>,
) -> Option<AssetFilenames> {
    if !global.is_silent() {
        aprintln!("{} Change detected, rebuilding...", p_y("🔄"));
    }

    // Run bun build
    match run_frontend_build().await {
        Ok(()) => {
            // Build succeeded - trigger reload
            match trigger_reload(opts.port, prev_css, prev_server_js, prev_client_js).await {
                Ok(response) => {
                    if !global.is_silent() {
                        match response.change_type.as_str() {
                            "none" => {
                                aprintln!("{} No changes detected", p_y("○"));
                            }
                            "css_only" => {
                                aprintln!("{} CSS hot-swapped!", p_g("✓"));
                            }
                            "client_only" => {
                                aprintln!("{} Client JS reloaded!", p_g("✓"));
                            }
                            _ => {
                                // "full"
                                aprintln!("{} SSR pool swapped, reloaded!", p_g("✓"));
                            }
                        }
                    }
                    return Some(AssetFilenames {
                        css: response.css,
                        server_js: response.server_js,
                        client_js: response.client_js,
                    });
                }
                Err(e) => {
                    if !global.is_silent() {
                        aprintln!("{} Reload failed: {}", p_r("✗"), e);
                    }
                }
            }
        }
        Err(DevError::BuildFailedWithOutput(stderr)) => {
            // Build failed with captured stderr
            if !global.is_silent() {
                aprintln!("{} Build failed:\n{}", p_r("✗"), stderr);
            }

            // Send error to browser
            if let Err(e) = send_build_error(opts.port, &stderr).await {
                if !global.is_silent() {
                    aprintln!("{} Could not send error to browser: {}", p_y("⚠"), e);
                }
            }
        }
        Err(e) => {
            if !global.is_silent() {
                aprintln!("{} Build error: {}", p_r("✗"), e);
            }
        }
    }

    None
}

/// Run the frontend build.
async fn run_frontend_build() -> Result<()> {
    let output = Command::new("bun")
        .args(["run", "build:dev"])
        .current_dir("crates/frontend")
        .output()
        .await?;

    if !output.status.success() {
        // Combine stderr and stdout for complete error output
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = if stderr.is_empty() {
            stdout.to_string()
        } else if stdout.is_empty() {
            stderr.to_string()
        } else {
            format!("{}\n{}", stderr, stdout)
        };

        return Err(DevError::BuildFailedWithOutput(combined));
    }

    Ok(())
}

/// Response from the reload endpoint.
#[derive(serde::Deserialize)]
struct ReloadResponse {
    #[allow(dead_code)]
    success: bool,
    #[allow(dead_code)]
    bundle: String,
    css: String,
    server_js: String,
    client_js: String,
    change_type: String,
}

/// Get a filename from the manifest by key.
fn get_manifest_entry(key: &str) -> Option<String> {
    let manifest_path = Path::new("crates/frontend/manifest.json");
    let content = std::fs::read_to_string(manifest_path).ok()?;
    let manifest: serde_json::Value = serde_json::from_str(&content).ok()?;
    manifest
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Get current CSS filename from manifest.
fn get_current_css_filename() -> Option<String> {
    get_manifest_entry("calendsync.css")
}

/// Get current server JS filename from manifest.
fn get_current_server_js_filename() -> Option<String> {
    get_manifest_entry("calendsync.js")
}

/// Get current client JS filename from manifest.
fn get_current_client_js_filename() -> Option<String> {
    get_manifest_entry("calendsync-client.js")
}

/// Trigger SSR pool reload via HTTP.
async fn trigger_reload(
    port: u16,
    prev_css: Option<&str>,
    prev_server_js: Option<&str>,
    prev_client_js: Option<&str>,
) -> Result<ReloadResponse> {
    let client = reqwest::Client::new();
    let url = format!("http://localhost:{}/_dev/reload", port);

    let body = serde_json::json!({
        "prev_css": prev_css,
        "prev_server_js": prev_server_js,
        "prev_client_js": prev_client_js
    });

    let response = client.post(&url).json(&body).send().await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(DevError::ReloadFailed(body));
    }

    let reload_response: ReloadResponse = response.json().await?;
    Ok(reload_response)
}

/// Send build error to browser via HTTP.
async fn send_build_error(port: u16, error: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("http://localhost:{}/_dev/error", port);

    let response = client
        .post(&url)
        .json(&serde_json::json!({ "error": error }))
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(DevError::ReloadFailed(body));
    }

    Ok(())
}
