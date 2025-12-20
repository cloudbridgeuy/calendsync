use std::{env, time::Duration};

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Cache TTL in seconds (default: 300)
    pub cache_ttl_seconds: u64,
    /// Maximum number of cache entries (default: 10,000)
    pub cache_max_entries: usize,
    /// Maximum size of event history for SSE (default: 1,000)
    pub event_history_max_size: usize,
    /// Path to SQLite database file (default: "calendsync.db")
    pub sqlite_path: String,
    /// Redis connection URL (default: "redis://localhost:6379")
    /// Note: Only used when the `redis` feature is enabled.
    #[allow(dead_code)]
    pub redis_url: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Environment variables:
    /// - `CACHE_TTL_SECONDS` - Cache TTL in seconds (default: 300)
    /// - `CACHE_MAX_ENTRIES` - Maximum cache entries (default: 10,000)
    /// - `EVENT_HISTORY_MAX_SIZE` - SSE event history size (default: 1,000)
    /// - `SQLITE_PATH` - SQLite database path (default: "calendsync.db")
    /// - `REDIS_URL` - Redis connection URL (default: "redis://localhost:6379")
    pub fn from_env() -> Self {
        Self {
            cache_ttl_seconds: env::var("CACHE_TTL_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            cache_max_entries: env::var("CACHE_MAX_ENTRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10_000),
            event_history_max_size: env::var("EVENT_HISTORY_MAX_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1_000),
            sqlite_path: env::var("SQLITE_PATH").unwrap_or_else(|_| "calendsync.db".to_string()),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
        }
    }

    /// Get cache TTL as a Duration.
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache_ttl_seconds)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_ttl_conversion() {
        let config = Config {
            cache_ttl_seconds: 600,
            cache_max_entries: 10_000,
            event_history_max_size: 1_000,
            sqlite_path: "test.db".to_string(),
            redis_url: "redis://localhost:6379".to_string(),
        };

        assert_eq!(config.cache_ttl(), Duration::from_secs(600));
    }

    #[test]
    fn test_default_values() {
        // Clear environment variables to test defaults
        env::remove_var("CACHE_TTL_SECONDS");
        env::remove_var("CACHE_MAX_ENTRIES");
        env::remove_var("EVENT_HISTORY_MAX_SIZE");
        env::remove_var("SQLITE_PATH");
        env::remove_var("REDIS_URL");

        let config = Config::from_env();

        assert_eq!(config.cache_ttl_seconds, 300);
        assert_eq!(config.cache_max_entries, 10_000);
        assert_eq!(config.event_history_max_size, 1_000);
        assert_eq!(config.sqlite_path, "calendsync.db");
        assert_eq!(config.redis_url, "redis://localhost:6379");
    }
}
