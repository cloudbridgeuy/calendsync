//! Container management for development dependencies.
//!
//! This module manages Docker/Podman containers for development services like
//! DynamoDB Local and Redis. It follows the Functional Core - Imperative Shell
//! pattern:
//!
//! - **Pure functions** build command arguments and determine which containers
//!   are needed based on storage/cache configuration. These have no side effects.
//! - **I/O functions** execute container commands, check health, and manage
//!   container lifecycle.

use std::time::Duration;

use tokio::process::Command;

use super::error::{DevError, Result};

// ============================================================================
// Types
// ============================================================================

/// Storage backend selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum Storage {
    #[default]
    Inmemory,
    Sqlite,
    Dynamodb,
}

/// Cache backend selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum Cache {
    #[default]
    Memory,
    Redis,
}

/// Container runtime (Docker or Podman).
#[derive(Debug, Clone, Copy, Default)]
pub enum ContainerRuntime {
    #[default]
    Docker,
    Podman,
}

/// Specification for a container.
#[derive(Debug, Clone)]
pub struct ContainerSpec {
    pub name: &'static str,
    pub image: &'static str,
    pub port: u16,
    pub volume_name: &'static str,
    pub volume_path: &'static str,
    pub command: Option<&'static str>,
    pub health_check: HealthCheck,
}

/// Health check strategy for a container.
#[derive(Debug, Clone)]
pub enum HealthCheck {
    /// HTTP check - DynamoDB returns 400 for invalid requests when healthy.
    Http { port: u16, expected_status: u16 },
    /// Redis PING check.
    Redis,
}

// ============================================================================
// Container Specifications (Constants)
// ============================================================================

/// DynamoDB Local container specification.
pub const DYNAMODB_SPEC: ContainerSpec = ContainerSpec {
    name: "calendsync-dynamodb",
    image: "amazon/dynamodb-local:latest",
    port: 8000,
    volume_name: "calendsync-dynamodb-data",
    volume_path: "/data",
    command: Some("-jar DynamoDBLocal.jar -sharedDb -dbPath /data"),
    health_check: HealthCheck::Http {
        port: 8000,
        expected_status: 400,
    },
};

/// Redis container specification.
pub const REDIS_SPEC: ContainerSpec = ContainerSpec {
    name: "calendsync-redis",
    image: "redis:7-alpine",
    port: 6379,
    volume_name: "calendsync-redis-data",
    volume_path: "/data",
    command: Some("redis-server --appendonly yes"),
    health_check: HealthCheck::Redis,
};

// ============================================================================
// Pure Functions (Functional Core)
// ============================================================================

/// Builds arguments for `docker run` / `podman run`.
///
/// Returns a vector of command arguments including:
/// - `--name {name}`
/// - `-d` (detached mode)
/// - `-p {port}:{port}` (port mapping)
/// - `-v {volume_name}:{volume_path}` (volume mount)
/// - `{image}`
/// - Command args if present (split by whitespace)
pub fn container_run_args(spec: &ContainerSpec) -> Vec<String> {
    let mut args = vec![
        "run".to_string(),
        "--name".to_string(),
        spec.name.to_string(),
        "-d".to_string(),
        "-p".to_string(),
        format!("{}:{}", spec.port, spec.port),
        "-v".to_string(),
        format!("{}:{}", spec.volume_name, spec.volume_path),
        spec.image.to_string(),
    ];

    if let Some(cmd) = spec.command {
        args.extend(cmd.split_whitespace().map(String::from));
    }

    args
}

/// Returns which containers are needed based on storage and cache configuration.
///
/// - DynamoDB storage → include `DYNAMODB_SPEC`
/// - Redis cache → include `REDIS_SPEC`
/// - In-memory/SQLite storage and Memory cache → empty vec
pub fn required_containers(storage: Storage, cache: Cache) -> Vec<&'static ContainerSpec> {
    let mut containers = Vec::new();

    if storage == Storage::Dynamodb {
        containers.push(&DYNAMODB_SPEC);
    }

    if cache == Cache::Redis {
        containers.push(&REDIS_SPEC);
    }

    containers
}

