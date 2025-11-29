use super::error::{Error, Result};
use crate::prelude::*;

/// Create a git commit for version changes
pub async fn create_version_commit(version: &str, global: &crate::Global) -> Result<()> {
    info_log(
        global,
        &format!("Creating git commit for version {version}..."),
    );

    // Stage files
    let files = vec!["Cargo.toml", "crates/vnt/Cargo.toml", "Cargo.lock"];

    for file in files {
        let output = execute_command("git", &["add", file]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug_log(global, &format!("Failed to stage {file}: {stderr}"));
        }
    }

    // Create commit
    let commit_message = format!("chore: bump version to {version}");
    let output = execute_command("git", &["commit", "-m", &commit_message]).await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Git(format!("Failed to create commit: {stderr}")));
    }

    success_log(global, "Version commit created");
    Ok(())
}

/// Create an annotated git tag
pub async fn create_tag(version: &str, global: &crate::Global) -> Result<()> {
    let tag = format!("v{version}");
    info_log(global, &format!("Creating git tag {tag}..."));

    let tag_message = format!("Release {version}");
    let output = execute_command("git", &["tag", "-a", &tag, "-m", &tag_message]).await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Git(format!("Failed to create tag: {stderr}")));
    }

    success_log(global, &format!("Tag {tag} created"));
    Ok(())
}

/// Push changes and tag to origin
pub async fn push_changes(version: &str, global: &crate::Global) -> Result<()> {
    let tag = format!("v{version}");
    info_log(global, "Pushing changes and tag to origin...");

    // Push main branch
    let output = execute_command("git", &["push", "origin", "main"]).await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Git(format!("Failed to push to main: {stderr}")));
    }

    // Push tag
    let output = execute_command("git", &["push", "origin", &tag]).await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Git(format!("Failed to push tag: {stderr}")));
    }

    success_log(
        global,
        &format!("✅ Tag {tag} created and pushed successfully!"),
    );
    Ok(())
}

/// Clean up local and remote tags
pub async fn cleanup_tag(
    version: &str,
    cleanup_remote: bool,
    global: &crate::Global,
) -> Result<()> {
    let tag = format!("v{version}");
    warning_log(global, &format!("Cleaning up tag: {tag}"));

    // Check if local tag exists
    let output = execute_command("git", &["tag", "-l"]).await?;
    let tag_list = String::from_utf8_lossy(&output.stdout);

    if tag_list.lines().any(|line| line.trim() == tag) {
        info_log(global, &format!("Removing local tag: {tag}"));
        let output = execute_command("git", &["tag", "-d", &tag]).await?;

        if !output.status.success() {
            warning_log(global, &format!("Failed to remove local tag: {tag}"));
        }
    }

    // Remove remote tag if requested
    if cleanup_remote {
        info_log(global, &format!("Checking if remote tag exists: {tag}"));

        let output = execute_command("git", &["ls-remote", "--tags", "origin"]).await?;

        if output.status.success() {
            let remote_tags = String::from_utf8_lossy(&output.stdout);
            let tag_ref = format!("refs/tags/{tag}");

            if remote_tags.lines().any(|line| line.contains(&tag_ref)) {
                info_log(global, &format!("Removing remote tag: {tag}"));

                let output = execute_command("git", &["push", "--delete", "origin", &tag]).await?;

                if !output.status.success() {
                    warning_log(global, &format!("Failed to remove remote tag: {tag}"));
                }
            }
        }
    }

    Ok(())
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

fn debug_log(global: &crate::Global, message: &str) {
    if !global.is_silent() && global.is_verbose() {
        aprintln!("{}", p_m(&format!("DEBUG: {message}")));
    }
}
