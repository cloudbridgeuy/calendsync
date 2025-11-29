use super::error::{Error, Result};
use crate::prelude::*;

const GITHUB_REPO: &str = "VNTANA-3D/vntana-devops-cli";

/// Check if we're on the main branch
pub async fn check_main_branch(global: &crate::Global) -> Result<()> {
    info_log(global, "Checking current git branch...");

    let output = execute_command("git", &["branch", "--show-current"]).await?;

    if !output.status.success() {
        return Err(Error::Git("Failed to determine current branch".to_string()));
    }

    let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if current_branch != "main" {
        return Err(Error::NotOnMainBranch(format!(
            "Current branch: {current_branch}"
        )));
    }

    info_log(global, "On main branch ✓");
    Ok(())
}

/// Check if working directory is clean
pub async fn check_clean_working_dir(global: &crate::Global) -> Result<()> {
    info_log(global, "Checking working directory status...");

    let output = execute_command("git", &["status", "--porcelain"]).await?;

    if !output.status.success() {
        return Err(Error::Git(
            "Failed to check working directory status".to_string(),
        ));
    }

    let status_output = String::from_utf8_lossy(&output.stdout);

    if !status_output.trim().is_empty() {
        return Err(Error::DirtyWorkingDirectory);
    }

    info_log(global, "Working directory is clean ✓");
    Ok(())
}

/// Check if GitHub CLI is available and authenticated
pub async fn check_gh_cli(global: &crate::Global) -> Result<()> {
    info_log(global, "Checking GitHub CLI availability...");

    // Check if gh is installed
    super::error::require_command("gh", "Install from: https://cli.github.com/")?;

    info_log(global, "GitHub CLI found");

    // Check if authenticated
    info_log(global, "Checking GitHub CLI authentication...");

    let output = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        execute_command("gh", &["auth", "status"]),
    )
    .await;

    match output {
        Ok(Ok(result)) if result.status.success() => {
            info_log(global, "GitHub CLI authenticated successfully");
            Ok(())
        }
        Ok(Ok(_)) => Err(Error::GitHubCli(
            "GitHub CLI is not authenticated. Run: gh auth login".to_string(),
        )),
        Ok(Err(e)) => Err(Error::Io(e)),
        Err(_) => Err(Error::GitHubCli(
            "Authentication check timed out. Run: gh auth login".to_string(),
        )),
    }
}

/// Structure to hold check run information
#[derive(Debug, serde::Deserialize)]
struct CheckRun {
    name: String,
    status: String,
    conclusion: Option<String>,
}

/// Check if all GitHub checks are passing for current commit
pub async fn check_commit_status(global: &crate::Global) -> Result<bool> {
    info_log(global, "Checking GitHub commit status...");

    // Get current commit SHA
    let output = execute_command("git", &["rev-parse", "HEAD"]).await?;

    if !output.status.success() {
        return Err(Error::Git("Failed to get current commit SHA".to_string()));
    }

    let commit_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let short_sha = &commit_sha[..7.min(commit_sha.len())];
    info_log(global, &format!("Current commit: {short_sha}"));

    // Get check runs for this commit
    let output = execute_command(
        "gh",
        &[
            "api",
            &format!("repos/{GITHUB_REPO}/commits/{commit_sha}/check-runs"),
            "--jq",
            ".check_runs[] | {name: .name, status: .status, conclusion: .conclusion}",
        ],
    )
    .await?;

    if !output.status.success() {
        return Err(Error::GitHubApi(
            "Failed to fetch check runs from GitHub".to_string(),
        ));
    }

    let check_runs_output = String::from_utf8_lossy(&output.stdout);

    if check_runs_output.trim().is_empty() {
        warning_log(global, "No check runs found for current commit");
        warning_log(
            global,
            "This might mean checks haven't started yet or there are no checks configured",
        );
        println!();
        return ask_user_confirmation("Continue anyway?", false);
    }

    // Parse check runs
    let mut has_failures = false;
    let mut has_pending = false;
    let mut failed_checks = Vec::new();
    let mut pending_checks = Vec::new();

    for line in check_runs_output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<CheckRun>(line) {
            Ok(check) => {
                if check.status != "completed" {
                    has_pending = true;
                    pending_checks.push(check.name.clone());
                } else if let Some(conclusion) = &check.conclusion {
                    if conclusion != "success" && conclusion != "skipped" && conclusion != "neutral"
                    {
                        has_failures = true;
                        failed_checks.push(format!("{} ({})", check.name, conclusion));
                    }
                }
            }
            Err(e) => {
                debug_log(global, &format!("Failed to parse check run: {e}"));
            }
        }
    }

    // Report status
    if has_failures {
        aprintln!("{}", p_r("Some checks have failed:"));
        for check in &failed_checks {
            aprintln!("  ❌ {}", check);
        }
        println!();
        aprintln!("{}", p_r("Cannot create release with failing checks"));
        aprintln!(
            "{}",
            p_b(&format!(
                "View details at: https://github.com/{GITHUB_REPO}/commit/{commit_sha}/checks"
            ))
        );
        return Err(Error::CiChecksFailed(failed_checks.join(", ")));
    }

    if has_pending {
        warning_log(global, "Some checks are still pending:");
        for check in &pending_checks {
            aprintln!("  ⏳ {}", check);
        }
        println!();
        warning_log(
            global,
            "It's recommended to wait for all checks to complete before releasing",
        );
        aprintln!(
            "{}",
            p_b(&format!(
                "View details at: https://github.com/{GITHUB_REPO}/commit/{commit_sha}/checks"
            ))
        );
        println!();
        return ask_user_confirmation("Continue anyway?", false);
    }

    info_log(global, "All GitHub checks passing ✓");
    Ok(true)
}

/// Helper function to ask for user confirmation
fn ask_user_confirmation(message: &str, default: bool) -> Result<bool> {
    use dialoguer::Confirm;

    Confirm::new()
        .with_prompt(message)
        .default(default)
        .interact()
        .map_err(|e| Error::Generic(format!("Failed to get user input: {e}")))
}

// Helper logging functions (reuse pattern from build/common.rs)
fn info_log(global: &crate::Global, message: &str) {
    if !global.is_silent() {
        aprintln!("{}", p_b(message));
    }
}

fn warning_log(global: &crate::Global, message: &str) {
    if !global.is_silent() {
        aprintln!("{}", p_y(message));
    }
}

fn debug_log(global: &crate::Global, message: &str) {
    if !global.is_silent() && global.is_verbose() {
        aprintln!("{}", p_m(&format!("DEBUG: {message}")));
    }
}
