//! Entry API operations.

use super::CalendsyncClient;
use crate::error::Result;
use calendsync_core::calendar::CalendarEntry;
use chrono::{NaiveDate, NaiveTime};
use uuid::Uuid;

/// Query parameters for listing entries.
#[derive(Debug, Default, serde::Serialize)]
pub struct ListEntriesQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calendar_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub highlighted_day: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<u32>,
}

/// Request for creating an entry.
#[derive(Debug, serde::Serialize)]
pub struct CreateEntryRequest {
    pub calendar_id: Uuid,
    pub title: String,
    pub date: NaiveDate,
    pub entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<NaiveTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<NaiveTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Request for updating an entry.
#[derive(Debug, serde::Serialize)]
pub struct UpdateEntryRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<NaiveTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<NaiveTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
}

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
