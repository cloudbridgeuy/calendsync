//! Events CLI commands.

use clap::{Parser, Subcommand};
use uuid::Uuid;

/// Events management commands.
#[derive(Debug, Parser)]
pub struct EventsCommand {
    #[command(subcommand)]
    pub action: EventsAction,
}

/// Available events actions.
#[derive(Debug, Subcommand)]
pub enum EventsAction {
    /// Watch real-time SSE events.
    Watch {
        /// Calendar ID to watch.
        calendar_id: Uuid,
        /// Resume from event ID.
        #[arg(long)]
        last_event_id: Option<u64>,
    },
}
