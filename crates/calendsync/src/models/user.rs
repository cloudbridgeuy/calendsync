use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

/// Request payload for creating a new user.
#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
}

impl User {
    /// Create a new user with a generated UUID.
    pub fn new(name: String, email: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            email,
        }
    }
}
