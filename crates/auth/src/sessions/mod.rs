//! Session storage implementations.
//!
//! Provides `SessionRepository` implementations for:
//! - SQLite (with `sqlite` feature)
//! - Redis (with `redis` feature)
//! - In-memory (with `mock` feature)
//!
//! When multiple features are enabled, priority is: sqlite > redis > mock.
//! All modules compile independently, but only one `SessionStore` is re-exported.

#[cfg(feature = "mock")]
mod inmemory;
#[cfg(feature = "redis")]
mod redis_impl;
#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "sqlite")]
pub use sqlite::SessionStore;

#[cfg(all(feature = "redis", not(feature = "sqlite")))]
pub use redis_impl::SessionStore;

#[cfg(all(feature = "mock", not(feature = "sqlite"), not(feature = "redis")))]
pub use inmemory::SessionStore;
