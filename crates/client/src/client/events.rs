//! SSE events operations.

use super::CalendsyncClient;
use crate::error::{ClientError, Result};
use uuid::Uuid;

// Re-export from core for public API
pub use calendsync_core::calendar::CalendarEvent;

impl CalendsyncClient {
    /// Watch SSE events for a calendar.
    /// Returns a stream of events.
    pub async fn watch_events(
        &self,
        calendar_id: Uuid,
        last_event_id: Option<u64>,
    ) -> Result<impl futures_core::Stream<Item = Result<CalendarEvent>>> {
        let mut url = format!("{}/api/events?calendar_id={}", self.base_url, calendar_id);
        if let Some(id) = last_event_id {
            url.push_str(&format!("&last_event_id={}", id));
        }

        let response = self
            .client
            .get(&url)
            .header("Accept", "text/event-stream")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ClientError::ServerError {
                status: response.status().as_u16(),
                message: "Failed to connect to SSE endpoint".to_string(),
            });
        }

        let stream = async_stream::stream! {
            use tokio_stream::StreamExt;

            let mut byte_stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));

                        // Parse complete SSE events from buffer
                        while let Some(pos) = buffer.find("\n\n") {
                            let event_str = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();

                            if let Some(event) = parse_sse_event(&event_str) {
                                yield Ok(event);
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(ClientError::Connection(e.to_string()));
                        break;
                    }
                }
            }
        };

        Ok(stream)
    }
}

/// Parse an SSE event from a string.
fn parse_sse_event(event_str: &str) -> Option<CalendarEvent> {
    let mut data = None;

    for line in event_str.lines() {
        if let Some(value) = line.strip_prefix("data: ") {
            data = Some(value);
        }
    }

    data.and_then(|d| serde_json::from_str(d).ok())
}
