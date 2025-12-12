use thiserror::Error;

#[derive(Debug, Error)]
pub enum DevError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Frontend build failed:\n{0}")]
    BuildFailedWithOutput(String),

    #[error("Reload request failed: {0}")]
    ReloadFailed(String),

    #[error("File watcher error: {0}")]
    WatcherError(#[from] notify::Error),

    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, DevError>;
