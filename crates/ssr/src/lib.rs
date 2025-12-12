//! SSR Worker Pool - Imperative Shell.
//!
//! This crate orchestrates I/O operations using pure functions from
//! `calendsync_ssr_core`. It provides a worker pool for concurrent
//! React server-side rendering using `deno_core`.
//!
//! # Architecture
//!
//! - **Functional Core** (`calendsync_ssr_core`): Pure validation, config, polyfills
//! - **Imperative Shell** (this crate): I/O, threading, JsRuntime execution
//!
//! # Example
//!
//! ```ignore
//! use calendsync_ssr::{SsrPool, SsrPoolConfig, SsrConfig};
//! use std::path::Path;
//!
//! // Create pool config with validation
//! let pool_config = SsrPoolConfig::with_defaults(4).unwrap();
//!
//! // Create pool (I/O: reads bundle, spawns threads)
//! let pool = SsrPool::new(pool_config, Path::new("dist/server.js")).unwrap();
//!
//! // Render (I/O: sends to worker, waits for response)
//! let config = SsrConfig::new(serde_json::json!({
//!     "initialData": { "calendarId": "abc" }
//! })).unwrap();
//! let html = pool.render(config).await.unwrap();
//! ```

mod error;
mod pool;
mod runtime;
mod worker;

// Re-export core types for convenience
pub use calendsync_ssr_core::{SsrConfig, SsrCoreError, SsrPoolConfig, MAX_INITIAL_DATA_SIZE};

// Export shell types
pub use error::{sanitize_error, Result, SsrError};
pub use pool::{HealthStatus, SsrPool, SsrPoolStats};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use worker::{RenderRequest, SsrWorker};

    /// Minimal JavaScript bundle that sets HTML via our op.
    const MINIMAL_BUNDLE: &str = r#"
        globalThis.Deno.core.ops.op_set_html('<html>test</html>');
    "#;

    fn create_test_worker() -> SsrWorker {
        let bundle_code = Arc::new(MINIMAL_BUNDLE.to_string());
        let config = Arc::new(SsrPoolConfig::with_defaults(1).unwrap());
        SsrWorker::spawn(bundle_code, config)
    }

    #[test]
    fn worker_terminates_on_drop() {
        // Spawn a worker and immediately drop it
        let worker = create_test_worker();

        // Give worker time to start
        std::thread::sleep(Duration::from_millis(50));

        // Worker should be running
        assert!(!worker.is_finished(), "Worker should be running");

        // Drop the worker (triggers shutdown)
        drop(worker);

        // Give the background cleanup thread time to work
        std::thread::sleep(Duration::from_millis(200));

        // Note: We can't directly check if thread terminated since we don't
        // have the handle anymore, but we can verify no panic occurred.
    }

    #[tokio::test]
    async fn worker_completes_inflight_request_before_shutdown() {
        let worker = create_test_worker();

        // Send a request
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let sender = worker.sender();

        sender
            .send(RenderRequest {
                config_json: r#"{"initialData":{"calendarId":"test"}}"#.to_string(),
                response_tx,
            })
            .await
            .expect("Failed to send request");

        // Wait for response (should complete before shutdown)
        let result = tokio::time::timeout(Duration::from_secs(5), response_rx)
            .await
            .expect("Timed out waiting for response")
            .expect("Channel closed");

        // Response should be Ok (render succeeded)
        assert!(result.is_ok(), "Expected successful render: {:?}", result);

        // Now drop (should shutdown cleanly)
        drop(worker);
    }

    #[tokio::test]
    async fn pool_shutdown_terminates_all_workers() {
        let config = SsrPoolConfig::with_defaults(2).unwrap();

        // Create a temporary bundle file
        let temp_dir = std::env::temp_dir();
        let bundle_path = temp_dir.join("test_bundle.js");
        std::fs::write(&bundle_path, MINIMAL_BUNDLE).expect("Failed to write test bundle");

        // Create pool
        let pool = SsrPool::new(config, &bundle_path).expect("Failed to create pool");

        // Verify pool has workers
        let stats = pool.stats();
        assert_eq!(stats.worker_count, 2, "Pool should have 2 workers");

        // Drop the pool (should shutdown all workers)
        drop(pool);

        // Give workers time to terminate
        std::thread::sleep(Duration::from_millis(200));

        // Clean up temp file
        let _ = std::fs::remove_file(&bundle_path);

        // Note: We can't directly check if workers terminated, but we can
        // verify no panic occurred and the Drop completed.
    }
}
