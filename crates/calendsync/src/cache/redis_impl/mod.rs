//! Redis cache backend implementation.
//!
//! Provides a distributed cache using Redis for multi-instance deployments.
//! Supports connection pooling, TTL, pattern-based deletion, and pub/sub.

#![allow(dead_code)]

mod cache;
mod error;
mod pubsub;

pub use cache::RedisCache;
pub use pubsub::RedisPubSub;
