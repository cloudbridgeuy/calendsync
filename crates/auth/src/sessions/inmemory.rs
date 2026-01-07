//! In-memory session storage for testing with auth-mock.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use calendsync_core::auth::{AuthFlowState, Result, Session, SessionId, SessionRepository};

/// In-memory session store for development and testing.
///
/// Stores sessions and auth flow state in HashMaps wrapped in `Arc<RwLock<_>>`.
/// Data is not persisted and will be lost when the store is dropped.
#[derive(Debug, Clone)]
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    auth_flows: Arc<RwLock<HashMap<String, AuthFlowState>>>,
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStore {
    /// Creates a new empty in-memory session store.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            auth_flows: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl SessionRepository for SessionStore {
    async fn create_session(&self, session: &Session) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.as_str().to_string(), session.clone());
        Ok(())
    }

    async fn get_session(&self, id: &SessionId) -> Result<Option<Session>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(id.as_str()).cloned())
    }

    async fn delete_session(&self, id: &SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(id.as_str());
        Ok(())
    }

    async fn delete_user_sessions(&self, user_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, s| s.user_id != user_id);
        Ok(())
    }

    async fn store_auth_flow(&self, state: &str, flow: &AuthFlowState) -> Result<()> {
        let mut flows = self.auth_flows.write().await;
        flows.insert(state.to_string(), flow.clone());
        Ok(())
    }

    async fn peek_auth_flow(&self, state: &str) -> Result<Option<AuthFlowState>> {
        let flows = self.auth_flows.read().await;
        Ok(flows.get(state).cloned())
    }

    async fn take_auth_flow(&self, state: &str) -> Result<Option<AuthFlowState>> {
        let mut flows = self.auth_flows.write().await;
        Ok(flows.remove(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use calendsync_core::auth::OidcProvider;
    use chrono::Utc;

    fn create_test_session(id: &str, user_id: &str) -> Session {
        Session {
            id: SessionId::new(id.to_string()),
            user_id: user_id.to_string(),
            provider: OidcProvider::Google,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
        }
    }

    fn create_test_auth_flow() -> AuthFlowState {
        AuthFlowState {
            pkce_verifier: "test-verifier".to_string(),
            provider: OidcProvider::Google,
            created_at: Utc::now(),
            return_to: None,
            redirect_uri: None,
        }
    }

    // ==================== Session CRUD Tests ====================

    #[tokio::test]
    async fn test_session_create_and_get() {
        let store = SessionStore::new();
        let session = create_test_session("session-1", "user-123");

        store.create_session(&session).await.unwrap();

        let retrieved = store
            .get_session(&SessionId::new("session-1".to_string()))
            .await
            .unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id.as_str(), "session-1");
        assert_eq!(retrieved.user_id, "user-123");
    }

    #[tokio::test]
    async fn test_session_get_nonexistent() {
        let store = SessionStore::new();

        let result = store
            .get_session(&SessionId::new("nonexistent".to_string()))
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_session_delete() {
        let store = SessionStore::new();
        let session = create_test_session("session-1", "user-123");

        store.create_session(&session).await.unwrap();
        store
            .delete_session(&SessionId::new("session-1".to_string()))
            .await
            .unwrap();

        let retrieved = store
            .get_session(&SessionId::new("session-1".to_string()))
            .await
            .unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_session_delete_nonexistent() {
        let store = SessionStore::new();

        // Should not error when deleting nonexistent session
        let result = store
            .delete_session(&SessionId::new("nonexistent".to_string()))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_user_sessions() {
        let store = SessionStore::new();

        // Create multiple sessions for the same user
        let session1 = create_test_session("session-1", "user-123");
        let session2 = create_test_session("session-2", "user-123");
        let session3 = create_test_session("session-3", "user-456");

        store.create_session(&session1).await.unwrap();
        store.create_session(&session2).await.unwrap();
        store.create_session(&session3).await.unwrap();

        // Delete all sessions for user-123
        store.delete_user_sessions("user-123").await.unwrap();

        // Verify user-123 sessions are deleted
        assert!(store
            .get_session(&SessionId::new("session-1".to_string()))
            .await
            .unwrap()
            .is_none());
        assert!(store
            .get_session(&SessionId::new("session-2".to_string()))
            .await
            .unwrap()
            .is_none());

        // Verify user-456 session is preserved
        assert!(store
            .get_session(&SessionId::new("session-3".to_string()))
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn test_delete_user_sessions_nonexistent_user() {
        let store = SessionStore::new();

        // Should not error when deleting sessions for nonexistent user
        let result = store.delete_user_sessions("nonexistent-user").await;
        assert!(result.is_ok());
    }

    // ==================== Auth Flow Tests ====================

    #[tokio::test]
    async fn test_auth_flow_store_and_take() {
        let store = SessionStore::new();
        let flow = create_test_auth_flow();

        store.store_auth_flow("state-abc", &flow).await.unwrap();

        let retrieved = store.take_auth_flow("state-abc").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.pkce_verifier, "test-verifier");

        // Should be gone after taking
        let second_take = store.take_auth_flow("state-abc").await.unwrap();
        assert!(second_take.is_none());
    }

    #[tokio::test]
    async fn test_auth_flow_take_nonexistent() {
        let store = SessionStore::new();

        let result = store.take_auth_flow("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_auth_flow_peek() {
        let store = SessionStore::new();
        let flow = AuthFlowState {
            pkce_verifier: "test-verifier".to_string(),
            provider: OidcProvider::Google,
            created_at: Utc::now(),
            return_to: None,
            redirect_uri: Some("calendsync://auth/callback".to_string()),
        };

        store.store_auth_flow("state-abc", &flow).await.unwrap();

        // Peek should return the flow without consuming it
        let peeked = store.peek_auth_flow("state-abc").await.unwrap();
        assert!(peeked.is_some());
        let peeked = peeked.unwrap();
        assert_eq!(peeked.pkce_verifier, "test-verifier");
        assert_eq!(
            peeked.redirect_uri,
            Some("calendsync://auth/callback".to_string())
        );

        // Peek again - should still be there
        let peeked_again = store.peek_auth_flow("state-abc").await.unwrap();
        assert!(peeked_again.is_some());

        // Take should still work after peek
        let taken = store.take_auth_flow("state-abc").await.unwrap();
        assert!(taken.is_some());

        // Now it should be gone
        let gone = store.peek_auth_flow("state-abc").await.unwrap();
        assert!(gone.is_none());
    }

    #[tokio::test]
    async fn test_auth_flow_peek_nonexistent() {
        let store = SessionStore::new();

        let result = store.peek_auth_flow("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_auth_flow_overwrite() {
        let store = SessionStore::new();

        let flow1 = AuthFlowState {
            pkce_verifier: "verifier-1".to_string(),
            provider: OidcProvider::Google,
            created_at: Utc::now(),
            return_to: None,
            redirect_uri: None,
        };
        let flow2 = AuthFlowState {
            pkce_verifier: "verifier-2".to_string(),
            provider: OidcProvider::Apple,
            created_at: Utc::now(),
            return_to: Some("/calendar/abc".to_string()),
            redirect_uri: None,
        };

        store.store_auth_flow("same-state", &flow1).await.unwrap();
        store.store_auth_flow("same-state", &flow2).await.unwrap();

        let retrieved = store.take_auth_flow("same-state").await.unwrap().unwrap();
        assert_eq!(retrieved.pkce_verifier, "verifier-2");
        assert_eq!(retrieved.provider, OidcProvider::Apple);
    }

    // ==================== Clone Tests ====================

    #[tokio::test]
    async fn test_clone_shares_state() {
        let store = SessionStore::new();
        let clone = store.clone();

        let session = create_test_session("session-1", "user-123");
        store.create_session(&session).await.unwrap();

        // Clone should see the same session
        let retrieved = clone
            .get_session(&SessionId::new("session-1".to_string()))
            .await
            .unwrap();
        assert!(retrieved.is_some());
    }
}
