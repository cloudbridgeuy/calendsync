//! Storage backend implementations.
//!
//! This module provides concrete implementations of the repository traits
//! defined in `calendsync_core::storage`. The implementations are selected
//! at compile time via feature flags.
//!
//! # Feature Flags
//!
//! - `sqlite` (default): SQLite storage backend using `rusqlite` and `tokio-rusqlite`
//! - `dynamodb`: AWS DynamoDB storage backend using `aws-sdk-dynamodb`
//!
//! These features are mutually exclusive - only one storage backend can be
//! enabled at a time.
//!
//! # Examples
//!
//! Build with SQLite (default):
//! ```bash
//! cargo build -p calendsync
//! ```
//!
//! Build with DynamoDB:
//! ```bash
//! cargo build -p calendsync --no-default-features --features dynamodb
//! ```

// Compile-time checks for mutual exclusivity
#[cfg(all(feature = "sqlite", feature = "dynamodb"))]
compile_error!(
    "Features 'sqlite' and 'dynamodb' are mutually exclusive. \
    Enable only one storage backend at a time."
);

#[cfg(not(any(feature = "sqlite", feature = "dynamodb")))]
compile_error!(
    "No storage backend selected. Enable 'sqlite' or 'dynamodb' feature. \
    Example: cargo build -p calendsync --features sqlite"
);

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "dynamodb")]
pub mod dynamodb;

// Re-export the active repository implementation for convenience
// Note: These are currently unused but will be used when handlers are updated
// to use the repository pattern (Phase 4 of storage design).
#[cfg(feature = "sqlite")]
#[allow(unused_imports)]
pub use sqlite::SqliteRepository;

#[cfg(feature = "dynamodb")]
#[allow(unused_imports)]
pub use dynamodb::DynamoDbRepository;
