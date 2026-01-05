//! OIDC provider implementations.
//!
//! This module contains implementations of `OidcProviderClient` for:
//! - Google
//! - Apple (with JWT client secret generation)

mod apple;
mod google;
#[cfg(feature = "mock")]
mod mock;

pub use apple::AppleProvider;
pub use google::GoogleProvider;
#[cfg(feature = "mock")]
pub use mock::MockProvider;
