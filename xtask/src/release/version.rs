use super::error::{Error, Result};
use crate::prelude::*;
use semver::Version;
use std::path::{Path, PathBuf};
use toml_edit::DocumentMut;

/// Get the project root directory
fn get_project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Parse and validate a version string
pub fn parse_version(version_str: &str) -> Result<Version> {
    let version = Version::parse(version_str)?;
    Ok(version)
}

/// Get current version from Cargo.toml
pub fn get_current_version() -> Result<String> {
    let project_root = get_project_root();

    // Try root Cargo.toml first (single crate)
    let root_cargo = project_root.join("Cargo.toml");
    if let Ok(version) = extract_version_from_toml(&root_cargo) {
        if !version.is_empty() && version != "0.0.0" {
            return Ok(version);
        }
    }

    // Try crates/vnt/Cargo.toml (workspace)
    let vnt_cargo = project_root.join("crates/vnt/Cargo.toml");
    if vnt_cargo.exists() {
        if let Ok(version) = extract_version_from_toml(&vnt_cargo) {
            return Ok(version);
        }
    }

    // Fallback
    Ok("0.0.0".to_string())
}

/// Extract version field from a Cargo.toml file
fn extract_version_from_toml(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path)?;
    let doc = content
        .parse::<DocumentMut>()
        .map_err(|e| Error::CargoTomlParse(format!("Failed to parse {}: {}", path.display(), e)))?;

    if let Some(package) = doc.get("package") {
        if let Some(version) = package.get("version") {
            if let Some(version_str) = version.as_str() {
                return Ok(version_str.to_string());
            }
        }
    }

    Err(Error::CargoTomlParse(format!(
        "No version field found in {}",
        path.display()
    )))
}

/// Update version in Cargo.toml files
pub async fn update_version(new_version: &str, global: &crate::Global) -> Result<()> {
    info_log(
        global,
        &format!("Updating version to {new_version} in Cargo.toml files..."),
    );

    let project_root = get_project_root();

    // Update root Cargo.toml
    let root_cargo = project_root.join("Cargo.toml");
    update_version_in_toml(&root_cargo, new_version)?;

    // Update crates/vnt/Cargo.toml
    let vnt_cargo = project_root.join("crates/vnt/Cargo.toml");
    if vnt_cargo.exists() {
        update_version_in_toml(&vnt_cargo, new_version)?;
    }

    // Update Cargo.lock by running cargo check
    info_log(global, "Updating Cargo.lock...");

    let output = tokio::process::Command::new("cargo")
        .arg("check")
        .arg("--quiet")
        .current_dir(&project_root)
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::VersionUpdate(format!(
            "cargo check failed after version update: {stderr}"
        )));
    }

    success_log(global, "Version updated successfully");
    Ok(())
}

/// Update version field in a specific Cargo.toml file
fn update_version_in_toml(path: &Path, new_version: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let mut doc = content
        .parse::<DocumentMut>()
        .map_err(|e| Error::CargoTomlParse(format!("Failed to parse {}: {}", path.display(), e)))?;

    // Update version in [package] section
    if let Some(package) = doc.get_mut("package") {
        if let Some(package_table) = package.as_table_mut() {
            package_table["version"] = toml_edit::value(new_version);
        }
    }

    // Write back to file
    std::fs::write(path, doc.to_string())?;

    Ok(())
}

/// Rollback version changes (used on failure)
pub async fn rollback_version(global: &crate::Global) -> Result<()> {
    warning_log(global, "Rolling back version changes...");

    let project_root = get_project_root();

    // Check if there are uncommitted changes
    let diff_output = execute_command("git", &["diff", "--cached", "--quiet"]).await;
    let diff_output2 = execute_command("git", &["diff", "--quiet"]).await;

    let has_changes = diff_output.is_err()
        || diff_output.map(|o| !o.status.success()).unwrap_or(false)
        || diff_output2.is_err()
        || diff_output2.map(|o| !o.status.success()).unwrap_or(false);

    if !has_changes {
        // If there are no changes, reset to previous commit
        let output = execute_command("git", &["reset", "--hard", "HEAD~1"]).await;
        if output.is_err() || !output.unwrap().status.success() {
            warning_log(global, "Could not rollback version commit");
        }
    } else {
        // If there are uncommitted changes, just reset the files
        let files = vec!["Cargo.toml", "crates/vnt/Cargo.toml", "Cargo.lock"];

        for file in files {
            let file_path = project_root.join(file);
            if file_path.exists() {
                let _ = execute_command("git", &["checkout", "HEAD", "--", file]).await;
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
