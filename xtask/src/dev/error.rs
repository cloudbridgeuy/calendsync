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

    #[error("Container runtime not found: {0}")]
    ContainerRuntimeNotFound(String),

    #[error("Container '{name}' not healthy after {timeout_secs}s")]
    ContainerNotHealthy { name: String, timeout_secs: u64 },

    #[error("Container start failed: {0}")]
    ContainerStartFailed(String),

    #[error("Seeding failed: {0}")]
    SeedingFailed(String),

    #[error("Server not healthy after {timeout_secs}s")]
    ServerNotHealthy { timeout_secs: u64 },
}

pub type Result<T> = std::result::Result<T, DevError>;
