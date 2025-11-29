use super::error::{Error, Result};
use crate::prelude::*;
use serde_json::Value;

const GITHUB_REPO: &str = "VNTANA-3D/vntana-devops-cli";

/// Workflow status information
#[derive(Debug, Clone)]
pub struct WorkflowStatus {
    pub status: String,
    pub conclusion: Option<String>,
}

/// Get workflow runs for a specific tag
pub async fn get_workflow_runs(
    tag: &str,
    workflow_file: &str,
    global: &crate::Global,
) -> Result<Option<WorkflowStatus>> {
    debug_log(global, &format!("Querying workflow runs for tag: {tag}"));

    let output = execute_command(
        "gh",
        &[
            "run",
            "list",
            &format!("--repo={GITHUB_REPO}"),
            &format!("--workflow={workflow_file}"),
            "--event=push",
            "--limit=5",
            "--json=status,conclusion,headBranch,headSha,event",
        ],
    )
    .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::GitHubApi(format!(
            "Failed to list workflow runs: {stderr}"
        )));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);

    // Parse JSON array
    let runs: Vec<Value> = serde_json::from_str(&output_str)?;

    // Find the run for this tag
    for run in runs {
        let head_branch = run.get("headBranch").and_then(|v| v.as_str()).unwrap_or("");
        let event = run.get("event").and_then(|v| v.as_str()).unwrap_or("");
        let head_sha = run.get("headSha").and_then(|v| v.as_str());

        // Match by tag name or by recent push event
        if head_branch == tag || (event == "push" && head_sha.is_some()) {
            let status = run
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let conclusion = run
                .get("conclusion")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            debug_log(
                global,
                &format!("Found workflow: status={status}, conclusion={conclusion:?}"),
            );

            return Ok(Some(WorkflowStatus { status, conclusion }));
        }
    }

    debug_log(global, "No matching workflow run found");
    Ok(None)
}

/// Check if vnt binary is installed
pub fn is_vnt_installed() -> bool {
    command_exists("vnt")
}

/// Run vnt upgrade command with sudo
pub async fn run_vnt_upgrade(global: &crate::Global) -> Result<()> {
    info_log(
        global,
        "Running 'vnt upgrade' (sudo required for system-wide installation)...",
    );

    // Use sudo to run vnt upgrade since it needs to replace the binary in /usr/local/bin
    let output = tokio::process::Command::new("sudo")
        .arg("vnt")
        .arg("upgrade")
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .await?;

    if !output.success() {
        return Err(Error::Generic("vnt upgrade command failed".to_string()));
    }

    Ok(())
}

// Helper logging functions
fn info_log(global: &crate::Global, message: &str) {
    if !global.is_silent() {
        aprintln!("{}", p_b(message));
    }
}

fn debug_log(global: &crate::Global, message: &str) {
    if !global.is_silent() && global.is_verbose() {
        aprintln!("{}", p_m(&format!("DEBUG: {message}")));
    }
}
