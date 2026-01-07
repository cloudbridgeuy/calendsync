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
    calculate_expiry, email_to_name, generate_session_id, generate_state, validate_redirect_uri,
    validate_return_to, AuthFlowState, OidcClaims, OidcProvider, Session,
};
use calendsync_core::calendar::{Calendar, CalendarMembership, User};
use chrono::{Duration, Utc};
use openidconnect::PkceCodeChallenge;
use serde::{Deserialize, Serialize};

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
    /// Custom redirect URI for native apps (e.g., calendsync://auth/callback).
    /// When set, the callback will redirect to this URI with code+state params
    /// instead of processing the code exchange (native app calls /auth/exchange).
    pub redirect_uri: Option<String>,
}

/// Request body for code exchange (native apps).
#[derive(Deserialize)]
pub struct ExchangeRequest {
    pub code: String,
    pub state: String,
}

/// Response for code exchange.
#[derive(Serialize)]
pub struct ExchangeResponse {
    pub session_id: String,
}

/// Creates the auth router with all authentication routes.
///
/// Routes:
/// - `GET /auth/google/login` - Initiate Google OIDC flow
/// - `GET /auth/google/callback` - Handle Google OIDC callback
/// - `GET /auth/apple/login` - Initiate Apple OIDC flow
/// - `POST /auth/apple/callback` - Handle Apple OIDC callback (form POST)
/// - `POST /auth/exchange` - Exchange code for session (native apps)
/// - `POST /auth/logout` - End current session
/// - `POST /auth/logout-all` - End all sessions for current user
/// - `GET /auth/me` - Get current authenticated user
pub fn auth_routes() -> Router<AuthState> {
    Router::new()
        .route("/auth/google/login", get(google_login))
        .route("/auth/google/callback", get(google_callback))
        .route("/auth/apple/login", get(apple_login))
        .route("/auth/apple/callback", post(apple_callback))
        .route("/auth/exchange", post(exchange))
        .route("/auth/logout", post(logout))
        .route("/auth/logout-all", post(logout_all))
        .route("/auth/me", get(me))
}

async fn google_login(
    State(state): State<AuthState>,
    Query(query): Query<LoginQuery>,
) -> Result<Redirect, AuthError> {
    initiate_login(
        &state,
        OidcProvider::Google,
        query.return_to,
        query.redirect_uri,
    )
    .await
}

async fn apple_login(
    State(state): State<AuthState>,
    Query(query): Query<LoginQuery>,
) -> Result<Redirect, AuthError> {
    initiate_login(
        &state,
        OidcProvider::Apple,
        query.return_to,
        query.redirect_uri,
    )
    .await
}

async fn initiate_login(
    state: &AuthState,
    provider: OidcProvider,
    return_to: Option<String>,
    redirect_uri: Option<String>,
) -> Result<Redirect, AuthError> {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let csrf_state = generate_state();

    // Validate return_to URL to prevent open redirect attacks
    let validated_return_to = return_to
        .as_deref()
        .and_then(validate_return_to)
        .map(String::from);

    // Validate redirect_uri to prevent redirect attacks
    let validated_redirect_uri = redirect_uri
        .as_deref()
        .and_then(validate_redirect_uri)
        .map(String::from);

    // Store PKCE verifier for callback
    let flow = AuthFlowState {
        pkce_verifier: pkce_verifier.secret().to_string(),
        provider,
        created_at: Utc::now(),
        return_to: validated_return_to,
        redirect_uri: validated_redirect_uri,
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

/// Result of processing an OAuth callback.
struct CallbackResult {
    session: Session,
    return_to: Option<String>,
}

/// Common logic for OAuth code exchange.
///
/// This is shared between the web callback (sets cookie) and the native app
/// exchange endpoint (returns session_id as JSON).
async fn process_code_exchange(
    state: &AuthState,
    code: &str,
    csrf_state: &str,
    apple_name: Option<String>,
) -> Result<CallbackResult, AuthError> {
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

    Ok(CallbackResult {
        session,
        return_to: flow.return_to,
    })
}

/// Exchange authorization code for session (native apps).
///
/// Returns the session_id as JSON AND sets a cookie, allowing native apps
/// to use either mechanism for subsequent requests.
async fn exchange(
    State(state): State<AuthState>,
    jar: CookieJar,
    Json(request): Json<ExchangeRequest>,
) -> Result<(CookieJar, Json<ExchangeResponse>), AuthError> {
    let result = process_code_exchange(&state, &request.code, &request.state, None).await?;

    // Set cookie (same as web callback)
    let cookie_name = state.config.cookie_name.clone();
    let session_value = result.session.id.to_string();
    let cookie = Cookie::build((cookie_name, session_value.clone()))
        .path("/")
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(
            state.config.session_ttl.as_secs() as i64
        ))
        .build();

    let jar = jar.add(cookie);

    Ok((
        jar,
        Json(ExchangeResponse {
            session_id: session_value,
        }),
    ))
}

async fn handle_callback(
    state: &AuthState,
    code: &str,
    csrf_state: &str,
    jar: CookieJar,
    apple_name: Option<String>,
) -> Result<(CookieJar, Redirect), AuthError> {
    // Check if this is a native app flow by peeking at the auth flow
    // If redirect_uri is set with calendsync://auth/callback, redirect with code+state
    // without processing (native app will call /auth/exchange)
    if let Some(flow) = state.sessions.peek_auth_flow(csrf_state).await? {
        if let Some(redirect_uri) = &flow.redirect_uri {
            if redirect_uri.starts_with("calendsync://auth/callback") {
                // Native app: DON'T consume the auth flow here - the native app
                // needs it when it calls /auth/exchange with code and state.
                // The flow will be consumed by process_code_exchange when
                // the native app calls /auth/exchange.

                // Redirect to native app with code and state
                let redirect_url = format!(
                    "{}?code={}&state={}",
                    redirect_uri,
                    urlencoding::encode(code),
                    urlencoding::encode(csrf_state)
                );
                return Ok((jar, Redirect::to(&redirect_url)));
            }
        }
    }

    // Web app: process normally
    let result = process_code_exchange(state, code, csrf_state, apple_name).await?;

    // Set secure cookie
    let cookie_name = state.config.cookie_name.clone();
    let session_value = result.session.id.to_string();
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
    let redirect_url = result.return_to.unwrap_or_else(|| "/".to_string());
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
