# Authentication Architecture

## Overview

OIDC authentication with Google and Apple providers, server-side sessions, role-based calendar access.

## Crate Structure

```
calendsync_core (auth feature)
├── auth/
│   ├── types.rs       # Session, SessionId, OidcClaims, OidcProvider, AuthFlowState
│   ├── traits.rs      # SessionRepository, OidcProviderClient traits
│   ├── functions.rs   # Pure functions: generate_session_id, generate_state,
│   │                  # calculate_expiry, is_session_expired, email_to_name
│   └── error.rs       # AuthError enum

calendsync_auth
├── config.rs          # AuthConfig, ProviderConfig, AppleConfig
├── state.rs           # AuthState (shared state for auth handlers)
├── handlers.rs        # Auth route handlers (login, callback, logout, me)
├── extractors.rs      # CurrentUser, OptionalUser axum extractors
├── error.rs           # Auth-specific errors
├── providers/
│   ├── google.rs      # GoogleProvider (OIDC client)
│   ├── apple.rs       # AppleProvider (OIDC client)
│   └── mock.rs        # MockProvider (for testing)
├── sessions/
│   ├── sqlite.rs      # SQLite session storage
│   └── redis_impl.rs  # Redis session storage
└── mock_idp/
    ├── server.rs      # Mock IdP HTTP server
    └── templates.rs   # Login page HTML templates

calendsync (main app)
├── Wires auth into app via AuthState
├── Spawns mock IdP in dev mode
└── Protected handlers use CurrentUser extractor
```

## Session Flow

```
┌──────────┐     1. Click login      ┌──────────┐
│  Client  │ ───────────────────────>│  Server  │
└──────────┘                         └──────────┘
                                           │
     2. Generate PKCE + state              │
     3. Store AuthFlowState (5min TTL)     │
                                           │
┌──────────┐     4. Redirect           ┌──────────┐
│  Client  │ <─────────────────────────│  Server  │
└──────────┘                           └──────────┘
     │
     │ 5. User authenticates
     ▼
┌──────────┐     6. Callback           ┌──────────┐
│ Provider │ ─────────────────────────>│  Server  │
└──────────┘   (code + state)          └──────────┘
                                           │
     7. Validate state, retrieve PKCE      │
     8. Exchange code for ID token         │
     9. Extract claims from token          │
    10. Find or create user                │
    11. Create session                     │
    12. Set secure cookie                  │
                                           │
┌──────────┐    13. Redirect to app    ┌──────────┐
│  Client  │ <─────────────────────────│  Server  │
└──────────┘                           └──────────┘
```

## Key Types

### Session
```rust
pub struct Session {
    pub id: SessionId,
    pub user_id: String,
    pub provider: OidcProvider,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
```

### OidcClaims
```rust
pub struct OidcClaims {
    pub subject: String,        // Provider's unique user ID
    pub email: Option<String>,
    pub name: Option<String>,
    pub provider: OidcProvider,
}
```

### AuthFlowState (PKCE)
```rust
pub struct AuthFlowState {
    pub pkce_verifier: String,
    pub provider: OidcProvider,
    pub created_at: DateTime<Utc>,
    pub return_to: Option<String>,  // Post-login redirect URL
}
```

## Extractors

### CurrentUser
Returns 401 if not authenticated. Use for protected endpoints.

```rust
async fn protected_handler(
    CurrentUser(user): CurrentUser,
) -> impl IntoResponse {
    // user is guaranteed to be authenticated
    Json(user)
}
```

### OptionalUser
Returns `None` if not authenticated. Use for optional auth.

```rust
async fn public_handler(
    OptionalUser(user): OptionalUser,
) -> impl IntoResponse {
    match user {
        Some(u) => format!("Hello, {}!", u.name),
        None => "Hello, guest!".to_string(),
    }
}
```

Both extractors support:
- Cookie-based auth (web clients): reads `session` cookie
- Bearer token auth (API/mobile): reads `Authorization: Bearer <session_id>` header

## Authorization

Calendar access is controlled by `CalendarMembership`:

| Role   | Create | Read | Update | Delete |
|--------|--------|------|--------|--------|
| Owner  | Yes    | Yes  | Yes    | Yes    |
| Writer | Yes    | Yes  | Yes    | No     |
| Reader | No     | Yes  | No     | No     |

