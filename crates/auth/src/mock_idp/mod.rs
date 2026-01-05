//! Mock Identity Provider for testing.
//!
//! Provides a fake OIDC server for integration tests without real OAuth providers.

mod server;
mod templates;

pub use server::MockIdpServer;
