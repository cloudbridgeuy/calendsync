//! Axum extractors for authentication.

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};
use axum_extra::extract::CookieJar;
use calendsync_core::auth::{is_session_expired, SessionId};
use calendsync_core::calendar::User;
use chrono::Utc;

use crate::AuthState;

/// Extractor for authenticated user. Returns 401 if not authenticated.
pub struct CurrentUser(pub User);

impl<S> FromRequestParts<S> for CurrentUser
where
    AuthState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_state = AuthState::from_ref(state);

        // Try Authorization header first (for API/mobile clients)
        let session_id = if let Some(auth_header) = parts.headers.get(AUTHORIZATION) {
            let header_value = auth_header
                .to_str()
                .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid authorization header"))?;

            header_value
                .strip_prefix("Bearer ")
                .map(|token| SessionId::new(token.to_string()))
        } else {
            None
        };

        // Fall back to cookie (for web clients)
        let session_id = match session_id {
            Some(id) => id,
            None => {
                let jar = CookieJar::from_headers(&parts.headers);
                let cookie = jar
                    .get(&auth_state.config.cookie_name)
                    .ok_or((StatusCode::UNAUTHORIZED, "No session cookie"))?;

                SessionId::new(cookie.value().to_string())
            }
        };

        // Look up session
        let session = auth_state
            .sessions
            .get_session(&session_id)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Session lookup failed"))?
            .ok_or((StatusCode::UNAUTHORIZED, "Session not found"))?;

        // Check expiry
        if is_session_expired(&session, Utc::now()) {
            return Err((StatusCode::UNAUTHORIZED, "Session expired"));
        }

        // Look up user - parse user_id as UUID
        let user_id: uuid::Uuid = session
            .user_id
            .parse()
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Invalid user ID"))?;

        let user = auth_state
            .users
            .get_user(user_id)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "User lookup failed"))?
            .ok_or((StatusCode::UNAUTHORIZED, "User not found"))?;

        Ok(CurrentUser(user))
    }
}

/// Extractor for optionally authenticated user. Returns None if not authenticated.
pub struct OptionalUser(pub Option<User>);

impl<S> FromRequestParts<S> for OptionalUser
where
    AuthState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_state = AuthState::from_ref(state);

        // Try Authorization header first (for API/mobile clients)
        let session_id = if let Some(auth_header) = parts.headers.get(AUTHORIZATION) {
            let header_value = match auth_header.to_str() {
                Ok(v) => v,
                Err(_) => return Ok(OptionalUser(None)),
            };

            header_value
                .strip_prefix("Bearer ")
                .map(|token| SessionId::new(token.to_string()))
        } else {
            None
        };

        // Fall back to cookie (for web clients)
        let session_id = match session_id {
            Some(id) => id,
            None => {
                let jar = CookieJar::from_headers(&parts.headers);
                match jar.get(&auth_state.config.cookie_name) {
                    Some(cookie) => SessionId::new(cookie.value().to_string()),
                    None => return Ok(OptionalUser(None)),
                }
            }
        };

        // Look up session
        let session = match auth_state.sessions.get_session(&session_id).await {
            Ok(Some(s)) => s,
            _ => return Ok(OptionalUser(None)),
        };

        // Check expiry
        if is_session_expired(&session, Utc::now()) {
            return Ok(OptionalUser(None));
        }

        // Look up user - parse user_id as UUID
        let user_id: uuid::Uuid = match session.user_id.parse() {
            Ok(id) => id,
            Err(_) => return Ok(OptionalUser(None)),
        };

        let user = match auth_state.users.get_user(user_id).await {
            Ok(Some(u)) => u,
            _ => return Ok(OptionalUser(None)),
        };

        Ok(OptionalUser(Some(user)))
    }
}
