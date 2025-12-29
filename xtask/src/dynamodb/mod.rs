//! DynamoDB infrastructure management commands.

mod client;
mod config;
mod deploy;
mod error;
mod planning;
mod seed;

pub use error::{DynamodbError, Result};

use crate::prelude::*;
use dialoguer::Confirm;

/// DynamoDB infrastructure management commands.
#[derive(Debug, clap::Parser)]
pub struct DynamodbCommand {
    #[command(subcommand)]
    pub action: DynamodbAction,
}

/// Available DynamoDB actions.
#[derive(Debug, clap::Subcommand)]
pub enum DynamodbAction {
    /// Deploy or destroy DynamoDB table infrastructure.
    Deploy(DeployCommand),

    /// Seed a calendar with mock entries.
    Seed(SeedCommand),
}

/// Deploy or update DynamoDB infrastructure.
#[derive(Debug, clap::Parser)]
#[command(long_about = "Deploy or destroy DynamoDB table infrastructure.

By default, this command creates or updates the calendsync DynamoDB table
with the required schema and Global Secondary Indexes (GSIs).

The command shows a plan of changes before applying and asks for confirmation.

Environment variables:
  AWS_ENDPOINT_URL    - Use local DynamoDB (e.g., http://localhost:8000)
  AWS_REGION          - AWS region (defaults to us-east-1)
  AWS_PROFILE         - AWS profile to use for credentials")]
pub struct DeployCommand {
    /// Skip confirmation prompts.
    #[arg(long)]
    pub force: bool,

    /// Destroy the table instead of creating/updating.
    #[arg(long)]
    pub destroy: bool,

    /// Table name to use.
    #[arg(long, default_value = "calendsync")]
    pub table_name: String,
}

/// Seed a calendar with mock entries.
#[derive(Debug, clap::Parser)]
#[command(long_about = "Generate and insert mock calendar entries into DynamoDB.

Creates realistic demo entries spread around the highlighted day,
including multi-day events, all-day events, timed activities, and tasks.

The entries are distributed across roughly a week centered on the
highlighted day to demonstrate various calendar features.")]
pub struct SeedCommand {
    /// Calendar ID to seed entries for.
    #[arg(long, value_name = "UUID")]
    pub calendar_id: uuid::Uuid,

    /// Center date for generated entries (defaults to today).
    /// Format: YYYY-MM-DD
    #[arg(long, value_name = "DATE")]
    pub highlighted_day: Option<chrono::NaiveDate>,

    /// Number of entries to generate.
    #[arg(long, default_value = "15")]
    pub count: u32,

    /// Table name to use.
    #[arg(long, default_value = "calendsync")]
    pub table_name: String,

    /// Skip confirmation prompts.
    #[arg(long)]
    pub force: bool,
}

/// Main entry point for dynamodb command.
pub async fn run(command: DynamodbCommand, global: crate::Global) -> Result<()> {
    match command.action {
        DynamodbAction::Deploy(deploy_cmd) => run_deploy(deploy_cmd, &global).await,
        DynamodbAction::Seed(seed_cmd) => run_seed(seed_cmd, &global).await,
    }
}

async fn run_deploy(cmd: DeployCommand, global: &crate::Global) -> Result<()> {
    let aws_config = client::AwsConfig::default();

    if !global.is_silent() {
        aprintln!("{} {}", p_b("Target:"), aws_config.target_display());
        aprintln!();
    }

    let dynamo_client = client::create_client(&aws_config).await?;
    let current_state = client::get_table_state(&dynamo_client, &cmd.table_name).await?;

    if cmd.destroy {
        // Destroy flow
        let plan = planning::calculate_destroy_plan(current_state.as_ref(), &cmd.table_name);

        if !global.is_silent() {
            aprintln!("{}", p_y("Destroy Plan:"));
            for line in planning::format_destroy_plan(&plan) {
                aprintln!("  {}", p_r(&line));
            }
            aprintln!();
        }

        if matches!(plan, planning::DestroyPlan::AlreadyGone { .. }) {
            if !global.is_silent() {
                aprintln!("{}", p_g("Nothing to destroy."));
            }
            return Ok(());
        }

        if !cmd.force {
            let confirmed = Confirm::new()
                .with_prompt("Are you sure you want to delete this table? ALL DATA WILL BE LOST")
                .default(false)
                .interact()
                .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?;

            if !confirmed {
                return Err(DynamodbError::UserCancelled);
            }
        }

        if !global.is_silent() {
            aprintln!("{}", p_b("Deleting table..."));
        }

        deploy::execute_destroy_plan(&dynamo_client, &plan).await?;

        if !global.is_silent() {
            aprintln!("{}", p_g("Table destroyed successfully."));
        }
    } else {
        // Deploy flow
        let table_config = config::calendsync_table_config().with_table_name(&cmd.table_name);

        let plan = planning::calculate_deploy_plan(current_state.as_ref(), &table_config);

        if !global.is_silent() {
            aprintln!("{}", p_c("Deploy Plan:"));
            for line in planning::format_deploy_plan(&plan) {
                if line.starts_with('+') {
                    aprintln!("  {}", p_g(&line));
                } else if line.starts_with('-') {
                    aprintln!("  {}", p_r(&line));
                } else if line.starts_with('~') {
                    aprintln!("  {}", p_y(&line));
                } else {
                    aprintln!("  {}", line);
                }
            }
            aprintln!();
        }

        if matches!(plan, planning::DeployPlan::NoChanges { .. }) {
            if !global.is_silent() {
                aprintln!("{}", p_g("Infrastructure is up to date."));
            }
            return Ok(());
        }

        if !cmd.force {
            let confirmed = Confirm::new()
                .with_prompt("Apply these changes?")
                .default(true)
                .interact()
                .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?;

            if !confirmed {
                return Err(DynamodbError::UserCancelled);
            }
        }

        if !global.is_silent() {
            aprintln!("{}", p_b("Applying changes..."));
        }

        deploy::execute_deploy_plan(&dynamo_client, &plan).await?;

        if !global.is_silent() {
            aprintln!("{}", p_g("Infrastructure deployed successfully."));
        }
    }

    Ok(())
}

async fn run_seed(cmd: SeedCommand, global: &crate::Global) -> Result<()> {
    let aws_config = client::AwsConfig::default();
    let highlighted_day = cmd
        .highlighted_day
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    if !global.is_silent() {
        aprintln!("{} {}", p_b("Target:"), aws_config.target_display());
        aprintln!("{} {}", p_b("Table:"), cmd.table_name);
        aprintln!("{} {}", p_b("Calendar:"), cmd.calendar_id);
        aprintln!("{} {}", p_b("Center date:"), highlighted_day);
        aprintln!("{} {}", p_b("Entry count:"), cmd.count);
        aprintln!();
    }

    let dynamo_client = client::create_client(&aws_config).await?;

    // Verify table exists
    let table_state = client::get_table_state(&dynamo_client, &cmd.table_name).await?;
    if table_state.is_none() {
        return Err(DynamodbError::TableNotFound {
            table_name: cmd.table_name,
        });
    }

    // Generate entries
    let entries = seed::generate_seed_entries(cmd.calendar_id, highlighted_day, cmd.count);

    if !global.is_silent() {
        aprintln!("{}", p_c("Entries to create:"));
        for entry in entries.iter().take(5) {
            aprintln!(
                "  {} - {} ({})",
                entry.start_date,
                entry.title,
                seed::format_entry_kind(&entry.kind)
            );
        }
        if entries.len() > 5 {
            aprintln!("  ... and {} more", entries.len() - 5);
        }
        aprintln!();
    }

    if !cmd.force {
        let confirmed = Confirm::new()
            .with_prompt(format!("Insert {} entries?", entries.len()))
            .default(true)
            .interact()
            .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?;

        if !confirmed {
            return Err(DynamodbError::UserCancelled);
        }
    }

    let inserted = seed::seed_entries(&dynamo_client, &cmd.table_name, &entries).await?;

    if !global.is_silent() {
        aprintln!("{} {} entries inserted.", p_g("Success:"), inserted);
    }

    Ok(())
}
