//! Health check operations.

use super::CalendsyncClient;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// SSR health status.
#[derive(Debug, Serialize, Deserialize)]
pub struct SsrHealth {
    pub status: String,
    pub latency_ms: u64,
}

/// SSR pool statistics.
#[derive(Debug, Serialize, Deserialize)]
pub struct SsrPoolStats {
    pub worker_count: usize,
    pub workers_with_capacity: usize,
}

impl CalendsyncClient {
    /// Check SSR pool health.
    pub async fn health_ssr(&self) -> Result<SsrHealth> {
        let response = self.client.get(self.url("/health/ssr")).send().await?;
        self.handle_response(response).await
    }

    /// Get SSR pool statistics.
    pub async fn health_ssr_stats(&self) -> Result<SsrPoolStats> {
        let response = self
            .client
            .get(self.url("/health/ssr/stats"))
            .send()
            .await?;
        self.handle_response(response).await
    }
}
