//! Pure functions for calculating deployment plans (Functional Core).

use super::config::{GsiConfig, TableConfig};

/// Represents the current state of a table.
#[derive(Debug, Clone)]
pub struct TableState {
    pub status: TableStatus,
    pub gsis: Vec<GsiState>,
}

/// Table status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableStatus {
    Active,
    Creating,
    Updating,
    Deleting,
}

/// GSI state.
#[derive(Debug, Clone)]
pub struct GsiState {
    pub name: String,
    pub status: GsiStatus,
}

/// GSI status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GsiStatus {
    Active,
    Creating,
    Updating,
    Deleting,
}

/// Planned changes for deployment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployPlan {
    /// Table doesn't exist, needs to be created.
    CreateTable { config: TableConfig },
    /// Table exists, GSIs need to be added.
    AddGsis {
        table_name: String,
        gsis_to_add: Vec<GsiConfig>,
    },
    /// Table is up to date, no changes needed.
    NoChanges { table_name: String },
}

/// Plan for destroying a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestroyPlan {
    /// Table exists and will be deleted.
    DeleteTable { table_name: String },
    /// Table doesn't exist, nothing to do.
    AlreadyGone { table_name: String },
}

/// Pure function: Calculate what changes are needed to reach desired state.
pub fn calculate_deploy_plan(current: Option<&TableState>, desired: &TableConfig) -> DeployPlan {
    match current {
        None => DeployPlan::CreateTable {
            config: desired.clone(),
        },
        Some(state) => {
            // Find GSIs that exist in desired but not in current
            let existing_gsi_names: Vec<&str> =
                state.gsis.iter().map(|g| g.name.as_str()).collect();

            let gsis_to_add: Vec<GsiConfig> = desired
                .gsis
                .iter()
                .filter(|gsi| !existing_gsi_names.contains(&gsi.name.as_str()))
                .cloned()
                .collect();

            if gsis_to_add.is_empty() {
                DeployPlan::NoChanges {
                    table_name: desired.table_name.clone(),
                }
            } else {
                DeployPlan::AddGsis {
                    table_name: desired.table_name.clone(),
                    gsis_to_add,
                }
            }
        }
    }
}

/// Pure function: Calculate destroy plan.
pub fn calculate_destroy_plan(current: Option<&TableState>, table_name: &str) -> DestroyPlan {
    match current {
        Some(_) => DestroyPlan::DeleteTable {
            table_name: table_name.to_string(),
        },
        None => DestroyPlan::AlreadyGone {
            table_name: table_name.to_string(),
        },
    }
}

/// Pure function: Format a deploy plan for display.
pub fn format_deploy_plan(plan: &DeployPlan) -> Vec<String> {
    match plan {
        DeployPlan::CreateTable { config } => {
            let mut lines = vec![
                format!("+ Create table: {}", config.table_name),
                format!("  Partition key: {} (S)", config.partition_key.name),
            ];
            if let Some(sk) = &config.sort_key {
                lines.push(format!("  Sort key: {} (S)", sk.name));
            }
            for gsi in &config.gsis {
                lines.push(format!("  + GSI: {}", gsi.name));
                lines.push(format!("    Partition key: {} (S)", gsi.partition_key.name));
                if let Some(sk) = &gsi.sort_key {
                    lines.push(format!("    Sort key: {} (S)", sk.name));
                }
            }
            lines.push("  Billing: PAY_PER_REQUEST".to_string());
            lines
        }
        DeployPlan::AddGsis {
            table_name,
            gsis_to_add,
        } => {
            let mut lines = vec![format!("~ Update table: {}", table_name)];
            for gsi in gsis_to_add {
                lines.push(format!("  + Add GSI: {}", gsi.name));
            }
            lines
        }
        DeployPlan::NoChanges { table_name } => {
            vec![format!("= Table '{}' is up to date", table_name)]
        }
    }
}

/// Pure function: Format a destroy plan for display.
pub fn format_destroy_plan(plan: &DestroyPlan) -> Vec<String> {
    match plan {
        DestroyPlan::DeleteTable { table_name } => {
            vec![format!(
                "- Delete table: {} (ALL DATA WILL BE LOST)",
                table_name
            )]
        }
        DestroyPlan::AlreadyGone { table_name } => {
            vec![format!("= Table '{}' does not exist", table_name)]
        }
    }
}
