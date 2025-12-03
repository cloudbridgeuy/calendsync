//! User CLI commands.

use clap::{Parser, Subcommand};
use uuid::Uuid;

/// User management commands.
#[derive(Debug, Parser)]
pub struct UsersCommand {
    #[command(subcommand)]
    pub action: UsersAction,
}

/// Available user actions.
#[derive(Debug, Subcommand)]
pub enum UsersAction {
    /// List all users.
    List,
    /// Create a new user.
    Create {
        /// User name.
        #[arg(long)]
        name: String,
        /// User email.
        #[arg(long)]
        email: String,
    },
    /// Get user by ID.
    Get {
        /// User ID.
        id: Uuid,
    },
    /// Delete user by ID.
    Delete {
        /// User ID.
        id: Uuid,
    },
}
