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