Handlers check membership before allowing operations:
```rust
// Example: Check user has write access
let membership = state.memberships
    .get_membership(calendar_id, user.id)
    .await?
    .ok_or(AppError::Forbidden)?;

if !membership.role.can_write() {
    return Err(AppError::Forbidden);
}
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `sqlite` | SQLite session storage (development) |
| `redis` | Redis session storage (production) |
| `mock` | Mock IdP server (development/testing only) |

Example builds:
```bash
# Development with SQLite sessions
cargo build -p calendsync_auth --features sqlite

# Production with Redis sessions
cargo build -p calendsync_auth --features redis

# Testing with mock provider
cargo build -p calendsync_auth --features sqlite,mock
```

## Configuration

All configuration is loaded from environment variables via `AuthConfig::from_env()`:

```rust
let config = AuthConfig::from_env()?;

// Config contains:
// - google: Option<ProviderConfig>
// - apple: Option<AppleConfig>
// - session_ttl: Duration
// - base_url: Url
// - cookie_name: String
// - cookie_secure: bool
```

## Key Files

| File | Purpose |
|------|---------|
| `crates/core/src/auth/` | Pure types and traits (Functional Core) |
| `crates/auth/src/lib.rs` | Public API exports |
| `crates/auth/src/handlers.rs` | HTTP route handlers |
| `crates/auth/src/extractors.rs` | CurrentUser, OptionalUser extractors |
| `crates/auth/src/state.rs` | AuthState with provider clients |
| `crates/auth/src/config.rs` | Configuration from environment |

## Authentication-Based Routing

Routes redirect users based on authentication state using the `OptionalUser` extractor.

### Route Behaviors

| Route | Unauthenticated | Authenticated |
|-------|-----------------|---------------|
| `/` | Redirect to `/login` | Redirect to first calendar |
| `/login` | Render login page | Redirect to first calendar |
| `/login?return_to=/calendar/x` | Render login page with return_to in links | Redirect to first calendar |
| `/calendar/{id}` | Redirect to `/login?return_to=/calendar/{id}` | Show calendar |

### Login Page

The login page (`/login`) renders provider buttons based on enabled auth providers:

```rust
// crates/calendsync/src/handlers/login.rs
pub async fn login_page(
    State(state): State<AppState>,
    OptionalUser(user): OptionalUser,
    Query(query): Query<LoginQuery>,
) -> Response {
    if let Some(user) = user {
        return redirect_to_first_calendar(&state, user.id).await;
    }
    render_login_html(&state, query.return_to)
}
```

The `return_to` parameter is URL-encoded and appended to provider login links:
- `/auth/google/login?return_to=%2Fcalendar%2Fabc123`
- `/auth/apple/login?return_to=%2Fcalendar%2Fabc123`

### Return-To Flow

1. User visits `/calendar/{id}` while unauthenticated
2. Handler redirects to `/login?return_to=/calendar/{id}`
3. User clicks provider button, which includes `return_to` in query string
4. Auth handler validates and stores `return_to` in `AuthFlowState`
5. After successful authentication, callback redirects to stored `return_to` URL
6. User lands on their original destination

### URL Validation

The `validate_return_to` function prevents open redirect attacks:

```rust
// crates/core/src/auth/validation.rs
pub fn validate_return_to(url: &str) -> Option<&str> {
    if !url.starts_with('/') { return None; }  // Must be relative
    if url.starts_with("//") { return None; }  // No protocol-relative
    if url.chars().any(|c| c.is_control()) { return None; }
    if url.contains("://") { return None; }    // No embedded schemes
    Some(url)
}
```

Invalid URLs are silently ignored, defaulting to `/` after login.

### Key Files

| File | Purpose |
|------|---------|
| `crates/calendsync/src/handlers/login.rs` | Login page handler |
| `crates/calendsync/src/handlers/root.rs` | Root redirect handler |
| `crates/core/src/auth/validation.rs` | URL validation (pure function) |

## Security Considerations

1. **PKCE Flow**: All OIDC flows use PKCE (S256) to prevent authorization code interception
2. **State Parameter**: Random state prevents CSRF attacks on the callback
3. **Secure Cookies**: Session cookies are HttpOnly, Secure (in prod), SameSite=Lax
4. **Session Expiry**: Sessions expire after configurable TTL (default: 7 days)
5. **Logout All**: Users can invalidate all sessions across devices
6. **Open Redirect Prevention**: `return_to` URLs are validated to be relative paths only