/// Returns the cargo feature string for the given storage and cache configuration.
///
/// Format: `"{storage},{cache}"` where storage is "inmemory", "sqlite", or "dynamodb"
/// and cache is "memory" or "redis".
pub fn cargo_features(storage: Storage, cache: Cache) -> String {
    let storage_str = match storage {
        Storage::Inmemory => "inmemory",
        Storage::Sqlite => "sqlite",
        Storage::Dynamodb => "dynamodb",
    };

    let cache_str = match cache {
        Cache::Memory => "memory",
        Cache::Redis => "redis",
    };

    format!("{},{}", storage_str, cache_str)
}

/// Returns environment variables for the given configuration.
///
/// Always includes:
/// - `PORT` - server port
/// - `DEV_MODE` - set to "1"
///
/// Storage-specific:
/// - DynamoDB: AWS endpoint and credentials for local development
/// - SQLite: path to database file
///
/// Cache-specific:
/// - Redis: connection URL
pub fn environment_variables(
    storage: Storage,
    cache: Cache,
    port: u16,
) -> Vec<(&'static str, String)> {
    let mut vars = vec![("PORT", port.to_string()), ("DEV_MODE", "1".to_string())];

    match storage {
        Storage::Dynamodb => {
            vars.push(("AWS_ENDPOINT_URL", "http://localhost:8000".to_string()));
            vars.push(("AWS_REGION", "us-east-1".to_string()));
            vars.push(("AWS_ACCESS_KEY_ID", "test".to_string()));
            vars.push(("AWS_SECRET_ACCESS_KEY", "test".to_string()));
        }
        Storage::Sqlite => {
            vars.push(("SQLITE_PATH", ".local/data/calendsync.db".to_string()));
        }
        Storage::Inmemory => {}
    }

    if cache == Cache::Redis {
        vars.push(("REDIS_URL", "redis://localhost:6379".to_string()));
    }

    vars
}

// ============================================================================
// I/O Functions (Imperative Shell)
// ============================================================================

/// Returns the command name for the container runtime.
pub fn runtime_command(runtime: ContainerRuntime) -> &'static str {
    match runtime {
        ContainerRuntime::Docker => "docker",
        ContainerRuntime::Podman => "podman",
    }
}

/// Detects which container runtime is available.
///
/// If `prefer_podman` is true, checks Podman first, then Docker.
/// Otherwise checks Docker first, then Podman.
///
/// Returns an error if neither runtime is available.
pub async fn detect_runtime(prefer_podman: bool) -> Result<ContainerRuntime> {
    let check_order = if prefer_podman {
        [
            (ContainerRuntime::Podman, "podman"),
            (ContainerRuntime::Docker, "docker"),
        ]
    } else {
        [
            (ContainerRuntime::Docker, "docker"),
            (ContainerRuntime::Podman, "podman"),
        ]
    };

    for (runtime, cmd) in check_order {
        let output = Command::new(cmd).arg("--version").output().await;

        if let Ok(output) = output {
            if output.status.success() {
                return Ok(runtime);
            }
        }
    }

    Err(DevError::ContainerRuntimeNotFound(
        "Neither docker nor podman found in PATH".to_string(),
    ))
}

/// Stops and removes a container.
///
/// Errors are ignored since the container might not exist.
pub async fn stop_container(runtime: ContainerRuntime, name: &str) -> Result<()> {
    let cmd = runtime_command(runtime);

    // Stop container (ignore errors - container might not be running)
    let _ = Command::new(cmd).args(["stop", name]).output().await;

    // Remove container (ignore errors - container might not exist)
    let _ = Command::new(cmd).args(["rm", name]).output().await;

    Ok(())
}

