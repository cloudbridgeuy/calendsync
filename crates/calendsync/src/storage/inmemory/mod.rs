//! In-memory storage backend for testing.
//!
//! This module provides an in-memory implementation of the repository traits
//! that stores all data in HashMaps wrapped in `Arc<RwLock<_>>`. This is useful
//! for testing and development scenarios where persistence is not required.
//!
//! # Example
//!
//! ```rust,ignore
//! use calendsync::storage::inmemory::InMemoryRepository;
//!
//! let repo = InMemoryRepository::new();
//! // Use repo for testing...
//! ```

mod repository;

#[cfg(all(
    feature = "auth-mock",
    not(feature = "auth-sqlite"),
    not(feature = "auth-redis")
))]
mod session_store;

pub use repository::InMemoryRepository;

#[cfg(all(
    feature = "auth-mock",
    not(feature = "auth-sqlite"),
    not(feature = "auth-redis")
))]
pub use session_store::InMemorySessionStore;
