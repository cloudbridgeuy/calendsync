//! Health CLI commands.

use clap::{Parser, Subcommand};

/// Health check commands.
#[derive(Debug, Parser)]
pub struct HealthCommand {
    #[command(subcommand)]
    pub action: HealthAction,
}

/// Available health actions.
#[derive(Debug, Subcommand)]
pub enum HealthAction {
    /// Active SSR pool health check.
    Ssr,
    /// Passive SSR pool statistics.
    SsrStats,
}
