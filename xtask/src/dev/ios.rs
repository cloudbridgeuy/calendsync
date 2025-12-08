use super::error::Result;
use crate::prelude::*;

#[derive(Debug, clap::Args)]
pub struct IosOptions {
    /// iOS simulator or device name (e.g., "iPhone 16 Pro")
    #[arg(long, short = 'd')]
    pub device: Option<String>,

    /// List available iOS simulators
    #[arg(long, conflicts_with_all = &["device", "open"])]
    pub list_devices: bool,

    /// Open Xcode instead of running directly
    #[arg(long, short = 'o')]
    pub open: bool,

    /// Use public network address for physical devices
    #[arg(long)]
    pub host: Option<String>,

    /// Build in release mode
    #[arg(long)]
    pub release: bool,

    /// Disable file watching
    #[arg(long)]
    pub no_watch: bool,
}

pub async fn run(opts: IosOptions, global: crate::Global) -> Result<()> {
    if opts.list_devices {
        return list_devices(&global).await;
    }

    if !global.is_silent() {
        if let Some(ref device) = opts.device {
            aprintln!("{} Starting iOS app on {}...", p_b("📱"), device);
        } else {
            aprintln!("{} Starting iOS app...", p_b("📱"));
        }
    }

    // Build command arguments
    let mut args = vec!["tauri", "ios", "dev"];

    // Device is a positional argument in cargo tauri ios dev
    if let Some(ref device) = opts.device {
        args.push(device.as_str());
    }

    if opts.open {
        args.push("--open");
    }

    if opts.release {
        args.push("--release");
    }

    if opts.no_watch {
        args.push("--no-watch");
    }

    if let Some(ref host) = opts.host {
        args.push("--host");
        args.push(host.as_str());
    }

    // Run cargo tauri ios dev
    execute_command_interactive("cargo", &args).await?;
    Ok(())
}

async fn list_devices(global: &crate::Global) -> Result<()> {
    if !global.is_silent() {
        aprintln!("{}", p_b("Available iOS Simulators:"));
        aprintln!();
    }

    let output = tokio::process::Command::new("xcrun")
        .args(["simctl", "list", "devices", "available"])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse and display devices nicely
    for line in stdout.lines() {
        if line.contains("--") {
            // iOS version header
            aprintln!("{}", p_b(line.trim()));
        } else if line.contains('(') && !line.trim().starts_with("==") {
            // Device line (has parentheses for UUID)
            let trimmed = line.trim();
            if trimmed.contains("(Booted)") {
                aprintln!("  {} {}", p_g("●"), trimmed);
            } else {
                aprintln!("  {} {}", p_y("○"), trimmed);
            }
        }
    }

    aprintln!();
    aprintln!(
        "{}",
        p_c("Usage: cargo xtask dev ios --device \"iPhone 16 Pro\"")
    );

    Ok(())
}
