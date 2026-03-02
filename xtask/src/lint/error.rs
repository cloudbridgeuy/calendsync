use thiserror::Error;

pub type Result<T> = std::result::Result<T, LintError>;

#[derive(Error, Debug)]
pub enum LintError {
    #[error("Code quality checks failed")]
    ChecksFailed,

    #[error("Git hooks installation failed: {0}")]
    HooksInstallFailed(String),

    #[error("Not a git repository")]
    NotGitRepository,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
