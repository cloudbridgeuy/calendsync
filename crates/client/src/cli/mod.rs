//! CLI command definitions.

pub mod calendars;
pub mod entries;
pub mod events;
pub mod health;
pub mod users;

use clap::{Parser, Subcommand, ValueEnum};

/// CLI client for calendsync API.
#[derive(Debug, Parser)]
#[command(name = "calendsync-client")]
#[command(about = "CLI client for calendsync API", long_about = None)]
pub struct Cli {
    /// Server base URL.
    #[arg(long, env = "CALENDSYNC_URL", default_value = "http://localhost:3000")]
    pub base_url: String,

    /// Output format.
    #[arg(long, default_value = "pretty")]
    pub format: OutputFormat,

    /// Suppress non-essential output.
    #[arg(long)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Output format options.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    /// Raw JSON output.
    Json,
    /// Human-readable output with colors.
    #[default]
    Pretty,
}

/// Available commands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// User management.
    Users(users::UsersCommand),
    /// Calendar management.
    Calendars(calendars::CalendarsCommand),
    /// Calendar entry management.
    Entries(entries::EntriesCommand),
    /// Watch real-time SSE events.
    Events(events::EventsCommand),
    /// Server health checks.
    Health(health::HealthCommand),
}
