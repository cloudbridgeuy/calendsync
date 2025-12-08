use super::error::Result;
use crate::prelude::*;

#[derive(Debug, clap::Args)]
pub struct DesktopOptions {
    /// Build in release mode
    #[arg(long)]
    pub release: bool,

    /// Target triple to build against
    #[arg(long, short = 't')]
    pub target: Option<String>,

    /// Disable file watching
    #[arg(long)]
    pub no_watch: bool,
}

pub async fn run(opts: DesktopOptions, global: crate::Global) -> Result<()> {
    if !global.is_silent() {
        aprintln!("{} Starting desktop app...", p_b("🖥️"));
    }

    let mut args = vec!["tauri", "dev"];

    if opts.release {
        args.push("--release");
    }

    if let Some(ref target) = opts.target {
        args.push("--target");
        args.push(target.as_str());
    }

    if opts.no_watch {
        args.push("--no-watch");
    }

    execute_command_interactive("cargo", &args).await?;
    Ok(())
}
