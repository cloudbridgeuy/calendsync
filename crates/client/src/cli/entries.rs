//! Entry CLI commands.

use chrono::{NaiveDate, NaiveTime};
use clap::{Parser, Subcommand, ValueEnum};
use uuid::Uuid;

// Re-export core EntryType for API usage
pub use calendsync_core::calendar::EntryType as CoreEntryType;

/// Entry management commands.
#[derive(Debug, Parser)]
pub struct EntriesCommand {
    #[command(subcommand)]
    pub action: EntriesAction,
}

/// CLI entry type for creation (with clap ValueEnum).
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum EntryType {
    AllDay,
    Timed,
    Task,
    MultiDay,
}

impl From<EntryType> for CoreEntryType {
    fn from(t: EntryType) -> Self {
        match t {
            EntryType::AllDay => CoreEntryType::AllDay,
            EntryType::Timed => CoreEntryType::Timed,
            EntryType::Task => CoreEntryType::Task,
            EntryType::MultiDay => CoreEntryType::MultiDay,
        }
    }
}

/// Available entry actions.
#[derive(Debug, Subcommand)]
pub enum EntriesAction {
    /// List entries with filters.
    List {
        /// Filter by calendar ID.
        #[arg(long)]
        calendar_id: Option<Uuid>,
        /// Start date (YYYY-MM-DD).
        #[arg(long)]
        start: Option<NaiveDate>,
        /// End date (YYYY-MM-DD).
        #[arg(long)]
        end: Option<NaiveDate>,
        /// Center date for query.
        #[arg(long)]
        highlighted_day: Option<NaiveDate>,
        /// Days before highlighted day.
        #[arg(long, default_value = "3")]
        before: u32,
        /// Days after highlighted day.
        #[arg(long, default_value = "3")]
        after: u32,
    },
    /// Create a new entry.
    Create {
        /// Calendar ID.
        #[arg(long)]
        calendar_id: Uuid,
        /// Entry title.
        #[arg(long)]
        title: String,
        /// Entry date (YYYY-MM-DD).
        #[arg(long)]
        date: NaiveDate,
        /// Entry type.
        #[arg(long, value_enum)]
        entry_type: EntryType,
        /// Optional description.
        #[arg(long)]
        description: Option<String>,
        /// Optional location.
        #[arg(long)]
        location: Option<String>,
        /// Start time (HH:MM) for timed entries.
        #[arg(long)]
        start_time: Option<NaiveTime>,
        /// End time (HH:MM) for timed entries.
        #[arg(long)]
        end_time: Option<NaiveTime>,
        /// End date (YYYY-MM-DD) for multi-day entries.
        #[arg(long)]
        end_date: Option<NaiveDate>,
        /// Optional accent color.
        #[arg(long)]
        color: Option<String>,
    },
    /// Get entry by ID.
    Get {
        /// Entry ID.
        id: Uuid,
    },
    /// Update an entry.
    Update {
        /// Entry ID.
        id: Uuid,
        /// New title.
        #[arg(long)]
        title: Option<String>,
        /// New date.
        #[arg(long)]
        date: Option<NaiveDate>,
        /// New entry type.
        #[arg(long)]
        entry_type: Option<EntryType>,
        /// New description.
        #[arg(long)]
        description: Option<String>,
        /// New location.
        #[arg(long)]
        location: Option<String>,
        /// New start time.
        #[arg(long)]
        start_time: Option<NaiveTime>,
        /// New end time.
        #[arg(long)]
        end_time: Option<NaiveTime>,
        /// New end date.
        #[arg(long)]
        end_date: Option<NaiveDate>,
        /// New color.
        #[arg(long)]
        color: Option<String>,
        /// Mark task as completed.
        #[arg(long)]
        completed: Option<bool>,
    },
    /// Delete entry by ID.
    Delete {
        /// Entry ID.
        id: Uuid,
    },
    /// Toggle task completion status.
    Toggle {
        /// Entry ID.
        id: Uuid,
    },
}
