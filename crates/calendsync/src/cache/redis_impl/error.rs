//! Redis error mapping to CacheError.

use calendsync_core::cache::CacheError;

/// Maps Redis errors to CacheError.
pub fn map_redis_error(err: redis::RedisError) -> CacheError {
    if err.is_connection_refusal() || err.is_timeout() || err.is_connection_dropped() {
        CacheError::ConnectionFailed(err.to_string())
    } else {
        CacheError::OperationFailed(err.to_string())
    }
}
