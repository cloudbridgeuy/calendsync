use askama::Template;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

use crate::{models::User, state::AppState};

/// Template wrapper that converts Askama templates into HTML responses.
struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template: {err}"),
            )
                .into_response(),
        }
    }
}

/// Index page template showing the user table and form.
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    users: Vec<User>,
}

/// Handler for the index page (GET /).
pub async fn index(State(state): State<AppState>) -> impl IntoResponse {
    let users = state
        .users
        .read()
        .expect("Failed to acquire read lock")
        .values()
        .cloned()
        .collect();

    HtmlTemplate(IndexTemplate { users })
}
