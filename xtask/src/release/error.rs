#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Generic release error: {0}")]
    Generic(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("GitHub CLI error: {0}")]
    GitHubCli(String),

    #[error("GitHub API error: {0}")]
    GitHubApi(String),

    #[error("Not on main branch: {0}")]
    NotOnMainBranch(String),

    #[error("Working directory is not clean")]
    DirtyWorkingDirectory,

    #[error("CI checks failed: {0}")]
    CiChecksFailed(String),

    #[error("Workflow failed: {0}")]
    WorkflowFailed(String),

    #[error("Workflow timeout after {0} seconds")]
    WorkflowTimeout(u64),

    #[error("Command not found: {command}\n{help}")]
    CommandNotFound { command: String, help: String },

    #[error("Cargo.toml parse error: {0}")]
    CargoTomlParse(String),

    #[error("Failed to update version: {0}")]
    VersionUpdate(String),

    #[error("Release cancelled by user")]
    UserCancelled,

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Semver parse error: {0}")]
    SemverParse(#[from] semver::Error),

    #[error("TOML edit error: {0}")]
    TomlEdit(#[from] toml_edit::TomlError),
}

/// Result type alias for release module
pub type Result<T> = std::result::Result<T, Error>;

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Generic(s)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Generic(s.to_string())
    }
}

/// Helper function to check if a command exists in PATH
pub fn require_command(command: &str, help: &str) -> Result<()> {
    if !crate::prelude::command_exists(command) {
        return Err(Error::CommandNotFound {
            command: command.to_string(),
            help: help.to_string(),
        });
    }
    Ok(())
}
