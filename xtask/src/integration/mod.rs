//! Integration test infrastructure.
//!
//! This module provides commands for running integration tests against
//! real infrastructure (DynamoDB Local, Redis via Docker).
//!
//! # Usage
//!
//! ```bash
//! # Run all integration tests (SQLite + DynamoDB with memory cache)
//! cargo xtask integration
//!
//! # Run only SQLite integration tests
//! cargo xtask integration --sqlite
//!
//! # Run only DynamoDB integration tests
//! cargo xtask integration --dynamodb
//!
//! # Run with Redis cache (requires Docker)
//! cargo xtask integration --redis
//!
//! # Run SQLite with Redis cache
//! cargo xtask integration --sqlite --redis
//!
//! # Skip container management (assumes services are already running)
//! cargo xtask integration --dynamodb --redis --no-docker
//! ```

pub mod error;

pub use error::{IntegrationError, Result};

use std::time::Duration;

use crate::dev::containers::{
    detect_runtime, get_container_port, start_container, stop_container, wait_for_health,
    ContainerRuntime, DYNAMODB_SPEC, REDIS_SPEC,
};
use crate::prelude::*;

/// Integration test command.
#[derive(Debug, clap::Parser)]
#[command(long_about = "Run integration tests against real infrastructure.

This command manages Docker containers for local testing and runs
the integration test suite against real storage and cache backends.

By default, it runs tests for both SQLite and DynamoDB backends with
in-memory cache. The command automatically starts required Docker
containers (DynamoDB Local, Redis) and stops them afterward.

