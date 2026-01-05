//! HTTP handlers for auth routes.

use axum::{
    extract::{Query, State},
    response::Redirect,
    routing::{get, post},
    Form, Json, Router,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use calendsync_core::auth::{
    calculate_expiry, email_to_name, generate_session_id, generate_state, validate_return_to,
    AuthFlowState, OidcClaims, OidcProvider, Session,
};
use calendsync_core::calendar::{Calendar, CalendarMembership, User};
use chrono::{Duration, Utc};
use openidconnect::PkceCodeChallenge;
use serde::Deserialize;

use crate::error::AuthError;
use crate::extractors::CurrentUser;
use crate::AuthState;

/// Query parameters for OAuth callback.
#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

/// Apple sends callback as POST with form data.
#[derive(Deserialize)]
pub struct AppleCallbackForm {
    pub code: String,
    pub state: String,
    /// JSON string with name on first login.
    pub user: Option<String>,
}

/// Query parameters for login endpoints.
#[derive(Deserialize, Default)]
pub struct LoginQuery {
    /// URL to redirect to after successful authentication.
    pub return_to: Option<String>,
}

/// Creates the auth router with all authentication routes.
///
/// Routes:
/// - `GET /auth/google/login` - Initiate Google OIDC flow
/// - `GET /auth/google/callback` - Handle Google OIDC callback
/// - `GET /auth/apple/login` - Initiate Apple OIDC flow
/// - `POST /auth/apple/callback` - Handle Apple OIDC callback (form POST)
/// - `POST /auth/logout` - End current session
/// - `POST /auth/logout-all` - End all sessions for current user
/// - `GET /auth/me` - Get current authenticated user
pub fn auth_routes() -> Router<AuthState> {
    Router::new()
        .route("/auth/google/login", get(google_login))
        .route("/auth/google/callback", get(google_callback))
        .route("/auth/apple/login", get(apple_login))
        .route("/auth/apple/callback", post(apple_callback))
        .route("/auth/logout", post(logout))
        .route("/auth/logout-all", post(logout_all))
        .route("/auth/me", get(me))
}

async fn google_login(
    State(state): State<AuthState>,
    Query(query): Query<LoginQuery>,
) -> Result<Redirect, AuthError> {
    initiate_login(&state, OidcProvider::Google, query.return_to).await
}

async fn apple_login(
    State(state): State<AuthState>,
    Query(query): Query<LoginQuery>,
) -> Result<Redirect, AuthError> {
    initiate_login(&state, OidcProvider::Apple, query.return_to).await
}

async fn initiate_login(
    state: &AuthState,
    provider: OidcProvider,
    return_to: Option<String>,
) -> Result<Redirect, AuthError> {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let csrf_state = generate_state();

    // Validate return_to URL to prevent open redirect attacks
    let validated_return_to = return_to
        .as_deref()
        .and_then(validate_return_to)
        .map(String::from);

    // Store PKCE verifier for callback
    let flow = AuthFlowState {
        pkce_verifier: pkce_verifier.secret().to_string(),
        provider,
        created_at: Utc::now(),
        return_to: validated_return_to,
    };
    state.sessions.store_auth_flow(&csrf_state, &flow).await?;

    // Get provider client and generate auth URL
    let provider_client = state.get_provider(provider)?;
    let auth_url = provider_client
        .authorization_url(&csrf_state, pkce_challenge.as_str())
        .await?;

    Ok(Redirect::to(auth_url.as_str()))
}

async fn google_callback(
    State(state): State<AuthState>,
    Query(params): Query<CallbackQuery>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), AuthError> {
    handle_callback(&state, &params.code, &params.state, jar, None).await
}

async fn apple_callback(
    State(state): State<AuthState>,
    jar: CookieJar,
    Form(form): Form<AppleCallbackForm>,
) -> Result<(CookieJar, Redirect), AuthError> {
    // Apple includes user info in form on first login
    let user_info = form
        .user
        .as_ref()
        .and_then(|u| serde_json::from_str::<serde_json::Value>(u).ok());

    let name = user_info.as_ref().and_then(|u| {
        let first = u.get("name")?.get("firstName")?.as_str()?;
        let last = u.get("name")?.get("lastName")?.as_str()?;
        Some(format!("{} {}", first, last))
    });

    handle_callback(&state, &form.code, &form.state, jar, name).await
}

async fn handle_callback(
    state: &AuthState,
    code: &str,
    csrf_state: &str,
    jar: CookieJar,
    apple_name: Option<String>,
) -> Result<(CookieJar, Redirect), AuthError> {
    // Retrieve and validate PKCE verifier
    let flow = state
        .sessions
        .take_auth_flow(csrf_state)
        .await?
        .ok_or(AuthError::Core(
            calendsync_core::auth::AuthError::InvalidState,
        ))?;

    // Exchange code for claims
    let provider_client = state.get_provider(flow.provider)?;
    let mut claims = provider_client
        .exchange_code(code, &flow.pkce_verifier)
        .await?;

    // Apple: use name from form if available (only sent on first login)
    if flow.provider == OidcProvider::Apple {
        if let Some(name) = apple_name {
            claims.name = Some(name);
        }
    }

    // Find or create user
    let user = find_or_create_user(state, &claims).await?;

    // Create session
    let now = Utc::now();
    let session = Session {
        id: generate_session_id(),
        user_id: user.id.to_string(),
        provider: claims.provider,
        created_at: now,
        expires_at: calculate_expiry(
            now,
            Duration::seconds(state.config.session_ttl.as_secs() as i64),
        ),
    };
    state.sessions.create_session(&session).await?;

    // Set secure cookie - clone the cookie name to own it
    let cookie_name = state.config.cookie_name.clone();
    let session_value = session.id.to_string();
    let cookie = Cookie::build((cookie_name, session_value))
        .path("/")
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(
            state.config.session_ttl.as_secs() as i64
        ))
        .build();

    let jar = jar.add(cookie);

    // Redirect to return_to URL or default to root
    let redirect_url = flow.return_to.unwrap_or_else(|| "/".to_string());
    Ok((jar, Redirect::to(&redirect_url)))
}

