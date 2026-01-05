use chrono::{DateTime, Duration, Utc};
use rand::{distr::Alphanumeric, Rng};

use super::{Session, SessionId};

/// Generate a cryptographically random session ID.
pub fn generate_session_id() -> SessionId {
    let id: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    SessionId::new(id)
}

/// Generate a random state parameter for CSRF protection.
pub fn generate_state() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}

/// Check if a session has expired.
pub fn is_session_expired(session: &Session, now: DateTime<Utc>) -> bool {
    session.expires_at <= now
}

/// Calculate session expiry from creation time and TTL.
pub fn calculate_expiry(created_at: DateTime<Utc>, ttl: Duration) -> DateTime<Utc> {
    created_at + ttl
}

/// Extract username from email if no name provided.
pub fn email_to_name(email: &str) -> String {
    match email.split('@').next() {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => "User".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::OidcProvider;

    #[test]
    fn generate_session_id_produces_32_char_alphanumeric() {
        let id = generate_session_id();
        assert_eq!(id.as_str().len(), 32);
        assert!(id.as_str().chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn generate_session_id_is_unique() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn generate_state_produces_32_char_string() {
        let state = generate_state();
        assert_eq!(state.len(), 32);
    }

    #[test]
    fn is_session_expired_returns_false_for_future_expiry() {
        let now = Utc::now();
        let session = Session {
            id: generate_session_id(),
            user_id: "user-1".to_string(),
            provider: OidcProvider::Google,
            created_at: now,
            expires_at: now + Duration::hours(1),
        };
        assert!(!is_session_expired(&session, now));
    }

    #[test]
    fn is_session_expired_returns_true_for_past_expiry() {
        let now = Utc::now();
        let session = Session {
            id: generate_session_id(),
            user_id: "user-1".to_string(),
            provider: OidcProvider::Google,
            created_at: now - Duration::hours(2),
            expires_at: now - Duration::hours(1),
        };
        assert!(is_session_expired(&session, now));
    }

    #[test]
    fn is_session_expired_returns_true_at_exact_expiry() {
        let now = Utc::now();
        let session = Session {
            id: generate_session_id(),
            user_id: "user-1".to_string(),
            provider: OidcProvider::Google,
            created_at: now - Duration::hours(1),
            expires_at: now,
        };
        assert!(is_session_expired(&session, now));
    }

    #[test]
    fn calculate_expiry_adds_ttl_to_created_at() {
        let created = Utc::now();
        let ttl = Duration::days(7);
        let expiry = calculate_expiry(created, ttl);
        assert_eq!(expiry, created + ttl);
    }

    #[test]
    fn email_to_name_extracts_username() {
        assert_eq!(email_to_name("john.doe@example.com"), "john.doe");
        assert_eq!(email_to_name("alice@test.org"), "alice");
    }

    #[test]
    fn email_to_name_handles_invalid_email() {
        assert_eq!(email_to_name("no-at-sign"), "no-at-sign");
        assert_eq!(email_to_name(""), "User");
    }
}
