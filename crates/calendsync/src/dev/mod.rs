//! Dev annotations SQLite persistence layer.
//!
//! Only compiled when the `dev-annotations` feature is enabled.

pub mod schema;
pub mod store;

pub use store::DevAnnotationStore;
