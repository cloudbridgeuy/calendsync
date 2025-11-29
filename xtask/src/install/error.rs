use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    #[error("Binary not found: {0}")]
    BinaryNotFound(String),

    #[error("Failed to find vnt installation location")]
    InstallLocationNotFound,
}

pub type Result<T> = std::result::Result<T, Error>;
