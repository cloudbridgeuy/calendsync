use super::error::{Error, Result};
use super::{git, github, version};
use crate::prelude::*;
use std::time::Duration;

const GITHUB_REPO: &str = "VNTANA-3D/vntana-devops-cli";
const WORKFLOW_FILE: &str = "release.yml";
const WORKFLOW_CHECK_INTERVAL: u64 = 30; // seconds
const WORKFLOW_TIMEOUT: u64 = 1800; // 30 minutes

/// Wait for GitHub Actions workflow to complete
pub async fn wait_for_workflow(version: &str, global: &crate::Global) -> Result<()> {
    let tag = format!("v{version}");
    let start_time = std::time::Instant::now();

    info_log(
        global,
        &format!("Monitoring GitHub Actions workflow for tag: {tag}"),
    );
    info_log(
        global,
        &format!(
            "Workflow timeout: {}s ({} minutes)",
            WORKFLOW_TIMEOUT,
            WORKFLOW_TIMEOUT / 60
        ),
    );

    // Wait a bit for the workflow to start
    info_log(global, "Waiting for workflow to start...");
    tokio::time::sleep(Duration::from_secs(10)).await;

    loop {
        let elapsed = start_time.elapsed().as_secs();

        // Check timeout
        if elapsed > WORKFLOW_TIMEOUT {
            return Err(Error::WorkflowTimeout(WORKFLOW_TIMEOUT));
        }

        // Get workflow status
        let workflow_status = github::get_workflow_runs(&tag, WORKFLOW_FILE, global).await?;

        if let Some(status) = workflow_status {
            match status.status.as_str() {
                "completed" => {
                    match status.conclusion.as_deref() {
                        Some("success") => {
                            println!(); // Clear progress line
                            success_log(
                                global,
                                "✅ GitHub Actions workflow completed successfully!",
                            );
                            info_log(
                                global,
                                &format!(
                                    "Release should be available at: https://github.com/{GITHUB_REPO}/releases/tag/{tag}"
                                ),
                            );
                            return Ok(());
                        }
                        Some(conclusion @ ("failure" | "cancelled" | "timed_out")) => {
                            println!(); // Clear progress line
                            return Err(Error::WorkflowFailed(format!(
                                "Workflow failed with conclusion: {conclusion}. Check logs at: https://github.com/{GITHUB_REPO}/actions"
                            )));
                        }
                        Some(other) => {
                            println!(); // Clear progress line
                            return Err(Error::WorkflowFailed(format!(
                                "Workflow completed with unknown conclusion: {other}"
                            )));
                        }
                        None => {
                            println!(); // Clear progress line
                            return Err(Error::WorkflowFailed(
                                "Workflow completed but no conclusion available".to_string(),
                            ));
                        }
                    }
                }
                status @ ("in_progress" | "queued" | "requested" | "waiting" | "pending") => {
                    // Show progress with carriage return (like bash script)
                    if !global.is_silent() {
                        print!(
                            "\r{} ⏳ Workflow status: {} ({}s elapsed)",
                            p_b("INFO:"),
                            status,
                            elapsed
                        );
                        use std::io::Write;
                        std::io::stdout().flush().ok();
                    }
                }
                other => {
                    warning_log(global, &format!("Unknown workflow status: {other}"));
                }
            }
        } else {
            // No workflow found yet
            if !global.is_silent() {
                print!(
                    "\r{} 🔍 Looking for workflow... ({}s elapsed)",
                    p_b("INFO:"),
                    elapsed
                );
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }

        tokio::time::sleep(Duration::from_secs(WORKFLOW_CHECK_INTERVAL)).await;
    }
}

/// Cleanup after a failed release
pub async fn cleanup_after_failure(version: &str, global: &crate::Global) -> Result<()> {
    aprintln!("{}", p_r("Release process failed. Starting cleanup..."));

    // Cleanup tags
    git::cleanup_tag(version, true, global).await?;

    // Rollback version changes
    version::rollback_version(global).await?;

    warning_log(
        global,
        "Cleanup completed. You can now fix the issues and try again.",
    );

    Ok(())
}

/// Retry release with cleanup between attempts
pub async fn retry_release(version: &str, max_retries: u32, global: &crate::Global) -> Result<()> {
    let mut retry_count = 0;

    while retry_count < max_retries {
        if retry_count > 0 {
            warning_log(
                global,
                &format!("Retry attempt {retry_count} of {max_retries}"),
            );
            println!();

            // Ask user if they want to retry
            if !ask_user_confirmation("Do you want to retry the release?", false)? {
                aprintln!("{}", p_b("Release cancelled by user."));
                return Err(Error::UserCancelled);
            }
        }

        // Attempt release
        match create_and_push_tag(version, true, global).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                retry_count += 1;
                aprintln!(
                    "{}",
                    p_r(&format!("Release attempt {retry_count} failed: {e}"))
                );

                if retry_count < max_retries {
                    cleanup_after_failure(version, global).await?;
                    info_log(global, "Cleaned up failed release. Ready for retry.");
                    println!();
                }
            }
        }
    }

    aprintln!(
        "{}",
        p_r(&format!("All {max_retries} release attempts failed."))
    );
    cleanup_after_failure(version, global).await?;

    Err(Error::Generic(format!(
        "Release failed after {max_retries} attempts"
    )))
}

/// Create and push tag with optional workflow monitoring
async fn create_and_push_tag(version: &str, monitor: bool, global: &crate::Global) -> Result<()> {
    // Clean up any existing tag first
    git::cleanup_tag(version, true, global).await?;

    // Create commit and tag
    git::create_version_commit(version, global).await?;
    git::create_tag(version, global).await?;

    // Push changes
    git::push_changes(version, global).await?;

    if monitor {
        info_log(global, "Monitoring GitHub Actions workflow...");
        info_log(
            global,
            &format!("You can also monitor at: https://github.com/{GITHUB_REPO}/actions"),
        );

        wait_for_workflow(version, global).await?;

        println!();
        success_log(
            global,
            &format!("🎉 Release {version} completed successfully!"),
        );
    } else {
        info_log(
            global,
            &format!(
                "Skipping workflow monitoring. Check status at: https://github.com/{GITHUB_REPO}/actions"
            ),
        );
    }

    Ok(())
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

// Helper logging functions
fn info_log(global: &crate::Global, message: &str) {
    if !global.is_silent() {
        aprintln!("{}", p_b(message));
    }
}

fn success_log(global: &crate::Global, message: &str) {
    if !global.is_silent() {
        aprintln!("{}", p_g(message));
    }
}

fn warning_log(global: &crate::Global, message: &str) {
    if !global.is_silent() {
        aprintln!("{}", p_y(message));
    }
}
