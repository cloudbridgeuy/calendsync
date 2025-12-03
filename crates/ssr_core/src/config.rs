//! Configuration types for SSR with validation.

use serde::{Deserialize, Serialize};

use crate::error::{Result, SsrCoreError, MAX_INITIAL_DATA_SIZE};

/// Configuration for a single SSR render request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SsrConfig {
    /// Initial data to inject as `globalThis.__SSR_CONFIG__`.
    pub initial_data: serde_json::Value,
}

impl SsrConfig {
    /// Create a new SSR config with initial data.
    ///
    /// Validates that the payload size is within limits.
    pub fn new(initial_data: serde_json::Value) -> Result<Self> {
        // Validate size
        let size = serde_json::to_string(&initial_data)
            .map(|s| s.len())
            .unwrap_or(0);

        if size > MAX_INITIAL_DATA_SIZE {
            return Err(SsrCoreError::PayloadTooLarge {
                size,
                max: MAX_INITIAL_DATA_SIZE,
            });
        }

        Ok(Self { initial_data })
    }

    /// Serialize config to JSON string (pure transformation).
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| SsrCoreError::Serialization(e.to_string()))
    }
}

/// Configuration for the SSR worker pool (validated).
#[derive(Clone, Debug)]
pub struct SsrPoolConfig {
    /// Number of worker threads.
    pub worker_count: usize,
    /// Maximum pending requests before rejecting.
    pub max_pending: usize,
    /// Render timeout in milliseconds.
    pub render_timeout_ms: u64,
    /// NODE_ENV value.
    pub node_env: String,
}

impl SsrPoolConfig {
    /// Create and validate pool config.
    pub fn new(
        worker_count: usize,
        max_pending: usize,
        render_timeout_ms: u64,
        node_env: String,
    ) -> Result<Self> {
        if worker_count == 0 {
            return Err(SsrCoreError::InvalidWorkerCount);
        }
        if render_timeout_ms == 0 {
            return Err(SsrCoreError::InvalidTimeout);
        }

        Ok(Self {
            worker_count,
            max_pending,
            render_timeout_ms,
            node_env,
        })
    }

    /// Create with defaults (4 workers, 100 pending, 10s timeout).
    pub fn with_defaults(worker_count: usize) -> Result<Self> {
        Self::new(worker_count, 100, 10_000, "production".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssr_config_new_valid() {
        let data = serde_json::json!({"test": true});
        let config = SsrConfig::new(data).unwrap();
        assert!(config.initial_data["test"].as_bool().unwrap());
    }

    #[test]
    fn test_ssr_config_payload_too_large() {
        // Create a large payload (> 5MB)
        let large_string = "x".repeat(6 * 1024 * 1024);
        let data = serde_json::json!({"large": large_string});
        let result = SsrConfig::new(data);
        assert!(matches!(result, Err(SsrCoreError::PayloadTooLarge { .. })));
    }

    #[test]
    fn test_ssr_config_to_json() {
        let data = serde_json::json!({"key": "value"});
        let config = SsrConfig::new(data).unwrap();
        let json = config.to_json().unwrap();
        assert!(json.contains("key"));
        assert!(json.contains("value"));
    }

    #[test]
    fn test_pool_config_valid() {
        let config = SsrPoolConfig::new(4, 100, 10_000, "production".to_string()).unwrap();
        assert_eq!(config.worker_count, 4);
        assert_eq!(config.max_pending, 100);
        assert_eq!(config.render_timeout_ms, 10_000);
        assert_eq!(config.node_env, "production");
    }

    #[test]
    fn test_pool_config_zero_workers() {
        let result = SsrPoolConfig::new(0, 100, 10_000, "production".to_string());
        assert!(matches!(result, Err(SsrCoreError::InvalidWorkerCount)));
    }

    #[test]
    fn test_pool_config_zero_timeout() {
        let result = SsrPoolConfig::new(4, 100, 0, "production".to_string());
        assert!(matches!(result, Err(SsrCoreError::InvalidTimeout)));
    }

    #[test]
    fn test_pool_config_with_defaults() {
        let config = SsrPoolConfig::with_defaults(8).unwrap();
        assert_eq!(config.worker_count, 8);
        assert_eq!(config.max_pending, 100);
        assert_eq!(config.render_timeout_ms, 10_000);
        assert_eq!(config.node_env, "production");
    }
}