async fn find_or_create_user(state: &AuthState, claims: &OidcClaims) -> Result<User, AuthError> {
    // Look up by provider + subject
    if let Some(user) = state
        .users
        .get_user_by_provider(&claims.provider.to_string(), &claims.subject)
        .await
        .map_err(|e| AuthError::Core(calendsync_core::auth::AuthError::Storage(e.to_string())))?
    {
        return Ok(user);
    }

    // Create new user
    let name = claims
        .name
        .clone()
        .or_else(|| claims.email.as_ref().map(|e| email_to_name(e)))
        .unwrap_or_else(|| "User".to_string());

    let email = claims.email.clone().unwrap_or_default();

    let user = User::new(&name, &email)
        .with_provider(claims.provider.to_string())
        .with_provider_subject(&claims.subject);

    state
        .users
        .create_user(&user)
        .await
        .map_err(|e| AuthError::Core(calendsync_core::auth::AuthError::Storage(e.to_string())))?;

    // Create default calendar with a default color
    let calendar = Calendar::new(format!("{}'s Calendar", name), "#3B82F6").as_default();

    state
        .calendars
        .create_calendar(&calendar)
        .await
        .map_err(|e| AuthError::Core(calendsync_core::auth::AuthError::Storage(e.to_string())))?;

    // Create ownership membership
    let membership = CalendarMembership::owner(calendar.id, user.id);

    state
        .memberships
        .create_membership(&membership)
        .await
        .map_err(|e| AuthError::Core(calendsync_core::auth::AuthError::Storage(e.to_string())))?;

    Ok(user)
}

async fn logout(
    State(state): State<AuthState>,
    CurrentUser(_user): CurrentUser,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    // Get session ID from cookie
    if let Some(cookie) = jar.get(&state.config.cookie_name) {
        let session_id = calendsync_core::auth::SessionId::new(cookie.value().to_string());
        state.sessions.delete_session(&session_id).await?;
    }

    // Remove cookie
    let jar = jar.remove(Cookie::from(state.config.cookie_name.clone()));
    Ok(jar)
}

async fn logout_all(
    State(state): State<AuthState>,
    CurrentUser(user): CurrentUser,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    state
        .sessions
        .delete_user_sessions(&user.id.to_string())
        .await?;

    // Remove cookie
    let jar = jar.remove(Cookie::from(state.config.cookie_name.clone()));
    Ok(jar)
}

async fn me(CurrentUser(user): CurrentUser) -> Json<User> {
    Json(user)
}
