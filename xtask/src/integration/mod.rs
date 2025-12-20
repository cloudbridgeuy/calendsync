//! Integration test infrastructure.
//!
//! This module provides commands for running integration tests against
//! real infrastructure (DynamoDB Local via Docker).
//!
//! # Usage
//!
//! ```bash
//! # Run all integration tests
//! cargo xtask integration
//!
//! # Run only SQLite integration tests
//! cargo xtask integration --sqlite
//!
//! # Run only DynamoDB integration tests
//! cargo xtask integration --dynamodb
//!
//! # Skip container management (assumes DynamoDB Local is already running)
//! cargo xtask integration --dynamodb --no-docker
//! ```

pub mod error;

pub use error::{IntegrationError, Result};

use crate::prelude::*;

/// Integration test command.
#[derive(Debug, clap::Parser)]
#[command(long_about = "Run integration tests against real infrastructure.

This command manages Docker containers for local testing and runs
the integration test suite against real storage backends.

By default, it runs tests for both SQLite and DynamoDB backends.
The command automatically starts DynamoDB Local via Docker before
running DynamoDB tests and stops it afterward.

Environment variables:
  AWS_ENDPOINT_URL    - Override DynamoDB endpoint (default: http://localhost:8000)
  AWS_REGION          - AWS region for DynamoDB (default: us-east-1)")]
pub struct IntegrationCommand {
    /// Run only SQLite integration tests.
    #[arg(long, conflicts_with = "dynamodb")]
    pub sqlite: bool,

    /// Run only DynamoDB integration tests.
    #[arg(long, conflicts_with = "sqlite")]
    pub dynamodb: bool,

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
    // Determine which backends to test
    // If neither flag is set, run both; otherwise run only the specified one
    let run_sqlite = command.sqlite || !command.dynamodb;
    let run_dynamodb = command.dynamodb || !command.sqlite;

    if !global.is_silent() {
        aprintln!("{}", p_b("Integration Tests"));
        aprintln!();
        aprintln!(
            "{} SQLite: {}, DynamoDB: {}",
            p_b("Backends:"),
            if run_sqlite { p_g("yes") } else { p_y("no") },
            if run_dynamodb { p_g("yes") } else { p_y("no") }
        );
        aprintln!();
    }

    let mut all_passed = true;

    // Run SQLite tests (no infrastructure needed)
    if run_sqlite && !run_sqlite_tests(&global).await? {
        all_passed = false;
    }

    // Run DynamoDB tests (requires Docker)
    if run_dynamodb && !run_dynamodb_tests(&command, &global).await? {
        all_passed = false;
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

/// Run SQLite integration tests.
async fn run_sqlite_tests(global: &crate::Global) -> Result<bool> {
    if !global.is_silent() {
        aprintln!(
            "{} {}",
            p_b("🔧"),
            p_b("Running SQLite integration tests...")
        );
    }

    // SQLite tests use in-memory database, no setup needed
    let status = tokio::process::Command::new("cargo")
        .args([
            "test",
            "-p",
            "calendsync",
            "--features",
            "sqlite",
            "--no-default-features",
        ])
        .status()
        .await?;

    if status.success() {
        if !global.is_silent() {
            aprintln!("{} {}", p_g("✅"), "SQLite integration tests passed");
        }
        Ok(true)
    } else {
        aprintln!("{} {}", p_r("❌"), "SQLite integration tests failed");
        Ok(false)
    }
}

/// Run DynamoDB integration tests.
async fn run_dynamodb_tests(command: &IntegrationCommand, global: &crate::Global) -> Result<bool> {
    if !global.is_silent() {
        aprintln!(
            "{} {}",
            p_b("🔧"),
            p_b("Running DynamoDB integration tests...")
        );
    }

    // Start DynamoDB Local container if needed
    let container_started = if !command.no_docker {
        start_dynamodb_container(command.health_timeout, global).await?
    } else {
        if !global.is_silent() {
            aprintln!(
                "{} {}",
                p_y("⚠️"),
                "Skipping container management (--no-docker)"
            );
        }
        false
    };

    // Set up the test table
    if !global.is_silent() {
        aprintln!("{} {}", p_b("📦"), "Setting up test table...");
    }
    setup_test_table(global).await?;

    // Run the tests
    let status = tokio::process::Command::new("cargo")
        .args([
            "test",
            "-p",
            "calendsync",
            "--features",
            "dynamodb",
            "--no-default-features",
        ])
        .env("AWS_ENDPOINT_URL", "http://localhost:8000")
        .env("AWS_REGION", "us-east-1")
        .env("AWS_ACCESS_KEY_ID", "test")
        .env("AWS_SECRET_ACCESS_KEY", "test")
        .status()
        .await?;

    let passed = status.success();

    // Clean up container if we started it
    if container_started && !command.keep_containers {
        stop_dynamodb_container(global).await?;
    } else if container_started && command.keep_containers && !global.is_silent() {
        aprintln!(
            "{} {}",
            p_y("⚠️"),
            "Container left running (--keep-containers)"
        );
    }

    if passed {
        if !global.is_silent() {
            aprintln!("{} {}", p_g("✅"), "DynamoDB integration tests passed");
        }
        Ok(true)
    } else {
        aprintln!("{} {}", p_r("❌"), "DynamoDB integration tests failed");
        Ok(false)
    }
}

/// Start the DynamoDB Local container.
async fn start_dynamodb_container(timeout_secs: u64, global: &crate::Global) -> Result<bool> {
    // Check if Docker is available
    let docker_check = tokio::process::Command::new("docker")
        .args(["--version"])
        .output()
        .await?;

    if !docker_check.status.success() {
        return Err(IntegrationError::DockerNotAvailable(
            "docker command not found or not working".to_string(),
        ));
    }

    // Check if container is already running
    let ps_output = tokio::process::Command::new("docker")
        .args(["ps", "-q", "-f", "name=calendsync-dynamodb"])
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
        return Ok(false); // Container exists but we didn't start it
    }

    if !global.is_silent() {
        aprintln!("{} {}", p_b("🐳"), "Starting DynamoDB Local container...");
    }

    // Start container using docker compose
    let compose_status = tokio::process::Command::new("docker")
        .args(["compose", "up", "-d", "dynamodb-local"])
        .status()
        .await?;

    if !compose_status.success() {
        return Err(IntegrationError::ContainerFailed(
            "docker compose up failed".to_string(),
        ));
    }

    // Wait for container to be healthy
    if !global.is_silent() {
        aprintln!(
            "{} {}",
            p_b("⏳"),
            format!("Waiting for container health (max {}s)...", timeout_secs)
        );
    }

    let start = std::time::Instant::now();
    loop {
        if start.elapsed().as_secs() > timeout_secs {
            return Err(IntegrationError::ContainerNotHealthy {
                name: "calendsync-dynamodb".to_string(),
                timeout_secs,
            });
        }

        // Check container health
        let health_output = tokio::process::Command::new("docker")
            .args([
                "inspect",
                "--format",
                "{{.State.Health.Status}}",
                "calendsync-dynamodb",
            ])
            .output()
            .await?;

        let health_status = String::from_utf8_lossy(&health_output.stdout)
            .trim()
            .to_string();

        if health_status == "healthy" {
            break;
        }

        // Also try a direct connection check
        let curl_check = tokio::process::Command::new("curl")
            .args([
                "-s",
                "-o",
                "/dev/null",
                "-w",
                "%{http_code}",
                "http://localhost:8000",
            ])
            .output()
            .await;

        if let Ok(output) = curl_check {
            let status_code = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if status_code == "400" {
                // DynamoDB returns 400 for invalid requests, but that means it's running
                break;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    if !global.is_silent() {
        aprintln!("{} {}", p_g("✅"), "DynamoDB Local is ready");
    }

    Ok(true)
}

/// Stop the DynamoDB Local container.
async fn stop_dynamodb_container(global: &crate::Global) -> Result<()> {
    if !global.is_silent() {
        aprintln!("{} {}", p_b("🐳"), "Stopping DynamoDB Local container...");
    }

    let status = tokio::process::Command::new("docker")
        .args(["compose", "down", "dynamodb-local"])
        .status()
        .await?;

    if !status.success() {
        return Err(IntegrationError::ContainerFailed(
            "docker compose down failed".to_string(),
        ));
    }

    if !global.is_silent() {
        aprintln!("{} {}", p_g("✅"), "Container stopped");
    }

    Ok(())
}

/// Set up the test table in DynamoDB Local.
async fn setup_test_table(global: &crate::Global) -> Result<()> {
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
        .env("AWS_ENDPOINT_URL", "http://localhost:8000")
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
