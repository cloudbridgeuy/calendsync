use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Application error type that wraps `anyhow::Error`.
///
/// This allows using `?` on functions that return `Result<_, anyhow::Error>`
/// to automatically convert them into `Result<_, AppError>`.
pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!(error = %self.0, "Application error");

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
