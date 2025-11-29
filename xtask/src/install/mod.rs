use crate::prelude::*;
use error::Result;
use std::path::PathBuf;

pub mod error;

/// Install the locally built vnt binary to replace the existing one
#[derive(Debug, clap::Parser)]
#[command(
    long_about = "Build vnt in release mode with git commit hash and install it to replace the existing binary.

This command will:
1. Build vnt in release mode with the current git commit embedded
2. Find the location of the currently installed vnt binary
3. Replace it with the newly built binary

The installed binary will show the git commit hash when running `vnt --version`.

This is useful for:
- Testing local changes before creating an official release
- Creating development builds from specific commits
- Quick iteration during development"
)]
pub struct InstallCommand {
    /// Preview actions without executing them
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(command: InstallCommand, global: crate::Global) -> Result<()> {
    if !global.is_silent() {
        aprintln!("{}", p_b("Building vnt in release mode..."));
    }

    // Build the binary in release mode
    let build_status =
        execute_command_interactive("cargo", &["build", "--release", "-p", "vnt"]).await?;

    if !build_status.success() {
        return Err(error::Error::CommandFailed(
            "Failed to build vnt binary".to_string(),
        ));
    }

    if !global.is_silent() {
        aprintln!("{}", p_g("✓ Build successful"));
    }

    // Find the built binary
    let built_binary = PathBuf::from("target/release/vnt");
    if !built_binary.exists() {
        return Err(error::Error::BinaryNotFound(
            "Built binary not found at target/release/vnt".to_string(),
        ));
    }

    // Find the installed vnt location
    let which_output = execute_command("which", &["vnt"]).await?;

    if !which_output.status.success() {
        return Err(error::Error::InstallLocationNotFound);
    }

    let install_location = String::from_utf8_lossy(&which_output.stdout)
        .trim()
        .to_string();

    if install_location.is_empty() {
        return Err(error::Error::InstallLocationNotFound);
    }

    if !global.is_silent() {
        aprintln!(
            "{}",
            p_b(&format!("Found vnt installed at: {install_location}"))
        );
    }

    if command.dry_run {
        aprintln!(
            "{}",
            p_y(&format!(
                "[DRY RUN] Would replace {} with {}",
                install_location,
                built_binary.display()
            ))
        );
        return Ok(());
    }

    // Replace the binary
    if !global.is_silent() {
        aprintln!("{}", p_b("Replacing binary..."));
    }

    // Copy the binary to the install location
    std::fs::copy(&built_binary, &install_location)?;

    // Make sure it's executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&install_location)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&install_location, perms)?;
    }

    if !global.is_silent() {
        aprintln!("{}", p_g("✓ Installation complete!"));
        aprintln!("");
        aprintln!("{}", p_c("Run 'vnt --version' to see the commit hash"));
    }

    Ok(())
}
