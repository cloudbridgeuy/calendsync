//! HTTP client for calendsync API.

pub mod calendars;
pub mod entries;
pub mod events;
pub mod health;
pub mod users;

use crate::error::{ClientError, Result};

/// HTTP client for the calendsync API.
#[derive(Debug, Clone)]
pub struct CalendsyncClient {
    client: reqwest::Client,
    base_url: String,
}

impl CalendsyncClient {
    /// Create a new client with the given base URL.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// Create from environment (CALENDSYNC_URL or default).
    pub fn from_env() -> Self {
        let base_url =
            std::env::var("CALENDSYNC_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        Self::new(base_url)
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Build a URL for an endpoint.
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Handle error responses.
    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T> {
        let status = response.status();
        if status.is_success() {
            response.json().await.map_err(ClientError::from)
        } else if status.as_u16() == 404 {
            Err(ClientError::NotFound {
                resource: "Resource".to_string(),
            })
        } else {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(ClientError::ServerError {
                status: status.as_u16(),
                message,
            })
        }
    }

    /// Handle delete responses (no body expected).
    async fn handle_delete_response(&self, response: reqwest::Response) -> Result<()> {
        let status = response.status();
        if status.is_success() {
            Ok(())
        } else if status.as_u16() == 404 {
            Err(ClientError::NotFound {
                resource: "Resource".to_string(),
            })
        } else {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(ClientError::ServerError {
                status: status.as_u16(),
                message,
            })
        }
    }
}
