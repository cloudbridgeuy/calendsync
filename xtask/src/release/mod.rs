mod error;
mod git;
mod github;
mod validation;
mod version;
mod workflow;

pub use error::{Error, Result};

use crate::prelude::*;
use dialoguer::Confirm;

const GITHUB_REPO: &str = "VNTANA-3D/vntana-devops-cli";

/// Release command and subcommands
#[derive(Debug, clap::Parser)]
pub struct ReleaseCommand {
    #[command(subcommand)]
    pub action: ReleaseAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum ReleaseAction {
    /// Create a new release with the specified version
    Create {
        /// Version to release (e.g., 1.0.0, 2.1.0-beta.1)
        version: String,

        /// Automatically upgrade local vnt binary after successful release
        #[arg(long)]
        auto_upgrade: bool,

        /// Skip workflow monitoring
        #[arg(long)]
        no_monitor: bool,
    },

    /// Clean up a failed release tag
    Cleanup {
        /// Tag to clean up (e.g., v1.0.0)
        tag: String,
    },
}

/// Main entry point for release command
pub async fn run(command: ReleaseCommand, global: crate::Global) -> Result<()> {
    match command.action {
        ReleaseAction::Create {
            version,
            auto_upgrade,
            no_monitor,
        } => create_release(&version, auto_upgrade, !no_monitor, &global).await,
        ReleaseAction::Cleanup { tag } => cleanup_command(&tag, &global).await,
    }
}

/// Create a new release
async fn create_release(
    new_version: &str,
    auto_upgrade: bool,
    monitor: bool,
    global: &crate::Global,
) -> Result<()> {
    info_log(
        global,
        &format!("Starting release process for version {new_version}..."),
    );
    println!();

    // ========== Pre-release Validation ==========
    step_log(global, "Running pre-release checks...");

    validation::check_gh_cli(global).await?;
    validation::check_main_branch(global).await?;
    validation::check_clean_working_dir(global).await?;
    validation::check_commit_status(global).await?;

    // Validate version format
    version::parse_version(new_version)?;
    info_log(global, &format!("Version format is valid: {new_version} ✓"));

    // Get current version
    let current_version = version::get_current_version()?;
    info_log(global, &format!("Current version: {current_version}"));
    info_log(global, &format!("New version: {new_version}"));

    // ========== Show Release Plan ==========
    println!();
    step_log(global, "Release Plan:");
    println!("  1. Verify all CI checks are passing ✓");
    println!("  2. Update version in Cargo.toml files");
    println!("  3. Create git commit and tag v{new_version}");
    println!("  4. Push changes and tag to GitHub");
    if monitor {
        println!("  5. Monitor GitHub Actions workflow");
        println!("  6. Verify release creation");
        println!("  7. If workflow fails: cleanup and offer retry");
    } else {
        println!("  5. Skip workflow monitoring (--no-monitor)");
    }
    if auto_upgrade && github::is_vnt_installed() {
        println!("  8. Automatically upgrade local vnt binary");
    }
    println!();

    // ========== User Confirmation ==========
    if !ask_confirmation(
        &format!("Are you sure you want to release version {new_version}?"),
        false,
    )? {
        aprintln!("{}", p_b("Release cancelled."));
        return Ok(());
    }

    // ========== Perform Release ==========
    version::update_version(new_version, global).await?;

    if monitor {
        // With retry mechanism
        workflow::retry_release(new_version, 3, global).await?;
    } else {
        // Without monitoring - just create and push
        git::create_version_commit(new_version, global).await?;
        git::create_tag(new_version, global).await?;
        git::push_changes(new_version, global).await?;
    }

    // ========== Success! ==========
    println!();
    success_log(
        global,
        &format!("🎉 Release {new_version} completed successfully!"),
    );
    info_log(
        global,
        &format!(
            "📦 Release available at: https://github.com/{GITHUB_REPO}/releases/tag/v{new_version}"
        ),
    );
    info_log(
        global,
        "📋 Installation instructions are included in the release notes.",
    );

    // ========== Optional Upgrade ==========
    if github::is_vnt_installed() {
        let should_upgrade = if auto_upgrade {
            true
        } else {
            println!();
            ask_confirmation(
                "Would you like to upgrade your local vnt binary to the new version?",
                false,
            )?
        };

        if should_upgrade {
            step_log(global, "Testing the upgrade command...");
            info_log(
                global,
                "Verifying the release by upgrading local vnt binary (you may be prompted for your password)",
            );

            match github::run_vnt_upgrade(global).await {
                Ok(()) => {
                    success_log(global, "✅ Upgrade command executed successfully!");
                    info_log(
                        global,
                        &format!("Your vnt binary has been updated to version {new_version}"),
                    );
                }
                Err(e) => {
                    warning_log(global, &format!("⚠️  Upgrade command failed: {e}"));
                    info_log(
                        global,
                        "You can still download the release manually from GitHub",
                    );
                }
            }
        } else {
            info_log(
                global,
                "💡 You can upgrade later by running: sudo vnt upgrade",
            );
        }
    } else {
        info_log(
            global,
            "💡 Tip: Install vnt to test releases with 'sudo vnt upgrade'",
        );
    }

    Ok(())
}

/// Clean up a failed release
async fn cleanup_command(tag: &str, global: &crate::Global) -> Result<()> {
    step_log(global, &format!("Cleaning up failed release: {tag}"));

    // Validate tag format
    if !tag.starts_with('v') || tag.len() < 2 {
        return Err(Error::Generic(format!(
            "Invalid tag format: {tag}. Expected format: v1.0.0"
        )));
    }

    // Extract version from tag
    let version = &tag[1..];

    // Validate as semver
    version::parse_version(version)?;

    // Confirm with user
    println!();
    warning_log(global, "This will:");
    println!("  • Remove local tag: {tag}");
    println!("  • Remove remote tag: {tag} (if exists)");
    println!("  • Rollback version changes (if any)");
    println!();

    if !ask_confirmation(&format!("Are you sure you want to cleanup {tag}?"), false)? {
        aprintln!("{}", p_b("Cleanup cancelled."));
        return Ok(());
    }

    workflow::cleanup_after_failure(version, global).await?;
    success_log(global, &format!("✅ Cleanup completed for {tag}"));

    Ok(())
}

/// Helper function to ask for user confirmation
fn ask_confirmation(message: &str, default: bool) -> Result<bool> {
    Confirm::new()
        .with_prompt(message)
        .default(default)
        .interact()
        .map_err(|e| Error::Generic(format!("Failed to get user input: {e}")))
}

// Helper logging functions
fn step_log(global: &crate::Global, message: &str) {
    if !global.is_silent() {
        aprintln!("{} {}", p_c("==>"), p_c(message));
    }
}

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
