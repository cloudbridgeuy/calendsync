//! Cached repository decorators.
//!
//! This module provides decorator implementations that wrap repository traits
//! with caching behavior. The decorators implement the cache-aside pattern:
//!
//! - **Reads**: Check cache first, on miss fetch from repository and populate cache
//! - **Writes**: Persist to repository, invalidate cache, publish events
//!
//! # Example
//!
//! ```ignore
//! use std::sync::Arc;
//! use std::time::Duration;
//!
//! let repo = Arc::new(SqliteRepository::new("db.sqlite").await?);
//! let cache = Arc::new(MemoryCache::new(10_000));
//! let pubsub = Arc::new(MemoryPubSub::new());
//!
//! let cached_repo = CachedEntryRepository::new(repo, cache, pubsub, Duration::from_secs(300));
//! ```

mod calendar;
mod entry;

pub use calendar::CachedCalendarRepository;
pub use entry::CachedEntryRepository;
