use serde::Deserialize;
use uuid::Uuid;

// Re-export User from core
pub use calendsync_core::calendar::User;

/// Request payload for creating a new user.
#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
}

impl CreateUser {
    /// Convert to a User with a generated UUID.
    pub fn into_user(self) -> User {
        User {
            id: Uuid::new_v4(),
            name: self.name,
            email: self.email,
        }
    }
}
