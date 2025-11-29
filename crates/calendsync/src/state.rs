use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use uuid::Uuid;

use crate::models::User;

/// Shared application state.
///
/// This is cloned for each request handler and contains shared resources
/// like the user repository.
#[derive(Clone)]
pub struct AppState {
    /// In-memory user storage.
    pub users: Arc<RwLock<HashMap<Uuid, User>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl AppState {
    /// Create a new application state.
    pub fn new() -> Self {
        Self::default()
    }
}