/// Removes a volume.
pub async fn remove_volume(runtime: ContainerRuntime, name: &str) -> Result<()> {
    let cmd = runtime_command(runtime);

    let output = Command::new(cmd)
        .args(["volume", "rm", name])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DevError::Io(std::io::Error::other(format!(
            "Failed to remove volume '{}': {}",
            name, stderr
        ))));
    }

    Ok(())
}

/// Starts a container with the given specification.
///
/// First stops and removes any existing container with the same name,
/// then starts a new container.
pub async fn start_container(runtime: ContainerRuntime, spec: &ContainerSpec) -> Result<()> {
    let cmd = runtime_command(runtime);

    // Clean up any existing container
    stop_container(runtime, spec.name).await?;

    // Build run arguments
    let args = container_run_args(spec);
    let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();

    // Start container
    let output = Command::new(cmd).args(&args_ref).output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DevError::ContainerStartFailed(format!(
            "Failed to start container '{}': {}",
            spec.name, stderr
        )));
    }

    Ok(())
}

/// Waits for a container to become healthy.
///
/// Polls the container's health check until it passes or the timeout is exceeded.
pub async fn wait_for_health(
    runtime: ContainerRuntime,
    spec: &ContainerSpec,
    timeout: Duration,
) -> Result<()> {
    let start = std::time::Instant::now();
    let poll_interval = Duration::from_millis(500);

    while start.elapsed() < timeout {
        let healthy = match &spec.health_check {
            HealthCheck::Http {
                port,
                expected_status,
            } => check_http_health(*port, *expected_status).await,
            HealthCheck::Redis => check_redis_health(runtime, spec.name).await,
        };

        if healthy {
            return Ok(());
        }

        tokio::time::sleep(poll_interval).await;
    }

    Err(DevError::ContainerNotHealthy {
        name: spec.name.to_string(),
        timeout_secs: timeout.as_secs(),
    })
}

/// Checks HTTP health by making a request and comparing the status code.
async fn check_http_health(port: u16, expected_status: u16) -> bool {
    let url = format!("http://localhost:{}/", port);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build();

    let Ok(client) = client else {
        return false;
    };

    match client.get(&url).send().await {
        Ok(response) => response.status().as_u16() == expected_status,
        Err(_) => false,
    }
}

