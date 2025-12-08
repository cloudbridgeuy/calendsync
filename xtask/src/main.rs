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
mod install;
mod lint;
mod prelude;
mod release;

/// Development tasks for the vntana-devops-cli repository
#[derive(Debug, Parser)]
#[command(name = "xtask")]
#[command(about = "Development tasks for vntana-devops-cli", long_about = None)]
struct Cli {
    #[command(flatten)]
    global: Global,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, clap::Args)]
pub struct Global {
    /// VNTANA environment
    #[clap(
        short = 'e',
        long,
        env = "VNTANA_ENVIRONMENT",
        global = true,
        value_enum,
        value_name = "ENV"
    )]
    pub environment: Option<Environment>,

    /// Silence the command output
    #[clap(long, global = true)]
    pub silent: bool,

    /// Enable verbose output
    #[clap(long, global = true)]
    pub verbose: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum Environment {
    Development,
    Acceptance,
    Staging,
    Production,
    Accenture,
    Sony,
    SonyStaging,
    PrdPtc,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Acceptance => write!(f, "acceptance"),
            Environment::Staging => write!(f, "staging"),
            Environment::Production => write!(f, "production"),
            Environment::Accenture => write!(f, "accenture"),
            Environment::Sony => write!(f, "sony"),
            Environment::SonyStaging => write!(f, "sony-staging"),
            Environment::PrdPtc => write!(f, "prd-ptc"),
        }
    }
}

impl Global {
    pub fn is_silent(&self) -> bool {
        self.silent
    }

    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    pub fn get_environment(&self) -> Option<Environment> {
        self.environment
    }
}

/// Get the cluster name for a given environment
/// Extracts the cluster name from the GKE context pattern
pub fn get_cluster_name_for_environment(env: Environment) -> &'static str {
    match env {
        Environment::Development => "development-vntana",
        Environment::Acceptance => "acceptance-vntana",
        Environment::Staging => "staging-vntana",
        Environment::Production => "production-vntana",
        Environment::Accenture => "accenture-vntana",
        Environment::Sony => "sony-vntana",
        Environment::SonyStaging => "sony-staging-vntana",
        Environment::PrdPtc => "prd-ptc-vntana",
    }
}

/// Get the full GKE context name for a given environment
pub fn get_context_name_for_environment(env: Environment) -> &'static str {
    match env {
        Environment::Development => {
            "gke_vntana-platform-2-development_europe-west4_development-vntana"
        }
        Environment::Acceptance => "gke_vntana-platform-2-acceptance_us-central1_acceptance-vntana",
        Environment::Staging => "gke_vntana-platform-2-staging_us-central1_staging-vntana",
        Environment::Production => "gke_vntana-platform-2-production_us-central1_production-vntana",
        Environment::Accenture => "gke_vntana-platform-2-accenture_us-central1_accenture-vntana",
        Environment::Sony => "gke_vntana-platform-2-sony_us-central1_sony-vntana",
        Environment::SonyStaging => {
            "gke_vntana-platform-2-sony-staging_us-central1_sony-staging-vntana"
        }
        Environment::PrdPtc => "gke_vntana-platform-2-prd-ptc_us-central1_prd-ptc-vntana",
    }
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    /// Run the application in development mode
    Dev(dev::DevCommand),

    /// Install locally built vnt binary
    Install(install::InstallCommand),

    /// Create and manage releases
    Release(release::ReleaseCommand),

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
        Commands::Install(install_cmd) => {
            install::run(install_cmd, cli.global).await?;
        }
        Commands::Release(release_cmd) => {
            release::run(release_cmd, cli.global).await?;
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
        Commands::Docs { .. } => {
            println!("Documentation automation coming soon!");
            println!("This will generate and validate documentation.");
        }
    }

    Ok(())
}
