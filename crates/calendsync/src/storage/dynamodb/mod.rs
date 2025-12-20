//! DynamoDB storage backend implementation.
//!
//! This module provides a DynamoDB-based implementation of the repository traits
//! using `aws-sdk-dynamodb`.
//!
//! Note: This module is currently not wired into the application handlers.
//! The dead_code warnings are expected until Phase 4 (Integration) is complete.

#![allow(dead_code)]

mod conversions;
mod error;
mod keys;
mod repository;

pub use repository::DynamoDbRepository;
