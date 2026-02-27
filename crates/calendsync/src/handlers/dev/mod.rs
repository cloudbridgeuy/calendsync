//! Development-only handlers for hot-reload and UI annotations.
//!
//! Only available when `DEV_MODE` environment variable is set.

#[cfg(feature = "dev-annotations")]
pub mod annotations;
pub mod hot_reload;
#[cfg(feature = "dev-annotations")]
pub mod sessions;
pub mod types;

/// Extract the dev store from state, returning 500 if not initialized.
///
/// Shared by annotation and session handlers.
#[cfg(feature = "dev-annotations")]
pub(crate) fn get_store(
    state: &crate::state::AppState,
) -> Result<&std::sync::Arc<crate::dev::DevAnnotationStore>, crate::handlers::AppError> {
    state.dev_store.as_ref().ok_or_else(|| {
        crate::handlers::AppError(anyhow::anyhow!("dev annotation store not initialized"))
    })
}
