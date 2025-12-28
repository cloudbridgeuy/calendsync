//! See <https://github.com/matklad/cargo-xtask/>
//!
//! This binary defines various auxiliary build commands, which are not
//! expressible with just `cargo`.
//!
//! The binary is integrated into the `cargo` command line by using an
//! alias in `.cargo/config`.

use clap::Parser;

mod dev;
mod dynamodb;
mod integration;
mod lint;
mod prelude;

/// Development tasks for the calendsync repository
#[derive(Debug, Parser)]
#[command(name = "xtask")]
#[command(about = "Development tasks for calendsync", long_about = None)]
struct Cli {
    #[command(flatten)]
    global: Global,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, clap::Args)]
pub struct Global {
    /// Silence the command output
    #[clap(long, global = true)]
    pub silent: bool,

    /// Enable verbose output
    #[clap(long, global = true)]
    pub verbose: bool,
}

impl Global {
    pub fn is_silent(&self) -> bool {
        self.silent
    }

    pub fn is_verbose(&self) -> bool {
        self.verbose
    }
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    /// Run the application in development mode
    Dev(dev::DevCommand),

    /// Run tests and benchmarks (coming soon)
    #[command(hide = true)]
    Test {
        /// Run benchmarks
        #[arg(long)]
        bench: bool,
    },

    /// Code quality checks and git hooks management
    Lint(lint::LintCommand),

    /// Manage DynamoDB infrastructure
    Dynamodb(dynamodb::DynamodbCommand),

    /// Run integration tests
    Integration(integration::IntegrationCommand),

    /// Documentation tasks (coming soon)
    #[command(hide = true)]
    Docs {
        /// Open documentation in browser
        #[arg(long)]
        open: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dev(dev_cmd) => {
            dev::run(dev_cmd, cli.global).await?;
        }
        Commands::Test { .. } => {
            println!("Test automation coming soon!");
            println!("This will run test suites and benchmarks.");
        }
        Commands::Lint(lint_cmd) => {
            lint::run(lint_cmd, cli.global).await?;
        }
        Commands::Dynamodb(dynamodb_cmd) => {
            dynamodb::run(dynamodb_cmd, cli.global).await?;
        }
        Commands::Integration(integration_cmd) => {
            integration::run(integration_cmd, cli.global).await?;
        }
        Commands::Docs { .. } => {
            println!("Documentation automation coming soon!");
            println!("This will generate and validate documentation.");
        }
    }

    Ok(())
}
