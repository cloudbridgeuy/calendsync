//! Redis session storage implementation.

use async_trait::async_trait;
use calendsync_core::auth::{
    AuthError, AuthFlowState, Result, Session, SessionId, SessionRepository,
};
use fred::prelude::*;
use std::time::Duration;

/// Redis-backed session storage.
pub struct SessionStore {
    pool: Pool,
    session_ttl: Duration,
    flow_ttl: Duration,
}

impl SessionStore {
    /// Creates a new Redis session store.
    ///
    /// # Arguments
    ///
    /// * `pool` - Redis connection pool
    /// * `session_ttl` - TTL for session data
    pub fn new(pool: Pool, session_ttl: Duration) -> Self {
        Self {
            pool,
            session_ttl,
            flow_ttl: Duration::from_secs(600), // 10 minutes for auth flow
        }
    }

    fn session_key(id: &SessionId) -> String {
        format!("session:{}", id)
    }

    fn user_sessions_key(user_id: &str) -> String {
        format!("user_sessions:{}", user_id)
    }

    fn flow_key(state: &str) -> String {
        format!("auth_flow:{}", state)
    }
}

#[async_trait]
impl SessionRepository for SessionStore {
    async fn create_session(&self, session: &Session) -> Result<()> {
        let key = Self::session_key(&session.id);
        let value =
            serde_json::to_string(session).map_err(|e| AuthError::Storage(e.to_string()))?;

        let ttl_secs = self.session_ttl.as_secs() as i64;

        self.pool
            .set::<(), _, _>(&key, &value, Some(Expiration::EX(ttl_secs)), None, false)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Track session in user's session set
        let user_key = Self::user_sessions_key(&session.user_id);
        self.pool
            .sadd::<(), _, _>(&user_key, session.id.as_str())
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn get_session(&self, id: &SessionId) -> Result<Option<Session>> {
        let key = Self::session_key(id);
        let value: Option<String> = self
            .pool
            .get(&key)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        match value {
            Some(json) => {
                let session: Session =
                    serde_json::from_str(&json).map_err(|e| AuthError::Storage(e.to_string()))?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    async fn delete_session(&self, id: &SessionId) -> Result<()> {
        // Get session first to find user_id
        if let Some(session) = self.get_session(id).await? {
            let key = Self::session_key(id);
            self.pool
                .del::<(), _>(&key)
                .await
                .map_err(|e| AuthError::Storage(e.to_string()))?;

            // Remove from user's session set
            let user_key = Self::user_sessions_key(&session.user_id);
            self.pool
                .srem::<(), _, _>(&user_key, id.as_str())
                .await
                .map_err(|e| AuthError::Storage(e.to_string()))?;
        }

        Ok(())
    }

    async fn delete_user_sessions(&self, user_id: &str) -> Result<()> {
        let user_key = Self::user_sessions_key(user_id);

        // Get all session IDs for user
        let session_ids: Vec<String> = self
            .pool
            .smembers(&user_key)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Delete each session
        for id in &session_ids {
            let key = format!("session:{}", id);
            self.pool
                .del::<(), _>(&key)
                .await
                .map_err(|e| AuthError::Storage(e.to_string()))?;
        }

        // Delete the user sessions set
        self.pool
            .del::<(), _>(&user_key)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn store_auth_flow(&self, state: &str, flow: &AuthFlowState) -> Result<()> {
        let key = Self::flow_key(state);
        let value = serde_json::to_string(flow).map_err(|e| AuthError::Storage(e.to_string()))?;

        let ttl_secs = self.flow_ttl.as_secs() as i64;

        self.pool
            .set::<(), _, _>(&key, &value, Some(Expiration::EX(ttl_secs)), None, false)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn take_auth_flow(&self, state: &str) -> Result<Option<AuthFlowState>> {
        let key = Self::flow_key(state);

        // Get and delete atomically
        let value: Option<String> = self
            .pool
            .getdel(&key)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        match value {
            Some(json) => {
                let flow: AuthFlowState =
                    serde_json::from_str(&json).map_err(|e| AuthError::Storage(e.to_string()))?;
                Ok(Some(flow))
            }
            None => Ok(None),
        }
    }
}
