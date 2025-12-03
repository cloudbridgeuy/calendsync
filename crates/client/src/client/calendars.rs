//! Calendar API operations.

use super::CalendsyncClient;
use crate::error::Result;
use calendsync_core::calendar::Calendar;
use uuid::Uuid;

/// Request for creating a calendar.
#[derive(Debug, serde::Serialize)]
pub struct CreateCalendarRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request for updating a calendar.
#[derive(Debug, serde::Serialize)]
pub struct UpdateCalendarRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl CalendsyncClient {
    /// List all calendars.
    pub async fn list_calendars(&self) -> Result<Vec<Calendar>> {
        let response = self.client.get(self.url("/api/calendars")).send().await?;
        self.handle_response(response).await
    }

    /// Create a new calendar.
    pub async fn create_calendar(&self, req: CreateCalendarRequest) -> Result<Calendar> {
        let response = self
            .client
            .post(self.url("/api/calendars"))
            .form(&req)
            .send()
            .await?;
        self.handle_response(response).await
    }

    /// Get calendar by ID.
    pub async fn get_calendar(&self, id: Uuid) -> Result<Calendar> {
        let response = self
            .client
            .get(self.url(&format!("/api/calendars/{}", id)))
            .send()
            .await?;
        self.handle_response(response).await
    }

    /// Update a calendar.
    pub async fn update_calendar(&self, id: Uuid, req: UpdateCalendarRequest) -> Result<Calendar> {
        let response = self
            .client
            .put(self.url(&format!("/api/calendars/{}", id)))
            .form(&req)
            .send()
            .await?;
        self.handle_response(response).await
    }

    /// Delete calendar by ID.
    pub async fn delete_calendar(&self, id: Uuid) -> Result<()> {
        let response = self
            .client
            .delete(self.url(&format!("/api/calendars/{}", id)))
            .send()
            .await?;
        self.handle_delete_response(response).await
    }
}
