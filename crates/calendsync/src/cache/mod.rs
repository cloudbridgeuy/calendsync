//! Cache backend implementations.
//!
//! This module provides concrete implementations of the cache traits
//! defined in `calendsync_core::cache`. The implementations are selected
//! at compile time via feature flags.
//!
//! # Feature Flags
//!
//! - `memory` (default): In-memory cache using tokio synchronization primitives
//! - `redis`: Redis cache using the redis crate
//!
//! These features are mutually exclusive - only one cache backend can be
//! enabled at a time.

// Compile-time checks for mutual exclusivity
#[cfg(all(feature = "memory", feature = "redis"))]
compile_error!(
    "Features 'memory' and 'redis' are mutually exclusive. \
    Enable only one cache backend at a time."
);

#[cfg(not(any(feature = "memory", feature = "redis")))]
compile_error!(
    "No cache backend selected. Enable 'memory' or 'redis' feature. \
    Example: cargo build -p calendsync --features memory"
);

#[cfg(feature = "memory")]
pub mod memory;

#[cfg(feature = "redis")]
pub mod redis_impl;

// Re-export the active cache implementation
#[cfg(feature = "memory")]
#[allow(unused_imports)]
pub use memory::{MemoryCache, MemoryPubSub};

#[cfg(feature = "redis")]
#[allow(unused_imports)]
pub use redis_impl::{RedisCache, RedisPubSub};
