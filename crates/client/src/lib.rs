//! calendsync_client - CLI client for calendsync API.

pub mod cli;
pub mod client;
pub mod error;
pub mod output;

pub use client::CalendsyncClient;
pub use error::{ClientError, Result};
