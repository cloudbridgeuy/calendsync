//! Server-Sent Events (SSE) module for real-time calendar updates.
//!
//! This module routes SSE through the Rust backend to ensure proper authentication
//! via session cookies. Browser EventSource bypasses Tauri's transport layer,
//! so we stream the SSE connection via reqwest and emit Tauri events.
//!
//! Architecture:
//! - `parse_sse_message` - Pure function to parse SSE text format (Functional Core)
//! - `start_connection` - Streams SSE and emits Tauri events (Imperative Shell)
//! - Last event ID tracking for reconnection catch-up

use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;

/// SSE event payload sent to frontend via Tauri events.
#[derive(Debug, Clone, Serialize)]
pub struct SseEventPayload {
    /// The parsed JSON data from the SSE event.
    pub data: serde_json::Value,
    /// The event ID from the server (for reconnection).
    pub id: Option<String>,
}

/// A parsed SSE message.
#[derive(Debug, Clone)]
pub struct SseMessage {
    /// Event type (e.g., "entry_added", "entry_updated", "entry_deleted").
    pub event_type: String,
    /// JSON data payload.
    pub data: String,
    /// Event ID for reconnection tracking.
    pub id: Option<String>,
}

/// Parse a single SSE message from buffer.
///
/// SSE format:
/// ```text
/// event: entry_added
/// data: {"entry": {...}, "date": "2026-01-08"}
/// id: 123
///
/// ```
///
/// Returns `Some((message, remaining_buffer))` if a complete message was parsed,
/// or `None` if the buffer doesn't contain a complete message yet.
pub fn parse_sse_message(buffer: &str) -> Option<(SseMessage, String)> {
    // SSE messages are terminated by double newline
    let end_pos = buffer.find("\n\n")?;

    let message_text = &buffer[..end_pos];
    let remaining = buffer[end_pos + 2..].to_string();

    let mut event_type = String::new();
    let mut data = String::new();
    let mut id = None;

    for line in message_text.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            event_type = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("data:") {
            data = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("id:") {
            id = Some(value.trim().to_string());
        }
        // Ignore comments (lines starting with ':') and unknown fields
    }

    // Skip heartbeat/keepalive messages (no event type)
    if event_type.is_empty() {
        // Return empty remaining to continue processing
        return Some((
            SseMessage {
                event_type: String::new(),
                data: String::new(),
                id: None,
            },
            remaining,
        ));
    }

    Some((
        SseMessage {
            event_type,
            data,
            id,
        },
        remaining,
    ))
}

