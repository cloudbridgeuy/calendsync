//! Root route handler with auth-based redirects.

use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
};

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use calendsync_auth::OptionalUser;

use crate::state::AppState;

/// Handler for GET /
///
/// - Unauthenticated: redirects to /login
/// - Authenticated: redirects to user's first calendar
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
pub async fn root_redirect(
    State(state): State<AppState>,
    OptionalUser(user): OptionalUser,
) -> Response {
    match user {
        None => Redirect::to("/login").into_response(),
        Some(user) => redirect_to_first_calendar(&state, user.id).await,
    }
}

/// Redirect an authenticated user to their first calendar.
///
/// If the user has no calendars, redirects back to /login (shouldn't happen but handle gracefully).
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
async fn redirect_to_first_calendar(state: &AppState, user_id: uuid::Uuid) -> Response {
    let auth = match &state.auth {
        Some(auth) => auth,
        None => {
            tracing::error!("Auth state not initialized");
            return Redirect::to("/login").into_response();
        }
    };

    match auth.memberships.get_calendars_for_user(user_id).await {
        Ok(calendars) if !calendars.is_empty() => {
            let first_calendar_id = calendars[0].0.id;
            Redirect::to(&format!("/calendar/{first_calendar_id}")).into_response()
        }
        Ok(_) => {
            // No calendars found - redirect back to login
            tracing::warn!(user_id = %user_id, "User has no calendars");
            Redirect::to("/login").into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get user calendars");
            Redirect::to("/login").into_response()
        }
    }
}
