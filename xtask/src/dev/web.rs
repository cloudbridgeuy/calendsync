use super::error::Result;
use crate::prelude::*;

#[derive(Debug, clap::Args)]
pub struct WebOptions {
    /// Port to run the server on
    #[arg(long, short = 'p', default_value = "3000")]
    pub port: u16,

    /// Build in release mode
    #[arg(long)]
    pub release: bool,
}

pub async fn run(opts: WebOptions, global: crate::Global) -> Result<()> {
    if !global.is_silent() {
        aprintln!("{} Starting web server on port {}...", p_b("🌐"), opts.port);
    }

    let mut args = vec!["run", "-p", "calendsync"];

    if opts.release {
        args.push("--release");
    }

    // Set PORT environment variable
    std::env::set_var("PORT", opts.port.to_string());

    execute_command_interactive("cargo", &args).await?;
    Ok(())
}
