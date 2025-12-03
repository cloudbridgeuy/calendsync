//! Entry API operations.

use super::CalendsyncClient;
use crate::error::Result;
use calendsync_core::calendar::CalendarEntry;
use uuid::Uuid;

// Re-export from core for public API
pub use calendsync_core::calendar::{
    CreateEntryRequest, EntryType, ListEntriesQuery, UpdateEntryRequest,
};

impl CalendsyncClient {
    /// List entries with filters.
    pub async fn list_entries(&self, query: ListEntriesQuery) -> Result<Vec<CalendarEntry>> {
        let response = self
            .client
            .get(self.url("/api/entries"))
            .query(&query)
            .send()
            .await?;
        self.handle_response(response).await
    }

    /// Create a new entry.
    pub async fn create_entry(&self, req: CreateEntryRequest) -> Result<CalendarEntry> {
        let response = self
            .client
            .post(self.url("/api/entries"))
            .form(&req)
            .send()
            .await?;
        self.handle_response(response).await
    }

    /// Get entry by ID.
    pub async fn get_entry(&self, id: Uuid) -> Result<CalendarEntry> {
        let response = self
            .client
            .get(self.url(&format!("/api/entries/{}", id)))
            .send()
            .await?;
        self.handle_response(response).await
    }

    /// Update an entry.
    pub async fn update_entry(&self, id: Uuid, req: UpdateEntryRequest) -> Result<CalendarEntry> {
        let response = self
            .client
            .put(self.url(&format!("/api/entries/{}", id)))
            .form(&req)
            .send()
            .await?;
        self.handle_response(response).await
    }

    /// Delete entry by ID.
    pub async fn delete_entry(&self, id: Uuid) -> Result<()> {
        let response = self
            .client
            .delete(self.url(&format!("/api/entries/{}", id)))
            .send()
            .await?;
        self.handle_delete_response(response).await
    }

    /// Toggle task completion status.
    pub async fn toggle_entry(&self, id: Uuid) -> Result<CalendarEntry> {
        let response = self
            .client
            .patch(self.url(&format!("/api/entries/{}/toggle", id)))
            .send()
            .await?;
        self.handle_response(response).await
    }
}
