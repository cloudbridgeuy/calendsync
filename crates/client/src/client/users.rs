//! User API operations.

use super::CalendsyncClient;
use crate::error::Result;
use calendsync_core::calendar::User;
use uuid::Uuid;

impl CalendsyncClient {
    /// List all users.
    pub async fn list_users(&self) -> Result<Vec<User>> {
        let response = self.client.get(self.url("/api/users")).send().await?;
        self.handle_response(response).await
    }

    /// Create a new user.
    pub async fn create_user(&self, name: &str, email: &str) -> Result<User> {
        let response = self
            .client
            .post(self.url("/api/users"))
            .form(&[("name", name), ("email", email)])
            .send()
            .await?;
        self.handle_response(response).await
    }

    /// Get user by ID.
    pub async fn get_user(&self, id: Uuid) -> Result<User> {
        let response = self
            .client
            .get(self.url(&format!("/api/users/{}", id)))
            .send()
            .await?;
        self.handle_response(response).await
    }

    /// Delete user by ID.
    pub async fn delete_user(&self, id: Uuid) -> Result<()> {
        let response = self
            .client
            .delete(self.url(&format!("/api/users/{}", id)))
            .send()
            .await?;
        self.handle_delete_response(response).await
    }
}
