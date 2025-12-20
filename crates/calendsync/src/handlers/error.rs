use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use calendsync_core::storage::{repository_error_to_status_code, RepositoryError};

pub struct AppError(pub anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status_code = if let Some(repo_error) = self.0.downcast_ref::<RepositoryError>() {
            let code = repository_error_to_status_code(repo_error);
            StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };

        (status_code, self.0.to_string()).into_response()
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
