//! Axum extractor for RequestContext.

use std::convert::Infallible;

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap},
};
use uuid::Uuid;

use super::types::{RequestContext, RequestId};

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use axum::extract::FromRef;
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use axum_extra::extract::CookieJar;
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
use calendsync_auth::{AuthState, OptionalUser};

fn extract_request_id(headers: &HeaderMap) -> RequestId {
    headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(RequestId::from_uuid)
        .unwrap_or_else(RequestId::new)
}

#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
impl<S> FromRequestParts<S> for RequestContext
where
    AuthState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let OptionalUser(user) = OptionalUser::from_request_parts(parts, state)
            .await
            .unwrap_or(OptionalUser(None));

        let request_id = extract_request_id(&parts.headers);

        // Extract session ID from cookie for dev tools
        let auth_state = AuthState::from_ref(state);
        let jar = CookieJar::from_headers(&parts.headers);
        let session_id = jar
            .get(&auth_state.config.cookie_name)
            .map(|cookie| cookie.value().to_string());

        Ok(RequestContext {
            user,
            request_id,
            session_id,
        })
    }
}

#[cfg(not(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock")))]
impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let request_id = extract_request_id(&parts.headers);
        Ok(RequestContext {
            user: None,
            request_id,
            session_id: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_request_id_from_header() {
        let mut headers = HeaderMap::new();
        let id = "550e8400-e29b-41d4-a716-446655440000";
        headers.insert("x-request-id", id.parse().unwrap());

        let request_id = extract_request_id(&headers);
        assert_eq!(request_id.to_string(), id);
    }

    #[test]
    fn test_extract_request_id_generates_when_missing() {
        let headers = HeaderMap::new();
        let request_id = extract_request_id(&headers);

        Uuid::parse_str(&request_id.to_string()).expect("Should be valid UUID");
    }

    #[test]
    fn test_extract_request_id_generates_when_invalid() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", "not-a-uuid".parse().unwrap());

        let request_id = extract_request_id(&headers);

        Uuid::parse_str(&request_id.to_string()).expect("Should be valid UUID");
    }
}
