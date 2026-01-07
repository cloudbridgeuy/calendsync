//! SQLite session storage implementation.

use async_trait::async_trait;
use calendsync_core::auth::{
    AuthError, AuthFlowState, OidcProvider, Result, Session, SessionId, SessionRepository,
};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

/// SQLite-backed session storage.
pub struct SessionStore {
    pool: SqlitePool,
}

impl SessionStore {
    /// Creates a new SQLite session store.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Runs database migrations to create required tables.
    pub async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

            CREATE TABLE IF NOT EXISTS auth_flows (
                state TEXT PRIMARY KEY,
                pkce_verifier TEXT NOT NULL,
                provider TEXT NOT NULL,
                created_at TEXT NOT NULL,
                return_to TEXT,
                redirect_uri TEXT
            );
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl SessionRepository for SessionStore {
    async fn create_session(&self, session: &Session) -> Result<()> {
        sqlx::query(
            "INSERT INTO sessions (id, user_id, provider, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(session.id.as_str())
        .bind(&session.user_id)
        .bind(session.provider.to_string())
        .bind(session.created_at.to_rfc3339())
        .bind(session.expires_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn get_session(&self, id: &SessionId) -> Result<Option<Session>> {
        let row = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, user_id, provider, created_at, expires_at FROM sessions WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

        match row {
            Some((id, user_id, provider, created_at, expires_at)) => {
                let provider = match provider.as_str() {
                    "google" => OidcProvider::Google,
                    "apple" => OidcProvider::Apple,
                    _ => {
                        return Err(AuthError::Storage(format!(
                            "Unknown provider: {}",
                            provider
                        )))
                    }
                };

                Ok(Some(Session {
                    id: SessionId::new(id),
                    user_id,
                    provider,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map_err(|e| AuthError::Storage(e.to_string()))?
                        .with_timezone(&Utc),
                    expires_at: DateTime::parse_from_rfc3339(&expires_at)
                        .map_err(|e| AuthError::Storage(e.to_string()))?
                        .with_timezone(&Utc),
                }))
            }
            None => Ok(None),
        }
    }

    async fn delete_session(&self, id: &SessionId) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn delete_user_sessions(&self, user_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn store_auth_flow(&self, state: &str, flow: &AuthFlowState) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO auth_flows (state, pkce_verifier, provider, created_at, return_to, redirect_uri) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(state)
        .bind(&flow.pkce_verifier)
        .bind(flow.provider.to_string())
        .bind(flow.created_at.to_rfc3339())
        .bind(&flow.return_to)
        .bind(&flow.redirect_uri)
        .execute(&self.pool)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

        Ok(())
    }

    async fn peek_auth_flow(&self, state: &str) -> Result<Option<AuthFlowState>> {
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>)>(
            "SELECT pkce_verifier, provider, created_at, return_to, redirect_uri FROM auth_flows WHERE state = ?",
        )
        .bind(state)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

        match row {
            Some((pkce_verifier, provider, created_at, return_to, redirect_uri)) => {
                let provider = match provider.as_str() {
                    "google" => OidcProvider::Google,
                    "apple" => OidcProvider::Apple,
                    _ => {
                        return Err(AuthError::Storage(format!(
                            "Unknown provider: {}",
                            provider
                        )))
                    }
                };

                Ok(Some(AuthFlowState {
                    pkce_verifier,
                    provider,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map_err(|e| AuthError::Storage(e.to_string()))?
                        .with_timezone(&Utc),
                    return_to,
                    redirect_uri,
                }))
            }
            None => Ok(None),
        }
    }

    async fn take_auth_flow(&self, state: &str) -> Result<Option<AuthFlowState>> {
        // Begin transaction to make SELECT and DELETE atomic, preventing replay attacks
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Fetch the auth flow
        let row = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>)>(
            "SELECT pkce_verifier, provider, created_at, return_to, redirect_uri FROM auth_flows WHERE state = ?",
        )
        .bind(state)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Delete the auth flow if it exists
        if row.is_some() {
            sqlx::query("DELETE FROM auth_flows WHERE state = ?")
                .bind(state)
                .execute(&mut *tx)
                .await
                .map_err(|e| AuthError::Storage(e.to_string()))?;
        }

        // Commit the transaction
        tx.commit()
            .await
            .map_err(|e| AuthError::Storage(e.to_string()))?;

        // Process the result
        match row {
            Some((pkce_verifier, provider, created_at, return_to, redirect_uri)) => {
                let provider = match provider.as_str() {
                    "google" => OidcProvider::Google,
                    "apple" => OidcProvider::Apple,
                    _ => {
                        return Err(AuthError::Storage(format!(
                            "Unknown provider: {}",
                            provider
                        )))
                    }
                };

                Ok(Some(AuthFlowState {
                    pkce_verifier,
                    provider,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map_err(|e| AuthError::Storage(e.to_string()))?
                        .with_timezone(&Utc),
                    return_to,
                    redirect_uri,
                }))
            }
            None => Ok(None),
        }
    }
}
