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

pub use repository::InMemoryRepository;
