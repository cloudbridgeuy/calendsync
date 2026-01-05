//! Session storage implementations.
//!
//! Provides `SessionRepository` implementations for:
//! - SQLite (with `sqlite` feature)
//! - Redis (with `redis` feature)

#[cfg(feature = "redis")]
mod redis_impl;
#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "redis")]
pub use redis_impl::SessionStore;
#[cfg(feature = "sqlite")]
pub use sqlite::SessionStore;
