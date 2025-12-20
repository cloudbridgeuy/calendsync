//! SQLite storage backend implementation.
//!
//! This module provides a SQLite-based implementation of the repository traits
//! using `rusqlite` for synchronous operations and `tokio-rusqlite` for async wrapping.
//!
//! Note: This module is currently not wired into the application handlers.
//! The dead_code warnings are expected until Phase 4 (Integration) is complete.

#![allow(dead_code)]

mod conversions;
mod error;
mod repository;
mod schema;

pub use repository::SqliteRepository;
