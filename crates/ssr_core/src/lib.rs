//! Pure SSR logic - no I/O, no async, no side effects.
//!
//! This crate provides:
//! - Configuration types with validation
//! - Polyfill generation for React 19 SSR
//! - Error types for validation failures
//!
//! # Example
//!
//! ```
//! use calendsync_ssr_core::{SsrConfig, SsrPoolConfig, generate_polyfills};
//!
//! // Create validated config
//! let config = SsrConfig::new(serde_json::json!({
//!     "initialData": { "calendarId": "abc" }
//! })).unwrap();
//!
//! // Serialize to JSON
//! let json = config.to_json().unwrap();
//!
//! // Generate polyfills (pure string transformation)
//! let polyfills = generate_polyfills(&json, "production").unwrap();
//!
//! // Create pool config with validation
//! let pool_config = SsrPoolConfig::with_defaults(4).unwrap();
//! assert_eq!(pool_config.worker_count, 4);
//! ```

mod config;
mod error;
mod polyfills;

pub use config::{SsrConfig, SsrPoolConfig};
pub use error::{Result, SsrCoreError, MAX_INITIAL_DATA_SIZE};
pub use polyfills::generate_polyfills;
