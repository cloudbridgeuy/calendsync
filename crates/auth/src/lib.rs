//! OIDC authentication for calendsync.
//!
//! This crate provides:
//! - OIDC flows with Google and Apple providers
//! - Session storage (SQLite or Redis via feature flags)
//! - Axum extractors for authentication

mod config;
mod error;
mod extractors;
mod handlers;
mod providers;
mod sessions;
mod state;

pub use config::{AppleConfig, AuthConfig, ProviderConfig};
pub use error::AuthError;
pub use extractors::{CurrentUser, OptionalUser};
pub use handlers::auth_routes;
#[cfg(feature = "mock")]
pub use providers::MockProvider;
pub use providers::{AppleProvider, GoogleProvider};
#[cfg(any(feature = "sqlite", feature = "redis"))]
pub use sessions::SessionStore;
pub use state::AuthState;

#[cfg(feature = "mock")]
pub mod mock_idp;