Environment variables:
  AWS_ENDPOINT_URL    - Override DynamoDB endpoint (default: http://localhost:8000)
  AWS_REGION          - AWS region for DynamoDB (default: us-east-1)
  REDIS_URL           - Override Redis URL (default: redis://localhost:6379)")]
pub struct IntegrationCommand {
    // Storage backend flags
    /// Run only SQLite integration tests.
    #[arg(long, conflicts_with = "dynamodb")]
    pub sqlite: bool,

    /// Run only DynamoDB integration tests.
    #[arg(long, conflicts_with = "sqlite")]
    pub dynamodb: bool,

    // Cache backend flags
    /// Run tests with in-memory cache (default if no cache flag specified).
    #[arg(long, conflicts_with = "redis")]
    pub memory: bool,

    /// Run tests with Redis cache (requires Docker).
    #[arg(long, conflicts_with = "memory")]
    pub redis: bool,

    // Container management flags
    /// Skip Docker container management (assume services are already running).
    #[arg(long)]
    pub no_docker: bool,

    /// Keep containers running after tests complete.
    #[arg(long)]
    pub keep_containers: bool,

    /// Timeout in seconds for container health checks.
    #[arg(long, default_value = "30")]
    pub health_timeout: u64,
}

/// Main entry point for integration command.
pub async fn run(command: IntegrationCommand, global: crate::Global) -> Result<()> {
    // Determine which storage backends to test
    // If neither flag is set, run both; otherwise run only the specified one
    let run_sqlite = command.sqlite || !command.dynamodb;
    let run_dynamodb = command.dynamodb || !command.sqlite;

    // Determine which cache backend to use
    // Default to memory if neither flag is set
    let use_redis = command.redis;
    let cache_backend = if use_redis { "redis" } else { "memory" };

    if !global.is_silent() {
        aprintln!("{}", p_b("Integration Tests"));
        aprintln!();
        aprintln!(
            "{} Storage: {}, Cache: {}",
            p_b("Config:"),
            if run_sqlite && run_dynamodb {
                "SQLite + DynamoDB".to_string()
            } else if run_sqlite {
                "SQLite".to_string()
            } else {
                "DynamoDB".to_string()
            },
            if use_redis { "Redis" } else { "Memory" }
        );
        aprintln!();
    }

    // Detect container runtime if we need containers
    let needs_containers = !command.no_docker && (use_redis || run_dynamodb);
    let runtime = if needs_containers {
        Some(detect_runtime(false).await?)
    } else {
        None
    };

    // Track which containers we started and their ports
    let mut redis_port: Option<u16> = None;
    let mut dynamodb_port: Option<u16> = None;
    let mut redis_started = false;
    let mut dynamodb_started = false;

    // Start Redis container if needed
    if use_redis && !command.no_docker {
        let rt = runtime.expect("runtime should be detected when containers are needed");
        if let Some(port) = start_redis_container(command.health_timeout, &global, rt).await? {
            redis_port = Some(port);
            redis_started = true;
        }
    } else if use_redis && command.no_docker {
        // Use default port when --no-docker
        redis_port = Some(6379);
        if !global.is_silent() {
            aprintln!(
                "{} {}",
                p_y("⚠️"),
                "Skipping Redis container management (--no-docker)"
            );
        }
    }

    let mut all_passed = true;

    // Run SQLite tests
    if run_sqlite {
        if !global.is_silent() {
            aprintln!(
                "{} {}",
                p_b("🔧"),
                p_b("Running SQLite integration tests...")
            );
        }

        let redis_url = redis_port.map(|p| format!("redis://localhost:{}", p));
        let env_vars: Vec<(&str, String)> = if let Some(ref url) = redis_url {
            vec![("REDIS_URL", url.clone())]
        } else {
            vec![]
        };

        if !run_tests_with_features("sqlite", cache_backend, env_vars, &global).await? {
            all_passed = false;
        }
    }

    // Run DynamoDB tests (requires Docker)
    if run_dynamodb {
        if !global.is_silent() {
            aprintln!(
                "{} {}",
                p_b("🔧"),
                p_b("Running DynamoDB integration tests...")
            );
        }

        // Start DynamoDB container if needed
        if !command.no_docker {
            let rt = runtime.expect("runtime should be detected when containers are needed");
            if let Some(port) =
                start_dynamodb_container(command.health_timeout, &global, rt).await?
            {
                dynamodb_port = Some(port);
                dynamodb_started = true;
            }
        } else {
            // Use default port when --no-docker
            dynamodb_port = Some(8000);
            if !global.is_silent() {
                aprintln!(
                    "{} {}",
                    p_y("⚠️"),
                    "Skipping DynamoDB container management (--no-docker)"
                );
            }
        }

        let ddb_port = dynamodb_port.unwrap_or(8000);

        // Set up the test table
        if !global.is_silent() {
            aprintln!("{} {}", p_b("📦"), "Setting up test table...");
        }
        setup_test_table(ddb_port, &global).await?;

        // Build environment variables with actual port
        let endpoint_url = format!("http://localhost:{}", ddb_port);
        let mut env_vars = vec![
            ("AWS_ENDPOINT_URL", endpoint_url),
            ("AWS_REGION", "us-east-1".to_string()),
            ("AWS_ACCESS_KEY_ID", "test".to_string()),
            ("AWS_SECRET_ACCESS_KEY", "test".to_string()),
        ];
        if let Some(port) = redis_port {
            env_vars.push(("REDIS_URL", format!("redis://localhost:{}", port)));
        }

        if !run_tests_with_features("dynamodb", cache_backend, env_vars, &global).await? {
            all_passed = false;
        }
    }

    // Cleanup containers
    if !command.keep_containers {
        if let Some(rt) = runtime {
            if redis_started {
                stop_redis_container(&global, rt).await?;
            }
            if dynamodb_started {
                stop_dynamodb_container(&global, rt).await?;
            }
        }
    } else if (redis_started || dynamodb_started) && !global.is_silent() {
        aprintln!(
            "{} {}",
            p_y("⚠️"),
            "Containers left running (--keep-containers)"
        );
    }

    aprintln!();
    if all_passed {
        aprintln!("{} {}", p_g("✅"), p_g("All integration tests passed!"));
        Ok(())
    } else {
        aprintln!("{} {}", p_r("❌"), p_r("Some integration tests failed"));
        Err(IntegrationError::TestFailed(
            "One or more test suites failed".to_string(),
        ))
    }
}

/// Run tests with specific storage and cache features.
async fn run_tests_with_features(
    storage: &str,
    cache: &str,
    env_vars: Vec<(&str, String)>,
    global: &crate::Global,
) -> Result<bool> {
    // Build feature string: e.g., "sqlite,memory" or "dynamodb,redis"
    let features = format!("{},{}", storage, cache);

    if !global.is_silent() {
        aprintln!("{} Running with features: {}", p_b("  →"), p_y(&features));
    }

    // Build the command
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.args([
        "test",
        "-p",
        "calendsync",
        "--features",
        &features,
        "--no-default-features",
    ]);

    // Add environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    let status = cmd.status().await?;

    if status.success() {
        if !global.is_silent() {
            aprintln!(
                "{} {} tests passed",
                p_g("✅"),
                format!("{}/{}", storage, cache)
            );
        }
        Ok(true)
    } else {
        aprintln!(
            "{} {} tests failed",
            p_r("❌"),
            format!("{}/{}", storage, cache)
        );
        Ok(false)
    }
}

/// Start the DynamoDB Local container.
///
/// Returns `Some(port)` with the actual host port if we started a new container,
/// or the port of an existing container if one was already running.
async fn start_dynamodb_container(
    timeout_secs: u64,
    global: &crate::Global,
    runtime: ContainerRuntime,
) -> Result<Option<u16>> {
    // Check if container is already running
    let cmd = crate::dev::containers::runtime_command(runtime);
    let ps_output = tokio::process::Command::new(cmd)
        .args(["ps", "-q", "-f", &format!("name={}", DYNAMODB_SPEC.name)])
        .output()
        .await?;

    if !String::from_utf8_lossy(&ps_output.stdout).trim().is_empty() {
        if !global.is_silent() {
            aprintln!(
                "{} {}",
                p_y("⚠️"),
                "DynamoDB Local container already running"
            );
        }
        // Query the port of the existing container
        let port =
            get_container_port(runtime, DYNAMODB_SPEC.name, DYNAMODB_SPEC.port, false).await?;
        return Ok(Some(port));
    }

    if !global.is_silent() {
        aprintln!("{} {}", p_b("🐳"), "Starting DynamoDB Local container...");
    }

    // Start container using the container module
    let verbose = global.is_verbose();
    start_container(runtime, &DYNAMODB_SPEC, verbose).await?;

    // Query the actual host port
    let port = get_container_port(runtime, DYNAMODB_SPEC.name, DYNAMODB_SPEC.port, verbose).await?;

    if !global.is_silent() {
        aprintln!(
            "   Container port: {} -> localhost:{}",
            DYNAMODB_SPEC.port,
            port
        );
    }

    // Wait for container to be healthy
    if !global.is_silent() {
        aprintln!(
            "{} {}",
            p_b("⏳"),
            format!("Waiting for container health (max {}s)...", timeout_secs)
        );
    }

    wait_for_health(
        runtime,
        &DYNAMODB_SPEC,
        port,
        Duration::from_secs(timeout_secs),
    )
    .await?;

    if !global.is_silent() {
        aprintln!("{} {}", p_g("✅"), "DynamoDB Local is ready");
    }

    Ok(Some(port))
}

/// Stop the DynamoDB Local container.
async fn stop_dynamodb_container(global: &crate::Global, runtime: ContainerRuntime) -> Result<()> {
    if !global.is_silent() {
        aprintln!("{} {}", p_b("🐳"), "Stopping DynamoDB Local container...");
    }

    stop_container(runtime, DYNAMODB_SPEC.name, global.is_verbose()).await?;

    if !global.is_silent() {
        aprintln!("{} {}", p_g("✅"), "DynamoDB container stopped");
    }

    Ok(())
}

/// Start the Redis container.
///
/// Returns `Some(port)` with the actual host port if we started a new container,
/// or `None` if the container was already running.
async fn start_redis_container(
    timeout_secs: u64,
    global: &crate::Global,
    runtime: ContainerRuntime,
) -> Result<Option<u16>> {
    // Check if container is already running
    let cmd = crate::dev::containers::runtime_command(runtime);
    let ps_output = tokio::process::Command::new(cmd)
        .args(["ps", "-q", "-f", &format!("name={}", REDIS_SPEC.name)])
        .output()
        .await?;

    if !String::from_utf8_lossy(&ps_output.stdout).trim().is_empty() {
        if !global.is_silent() {
            aprintln!("{} {}", p_y("⚠️"), "Redis container already running");
        }
        // Query the port of the existing container
        let port = get_container_port(runtime, REDIS_SPEC.name, REDIS_SPEC.port, false).await?;
        return Ok(Some(port));
    }

    if !global.is_silent() {
        aprintln!("{} {}", p_b("🐳"), "Starting Redis container...");
    }

    // Start container using the container module
    let verbose = global.is_verbose();
    start_container(runtime, &REDIS_SPEC, verbose).await?;

    // Query the actual host port
    let port = get_container_port(runtime, REDIS_SPEC.name, REDIS_SPEC.port, verbose).await?;

    if !global.is_silent() {
        aprintln!(
            "   Container port: {} -> localhost:{}",
            REDIS_SPEC.port,
            port
        );
    }

    // Wait for container to be healthy
    if !global.is_silent() {
        aprintln!(
            "{} {}",
            p_b("⏳"),
            format!("Waiting for Redis health (max {}s)...", timeout_secs)
        );
    }

    wait_for_health(
        runtime,
        &REDIS_SPEC,
        port,
        Duration::from_secs(timeout_secs),
    )
    .await?;

    if !global.is_silent() {
        aprintln!("{} {}", p_g("✅"), "Redis is ready");
    }

    Ok(Some(port))
}

/// Stop the Redis container.
async fn stop_redis_container(global: &crate::Global, runtime: ContainerRuntime) -> Result<()> {
    if !global.is_silent() {
        aprintln!("{} {}", p_b("🐳"), "Stopping Redis container...");
    }

    stop_container(runtime, REDIS_SPEC.name, global.is_verbose()).await?;

    if !global.is_silent() {
        aprintln!("{} {}", p_g("✅"), "Redis container stopped");
    }

    Ok(())
}

/// Set up the test table in DynamoDB Local.
///
/// The `port` parameter specifies the actual DynamoDB Local port (discovered at runtime).
async fn setup_test_table(port: u16, global: &crate::Global) -> Result<()> {
    let endpoint_url = format!("http://localhost:{}", port);

    // Use cargo xtask dynamodb deploy with force flag
    let status = tokio::process::Command::new("cargo")
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
        .await?;

    if !status.success() {
        return Err(IntegrationError::TableSetupFailed(
            "Failed to deploy test table".to_string(),
        ));
    }

    if !global.is_silent() {
        aprintln!("{} {}", p_g("✅"), "Test table ready");
    }

    Ok(())
}
