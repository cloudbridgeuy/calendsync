//! Standalone seed command for seeding calendars with authenticated requests.
//!
//! Usage: cargo xtask seed <CALENDAR_ID> --session <SESSION_ID>

use chrono::Utc;
use uuid::Uuid;

use crate::dev::seed::{generate_entries_for_seeding, SeedEntry};
use crate::Global;

/// Seed command arguments.
#[derive(Debug, clap::Args)]
pub struct SeedCommand {
    /// Calendar ID to seed with entries.
    pub calendar_id: Uuid,

    /// Session ID for authentication (from browser cookie or header).
    #[arg(long)]
    pub session: String,

    /// Number of entries to create (default: 25).
    #[arg(long, default_value = "25")]
    pub count: u32,

    /// Base URL of the server (default: http://localhost:3000).
    #[arg(long, default_value = "http://localhost:3000")]
    pub base_url: String,
}

/// Creates an entry via the HTTP API with authentication.
async fn create_entry_via_http_authenticated(
    client: &reqwest::Client,
    base_url: &str,
    session: &str,
    entry: &SeedEntry,
) -> anyhow::Result<()> {
    let url = format!("{base_url}/api/entries");

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {session}"))
        .form(entry)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create entry: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "Failed to create entry '{}': {status} - {body}",
            entry.title
        ));
    }

    Ok(())
}

/// Run the seed command.
pub async fn run(cmd: SeedCommand, global: Global) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    if !global.is_silent() {
        println!(
            "Seeding calendar {} with {} entries...",
            cmd.calendar_id, cmd.count
        );
    }

    let center_date = Utc::now().date_naive();
    let entries = generate_entries_for_seeding(cmd.calendar_id, center_date, cmd.count);

    for (i, entry) in entries.iter().enumerate() {
        create_entry_via_http_authenticated(&client, &cmd.base_url, &cmd.session, entry).await?;

        if global.is_verbose() {
            println!("  [{}/{}] Created: {}", i + 1, entries.len(), entry.title);
        }
    }

    if !global.is_silent() {
        println!(
            "Created {} entries in calendar {}",
            entries.len(),
            cmd.calendar_id
        );
    }

    Ok(())
}