/// Start SSE connection and emit events to the frontend.
///
/// This function:
/// 1. Connects to the server's SSE endpoint with session authentication
/// 2. Streams the response and parses SSE messages
/// 3. Emits Tauri events for each SSE message
/// 4. Tracks the last event ID for reconnection catch-up
/// 5. Handles disconnection and cancellation
///
/// # Arguments
///
/// * `app` - Tauri app handle for emitting events
/// * `calendar_id` - Calendar to subscribe to
/// * `last_event_id` - Optional last event ID for reconnection catch-up
/// * `session_id` - Session cookie for authentication
/// * `cancel_rx` - Oneshot receiver to signal cancellation
/// * `last_event_id_state` - Shared state to track the last event ID
pub async fn start_connection(
    app: AppHandle,
    calendar_id: &str,
    last_event_id: Option<String>,
    session_id: &str,
    mut cancel_rx: oneshot::Receiver<()>,
    last_event_id_state: &Arc<Mutex<Option<String>>>,
) -> Result<(), String> {
    let url = format!(
        "{}/api/events?calendar_id={}{}",
        crate::http::api_url(),
        calendar_id,
        last_event_id
            .as_ref()
            .map(|id| format!("&last_event_id={}", id))
            .unwrap_or_default()
    );

    tracing::info!("Starting SSE connection to {}", url);

    app.emit("sse:connection_state", "connecting")
        .map_err(|e| e.to_string())?;

    let response = Client::new()
        .get(&url)
        .header("Cookie", format!("session={}", session_id))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        app.emit("sse:connection_state", "error")
            .map_err(|e| e.to_string())?;
        return Err(format!("SSE connection failed: {}", status));
    }

    tracing::info!("SSE connection established");

    app.emit("sse:connection_state", "connected")
        .map_err(|e| e.to_string())?;

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    loop {
        tokio::select! {
            // Handle cancellation
            _ = &mut cancel_rx => {
                tracing::info!("SSE connection cancelled");
                break;
            }
            // Handle incoming data
            chunk = stream.next() => {
                match chunk {
                    Some(Ok(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // Parse all complete messages in the buffer
                        while let Some((msg, remaining)) = parse_sse_message(&buffer) {
                            buffer = remaining;

                            // Skip empty messages (heartbeats)
                            if msg.event_type.is_empty() {
                                continue;
                            }

                            // Track the last event ID for reconnection
                            if let Some(ref id) = msg.id {
                                *last_event_id_state.lock().unwrap() = Some(id.clone());
                            }

                            if let Err(e) = emit_event(&app, msg) {
                                tracing::error!("Failed to emit SSE event: {}", e);
                            }
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("SSE stream error: {}", e);
                        app.emit("sse:connection_state", "error")
                            .map_err(|e| e.to_string())?;
                        return Err(e.to_string());
                    }
                    None => {
                        // Stream ended - server closed connection
                        tracing::info!("SSE stream ended");
                        app.emit("sse:connection_state", "disconnected")
                            .map_err(|e| e.to_string())?;
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Emit a parsed SSE message as a Tauri event.
fn emit_event(app: &AppHandle, msg: SseMessage) -> Result<(), String> {
    let event_name = format!("sse:{}", msg.event_type);
    let payload = SseEventPayload {
        data: serde_json::from_str(&msg.data).unwrap_or(serde_json::Value::Null),
        id: msg.id,
    };

    tracing::debug!("Emitting {} event", event_name);

    app.emit(&event_name, payload).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_complete_message() {
        let buffer = "event: entry_added\ndata: {\"entry\":{\"id\":\"123\"},\"date\":\"2026-01-08\"}\nid: 42\n\n";

        let result = parse_sse_message(buffer);
        assert!(result.is_some());

        let (msg, remaining) = result.unwrap();
        assert_eq!(msg.event_type, "entry_added");
        assert_eq!(
            msg.data,
            "{\"entry\":{\"id\":\"123\"},\"date\":\"2026-01-08\"}"
        );
        assert_eq!(msg.id, Some("42".to_string()));
        assert_eq!(remaining, "");
    }

    #[test]
    fn test_parse_incomplete_message() {
        let buffer = "event: entry_added\ndata: {\"entry\":{\"id\":\"123\"}";

        let result = parse_sse_message(buffer);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_multiple_messages() {
        let buffer = "event: entry_added\ndata: {\"a\":1}\nid: 1\n\nevent: entry_updated\ndata: {\"b\":2}\nid: 2\n\n";

        // First message
        let result = parse_sse_message(buffer);
        assert!(result.is_some());
        let (msg1, remaining) = result.unwrap();
        assert_eq!(msg1.event_type, "entry_added");
        assert_eq!(msg1.data, "{\"a\":1}");

        // Second message
        let result = parse_sse_message(&remaining);
        assert!(result.is_some());
        let (msg2, remaining) = result.unwrap();
        assert_eq!(msg2.event_type, "entry_updated");
        assert_eq!(msg2.data, "{\"b\":2}");
        assert_eq!(remaining, "");
    }

    #[test]
    fn test_parse_message_without_id() {
        let buffer = "event: heartbeat\ndata: {}\n\n";

        let result = parse_sse_message(buffer);
        assert!(result.is_some());

        let (msg, _) = result.unwrap();
        assert_eq!(msg.event_type, "heartbeat");
        assert_eq!(msg.id, None);
    }

    #[test]
    fn test_parse_empty_message_heartbeat() {
        // Server sometimes sends just a comment or empty line as keepalive
        let buffer = ": keepalive\n\n";

        let result = parse_sse_message(buffer);
        assert!(result.is_some());

        let (msg, _) = result.unwrap();
        assert!(msg.event_type.is_empty()); // Empty event type = heartbeat
    }
}
