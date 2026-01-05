//! Flash message utilities for server-to-client communication.
//!
//! Flash messages are short-lived messages stored in cookies that get displayed
//! once on the client and then cleared. Used for communicating state across redirects.

use axum::http::header::SET_COOKIE;
use axum::response::{IntoResponse, Redirect, Response};
use serde::{Deserialize, Serialize};

/// Flash message structure stored in cookie.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlashMessage {
    /// Message type (e.g., "error", "success", "info", "warning")
    #[serde(rename = "type")]
    pub message_type: String,
    /// The message content to display
    pub message: String,
    /// Whether the message should auto-dismiss after a few seconds
    pub auto_dismiss: bool,
}

impl FlashMessage {
    /// Create an error flash message that requires manual dismissal.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message_type: "error".to_string(),
            message: message.into(),
            auto_dismiss: false,
        }
    }

    /// Create a success flash message that auto-dismisses.
    #[allow(dead_code)]
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message_type: "success".to_string(),
            message: message.into(),
            auto_dismiss: true,
        }
    }

    /// Create an info flash message that auto-dismisses.
    #[allow(dead_code)]
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message_type: "info".to_string(),
            message: message.into(),
            auto_dismiss: true,
        }
    }

    /// Set whether the message should auto-dismiss.
    #[allow(dead_code)]
    pub fn with_auto_dismiss(mut self, auto_dismiss: bool) -> Self {
        self.auto_dismiss = auto_dismiss;
        self
    }

    /// Serialize to JSON for cookie storage.
    pub fn to_cookie_value(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Build a Set-Cookie header value for the flash message.
    ///
    /// Cookie properties:
    /// - Path: / (accessible from any page)
    /// - SameSite: Lax (sent on navigation, not cross-site requests)
    /// - Max-Age: 60 (expires after 60 seconds as a safety net)
    /// - Not HttpOnly (must be readable by JavaScript)
    pub fn to_set_cookie_header(&self) -> String {
        let cookie_value = self.to_cookie_value();
        let encoded = urlencoding::encode(&cookie_value);
        format!(
            "flash_message={}; Path=/; SameSite=Lax; Max-Age=60",
            encoded
        )
    }
}

/// Create a redirect response with a flash message cookie.
pub fn redirect_with_flash(url: &str, flash: FlashMessage) -> Response {
    let cookie_header = flash.to_set_cookie_header();

    ([(SET_COOKIE, cookie_header)], Redirect::to(url)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_flash_message() {
        let flash = FlashMessage::error("Access denied");
        assert_eq!(flash.message_type, "error");
        assert_eq!(flash.message, "Access denied");
        assert!(!flash.auto_dismiss);
    }

    #[test]
    fn test_success_flash_message() {
        let flash = FlashMessage::success("Changes saved");
        assert_eq!(flash.message_type, "success");
        assert_eq!(flash.message, "Changes saved");
        assert!(flash.auto_dismiss);
    }

    #[test]
    fn test_to_cookie_value() {
        let flash = FlashMessage::error("Test message");
        let json = flash.to_cookie_value();
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("\"message\":\"Test message\""));
        assert!(json.contains("\"autoDismiss\":false"));
    }

    #[test]
    fn test_to_set_cookie_header() {
        let flash = FlashMessage::error("Test");
        let header = flash.to_set_cookie_header();
        assert!(header.starts_with("flash_message="));
        assert!(header.contains("Path=/"));
        assert!(header.contains("SameSite=Lax"));
        assert!(header.contains("Max-Age=60"));
    }
}
