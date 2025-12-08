use clap::Subcommand;

pub mod desktop;
pub mod error;
pub mod ios;
pub mod web;

use error::Result;

/// Run the application in development mode
#[derive(Debug, clap::Parser)]
#[command(long_about = "Run the application in development mode.

Supports three targets:
  web      - Run the Axum web server with hot-reload
  desktop  - Run the Tauri desktop app (macOS)
  ios      - Run the Tauri iOS app in simulator or device

Examples:
  cargo xtask dev web                        # Run web server on port 3000
  cargo xtask dev web --port 8080            # Run on custom port
  cargo xtask dev desktop                    # Run desktop app
  cargo xtask dev desktop --release          # Run in release mode
  cargo xtask dev ios                        # Run iOS simulator
  cargo xtask dev ios --device 'iPhone 16'   # Run on specific simulator
  cargo xtask dev ios --list-devices         # List available simulators
  cargo xtask dev ios --open                 # Open Xcode instead")]
pub struct DevCommand {
    #[command(subcommand)]
    pub target: DevTarget,
}

#[derive(Debug, Subcommand)]
pub enum DevTarget {
    /// Run the Axum web server
    Web(web::WebOptions),

    /// Run the Tauri desktop app
    Desktop(desktop::DesktopOptions),

    /// Run the Tauri iOS app
    Ios(ios::IosOptions),
}

pub async fn run(command: DevCommand, global: crate::Global) -> Result<()> {
    match command.target {
        DevTarget::Web(opts) => web::run(opts, global).await,
        DevTarget::Desktop(opts) => desktop::run(opts, global).await,
        DevTarget::Ios(opts) => ios::run(opts, global).await,
    }
}
