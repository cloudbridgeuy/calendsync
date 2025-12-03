//! Calendar CLI commands.

use clap::{Parser, Subcommand};
use uuid::Uuid;

/// Calendar management commands.
#[derive(Debug, Parser)]
pub struct CalendarsCommand {
    #[command(subcommand)]
    pub action: CalendarsAction,
}

/// Available calendar actions.
#[derive(Debug, Subcommand)]
pub enum CalendarsAction {
    /// List all calendars.
    List,
    /// Create a new calendar.
    Create {
        /// Calendar name.
        #[arg(long)]
        name: String,
        /// Calendar color (CSS color value).
        #[arg(long, default_value = "#3B82F6")]
        color: String,
        /// Calendar description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Get calendar by ID.
    Get {
        /// Calendar ID.
        id: Uuid,
    },
    /// Update a calendar.
    Update {
        /// Calendar ID.
        id: Uuid,
        /// New name.
        #[arg(long)]
        name: Option<String>,
        /// New color.
        #[arg(long)]
        color: Option<String>,
        /// New description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete calendar by ID.
    Delete {
        /// Calendar ID.
        id: Uuid,
    },
}
