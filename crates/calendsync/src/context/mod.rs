//! Request-scoped context module.
//!
//! Provides `RequestContext` extractor that bundles request-scoped state
//! to complement application-scoped `AppState`.

mod extractor;
mod types;

pub use types::RequestContext;