/// Checks Redis health by running `redis-cli ping` inside the container.
async fn check_redis_health(runtime: ContainerRuntime, name: &str) -> bool {
    let cmd = runtime_command(runtime);

    let output = Command::new(cmd)
        .args(["exec", name, "redis-cli", "ping"])
        .output()
        .await;

    match output {
        Ok(output) => {
            output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "PONG"
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_run_args_with_command() {
        let args = container_run_args(&DYNAMODB_SPEC);

        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"--name".to_string()));
        assert!(args.contains(&"calendsync-dynamodb".to_string()));
        assert!(args.contains(&"-d".to_string()));
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"8000:8000".to_string()));
        assert!(args.contains(&"-v".to_string()));
        assert!(args.contains(&"calendsync-dynamodb-data:/data".to_string()));
        assert!(args.contains(&"amazon/dynamodb-local:latest".to_string()));
        // Command args should be split
        assert!(args.contains(&"-jar".to_string()));
        assert!(args.contains(&"DynamoDBLocal.jar".to_string()));
    }

    #[test]
    fn test_container_run_args_redis() {
        let args = container_run_args(&REDIS_SPEC);

        assert!(args.contains(&"calendsync-redis".to_string()));
        assert!(args.contains(&"6379:6379".to_string()));
        assert!(args.contains(&"redis:7-alpine".to_string()));
        assert!(args.contains(&"redis-server".to_string()));
        assert!(args.contains(&"--appendonly".to_string()));
        assert!(args.contains(&"yes".to_string()));
    }

    #[test]
    fn test_required_containers_inmemory_memory() {
        let containers = required_containers(Storage::Inmemory, Cache::Memory);
        assert!(containers.is_empty());
    }

    #[test]
    fn test_required_containers_sqlite_memory() {
        let containers = required_containers(Storage::Sqlite, Cache::Memory);
        assert!(containers.is_empty());
    }

    #[test]
    fn test_required_containers_dynamodb_only() {
        let containers = required_containers(Storage::Dynamodb, Cache::Memory);
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].name, "calendsync-dynamodb");
    }

    #[test]
    fn test_required_containers_redis_only() {
        let containers = required_containers(Storage::Inmemory, Cache::Redis);
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].name, "calendsync-redis");
    }

    #[test]
    fn test_required_containers_dynamodb_and_redis() {
        let containers = required_containers(Storage::Dynamodb, Cache::Redis);
        assert_eq!(containers.len(), 2);
        assert_eq!(containers[0].name, "calendsync-dynamodb");
        assert_eq!(containers[1].name, "calendsync-redis");
    }

    #[test]
    fn test_cargo_features() {
        assert_eq!(
            cargo_features(Storage::Inmemory, Cache::Memory),
            "inmemory,memory"
        );
        assert_eq!(
            cargo_features(Storage::Sqlite, Cache::Memory),
            "sqlite,memory"
        );
        assert_eq!(
            cargo_features(Storage::Dynamodb, Cache::Memory),
            "dynamodb,memory"
        );
        assert_eq!(
            cargo_features(Storage::Inmemory, Cache::Redis),
            "inmemory,redis"
        );
        assert_eq!(
            cargo_features(Storage::Sqlite, Cache::Redis),
            "sqlite,redis"
        );
        assert_eq!(
            cargo_features(Storage::Dynamodb, Cache::Redis),
            "dynamodb,redis"
        );
    }

    #[test]
    fn test_environment_variables_inmemory_memory() {
        let vars = environment_variables(Storage::Inmemory, Cache::Memory, 3000);

        assert!(vars.contains(&("PORT", "3000".to_string())));
        assert!(vars.contains(&("DEV_MODE", "1".to_string())));
        assert_eq!(vars.len(), 2);
    }

    #[test]
    fn test_environment_variables_dynamodb() {
        let vars = environment_variables(Storage::Dynamodb, Cache::Memory, 8080);

        assert!(vars.contains(&("PORT", "8080".to_string())));
        assert!(vars.contains(&("DEV_MODE", "1".to_string())));
        assert!(vars.contains(&("AWS_ENDPOINT_URL", "http://localhost:8000".to_string())));
        assert!(vars.contains(&("AWS_REGION", "us-east-1".to_string())));
        assert!(vars.contains(&("AWS_ACCESS_KEY_ID", "test".to_string())));
        assert!(vars.contains(&("AWS_SECRET_ACCESS_KEY", "test".to_string())));
    }

    #[test]
    fn test_environment_variables_sqlite() {
        let vars = environment_variables(Storage::Sqlite, Cache::Memory, 3000);

        assert!(vars.contains(&("SQLITE_PATH", ".local/data/calendsync.db".to_string())));
    }

    #[test]
    fn test_environment_variables_redis() {
        let vars = environment_variables(Storage::Inmemory, Cache::Redis, 3000);

        assert!(vars.contains(&("REDIS_URL", "redis://localhost:6379".to_string())));
    }

    #[test]
    fn test_environment_variables_dynamodb_redis() {
        let vars = environment_variables(Storage::Dynamodb, Cache::Redis, 3000);

        // Should have all vars
        assert!(vars.contains(&("PORT", "3000".to_string())));
        assert!(vars.contains(&("DEV_MODE", "1".to_string())));
        assert!(vars.contains(&("AWS_ENDPOINT_URL", "http://localhost:8000".to_string())));
        assert!(vars.contains(&("REDIS_URL", "redis://localhost:6379".to_string())));
    }

    #[test]
    fn test_runtime_command() {
        assert_eq!(runtime_command(ContainerRuntime::Docker), "docker");
        assert_eq!(runtime_command(ContainerRuntime::Podman), "podman");
    }
}
