//! SQLite repository implementation.
//!
//! Implements the repository traits from `calendsync_core::storage` using SQLite.

use async_trait::async_trait;
use tokio_rusqlite::Connection;
use uuid::Uuid;

use calendsync_core::calendar::{Calendar, CalendarEntry, CalendarMembership, CalendarRole, User};
use calendsync_core::storage::{
    CalendarRepository, DateRange, EntryRepository, MembershipRepository, RepositoryError, Result,
    UserRepository,
};

use super::conversions::{
    entry_kind_to_json, format_date, format_datetime, role_to_string, row_to_calendar,
    row_to_calendar_with_role, row_to_entry, row_to_membership, row_to_user, row_to_user_with_role,
};
use super::error::map_tokio_rusqlite_error_with_id;
use super::schema;

/// Helper to wrap rusqlite errors for tokio_rusqlite closures.
fn wrap_err(e: rusqlite::Error) -> tokio_rusqlite::Error {
    tokio_rusqlite::Error::Rusqlite(e)
}

/// SQLite-based repository implementation.
///
/// Provides async access to SQLite storage for all entity types.
pub struct SqliteRepository {
    conn: Connection,
}

impl SqliteRepository {
    /// Creates a new repository with a file-based database.
    ///
    /// The database file will be created if it doesn't exist.
    /// Schema tables are created automatically.
    pub async fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)
            .await
            .map_err(|e| RepositoryError::ConnectionFailed(e.to_string()))?;

        Self::init_schema(&conn).await?;

        Ok(Self { conn })
    }

    /// Creates a new repository with an in-memory database.
    ///
    /// Useful for testing - data is lost when the connection is dropped.
    pub async fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .await
            .map_err(|e| RepositoryError::ConnectionFailed(e.to_string()))?;

        Self::init_schema(&conn).await?;

        Ok(Self { conn })
    }

    /// Initialize the database schema.
    async fn init_schema(conn: &Connection) -> Result<()> {
        conn.call(|conn| {
            conn.execute_batch(schema::CREATE_TABLES)
                .map_err(wrap_err)?;
            Ok(())
        })
        .await
        .map_err(|e| RepositoryError::QueryFailed(e.to_string()))
    }
}

// ============================================================================
// EntryRepository implementation
// ============================================================================

#[async_trait]
impl EntryRepository for SqliteRepository {
    async fn get_entry(&self, id: Uuid) -> Result<Option<CalendarEntry>> {
        let id_str = id.to_string();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(schema::SELECT_ENTRY_BY_ID).map_err(wrap_err)?;
                match stmt.query_row([&id_str], row_to_entry) {
                    Ok(entry) => Ok(Some(entry)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(wrap_err(e)),
                }
            })
            .await
            .map_err(|e| map_tokio_rusqlite_error_with_id(e, "CalendarEntry", id.to_string()))
    }

    async fn get_entries_by_calendar(
        &self,
        calendar_id: Uuid,
        date_range: DateRange,
    ) -> Result<Vec<CalendarEntry>> {
        let calendar_id_str = calendar_id.to_string();
        let start_str = format_date(&date_range.start);
        let end_str = format_date(&date_range.end);

        self.conn
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(schema::SELECT_ENTRIES_BY_CALENDAR_AND_DATE_RANGE)
                    .map_err(wrap_err)?;
                let rows = stmt
                    .query_map([&calendar_id_str, &start_str, &end_str], row_to_entry)
                    .map_err(wrap_err)?;

                let mut entries = Vec::new();
                for row_result in rows {
                    entries.push(row_result.map_err(wrap_err)?);
                }
                Ok(entries)
            })
            .await
            .map_err(|e| RepositoryError::QueryFailed(e.to_string()))
    }

    async fn create_entry(&self, entry: &CalendarEntry) -> Result<()> {
        let id = entry.id.to_string();
        let calendar_id = entry.calendar_id.to_string();
        let title = entry.title.clone();
        let description = entry.description.clone();
        let location = entry.location.clone();
        let kind_json = entry_kind_to_json(&entry.kind)?;
        let start_date = format_date(&entry.start_date);
        let end_date = format_date(&entry.end_date);
        let color = entry.color.clone();
        let created_at = format_datetime(&entry.created_at);
        let updated_at = format_datetime(&entry.updated_at);
        let entry_id = entry.id.to_string();

        self.conn
            .call(move |conn| {
                conn.execute(
                    schema::INSERT_ENTRY,
                    rusqlite::params![
                        id,
                        calendar_id,
                        title,
                        description,
                        location,
                        kind_json,
                        start_date,
                        end_date,
                        color,
                        created_at,
                        updated_at
                    ],
                )
                .map_err(wrap_err)?;
                Ok(())
            })
            .await
            .map_err(|e| map_tokio_rusqlite_error_with_id(e, "CalendarEntry", entry_id))
    }

    async fn update_entry(&self, entry: &CalendarEntry) -> Result<()> {
        let id = entry.id.to_string();
        let title = entry.title.clone();
        let description = entry.description.clone();
        let location = entry.location.clone();
        let kind_json = entry_kind_to_json(&entry.kind)?;
        let start_date = format_date(&entry.start_date);
        let end_date = format_date(&entry.end_date);
        let color = entry.color.clone();
        let updated_at = format_datetime(&entry.updated_at);
        let entry_id = entry.id.to_string();

        self.conn
            .call(move |conn| {
                let rows = conn
                    .execute(
                        schema::UPDATE_ENTRY,
                        rusqlite::params![
                            id,
                            title,
                            description,
                            location,
                            kind_json,
                            start_date,
                            end_date,
                            color,
                            updated_at
                        ],
                    )
                    .map_err(wrap_err)?;
                if rows == 0 {
                    Err(wrap_err(rusqlite::Error::QueryReturnedNoRows))
                } else {
                    Ok(())
                }
            })
            .await
            .map_err(|e| map_tokio_rusqlite_error_with_id(e, "CalendarEntry", entry_id))
    }

    async fn delete_entry(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        let entry_id = id.to_string();

        self.conn
            .call(move |conn| {
                let rows = conn
                    .execute(schema::DELETE_ENTRY, [&id_str])
                    .map_err(wrap_err)?;
                if rows == 0 {
                    Err(wrap_err(rusqlite::Error::QueryReturnedNoRows))
                } else {
                    Ok(())
                }
            })
            .await
            .map_err(|e| map_tokio_rusqlite_error_with_id(e, "CalendarEntry", entry_id))
    }
}

// ============================================================================
// CalendarRepository implementation
// ============================================================================

#[async_trait]
impl CalendarRepository for SqliteRepository {
    async fn get_calendar(&self, id: Uuid) -> Result<Option<Calendar>> {
        let id_str = id.to_string();

        self.conn
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(schema::SELECT_CALENDAR_BY_ID)
                    .map_err(wrap_err)?;
                match stmt.query_row([&id_str], row_to_calendar) {
                    Ok(calendar) => Ok(Some(calendar)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(wrap_err(e)),
                }
            })
            .await
            .map_err(|e| map_tokio_rusqlite_error_with_id(e, "Calendar", id.to_string()))
    }

    async fn create_calendar(&self, calendar: &Calendar) -> Result<()> {
        let id = calendar.id.to_string();
        let name = calendar.name.clone();
        let color = calendar.color.clone();
        let description = calendar.description.clone();
        let created_at = format_datetime(&calendar.created_at);
        let updated_at = format_datetime(&calendar.updated_at);
        let calendar_id = calendar.id.to_string();

        self.conn
            .call(move |conn| {
                conn.execute(
                    schema::INSERT_CALENDAR,
                    rusqlite::params![id, name, color, description, created_at, updated_at],
                )
                .map_err(wrap_err)?;
                Ok(())
            })
            .await
            .map_err(|e| map_tokio_rusqlite_error_with_id(e, "Calendar", calendar_id))
    }

    async fn update_calendar(&self, calendar: &Calendar) -> Result<()> {
        let id = calendar.id.to_string();
        let name = calendar.name.clone();
        let color = calendar.color.clone();
        let description = calendar.description.clone();
        let updated_at = format_datetime(&calendar.updated_at);
        let calendar_id = calendar.id.to_string();

        self.conn
            .call(move |conn| {
                let rows = conn
                    .execute(
                        schema::UPDATE_CALENDAR,
                        rusqlite::params![name, color, description, updated_at, id],
                    )
                    .map_err(wrap_err)?;
                if rows == 0 {
                    Err(wrap_err(rusqlite::Error::QueryReturnedNoRows))
                } else {
                    Ok(())
                }
            })
            .await
            .map_err(|e| map_tokio_rusqlite_error_with_id(e, "Calendar", calendar_id))
    }

    async fn delete_calendar(&self, id: Uuid) -> Result<()> {
        let id_str = id.to_string();
        let calendar_id = id.to_string();

        self.conn
            .call(move |conn| {
                let rows = conn
                    .execute(schema::DELETE_CALENDAR, [&id_str])
                    .map_err(wrap_err)?;
                if rows == 0 {
                    Err(wrap_err(rusqlite::Error::QueryReturnedNoRows))
                } else {
                    Ok(())
                }
            })
            .await
            .map_err(|e| map_tokio_rusqlite_error_with_id(e, "Calendar", calendar_id))
    }
}

// ============================================================================
// UserRepository implementation
// ============================================================================

#[async_trait]
impl UserRepository for SqliteRepository {
    async fn get_user(&self, id: Uuid) -> Result<Option<User>> {
        let id_str = id.to_string();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(schema::SELECT_USER_BY_ID).map_err(wrap_err)?;
                match stmt.query_row([&id_str], row_to_user) {
                    Ok(user) => Ok(Some(user)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(wrap_err(e)),
                }
            })
            .await
            .map_err(|e| map_tokio_rusqlite_error_with_id(e, "User", id.to_string()))
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let email = email.to_string();

        self.conn
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(schema::SELECT_USER_BY_EMAIL)
                    .map_err(wrap_err)?;
                match stmt.query_row([&email], row_to_user) {
                    Ok(user) => Ok(Some(user)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(wrap_err(e)),
                }
            })
            .await
            .map_err(|e| RepositoryError::QueryFailed(e.to_string()))
    }

    async fn get_user_by_provider(
        &self,
        provider: &str,
        provider_subject: &str,
    ) -> Result<Option<User>> {
        let provider = provider.to_string();
        let provider_subject = provider_subject.to_string();

        self.conn
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(schema::SELECT_USER_BY_PROVIDER)
                    .map_err(wrap_err)?;
                match stmt.query_row([&provider, &provider_subject], row_to_user) {
                    Ok(user) => Ok(Some(user)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(wrap_err(e)),
                }
            })
            .await
            .map_err(|e| RepositoryError::QueryFailed(e.to_string()))
    }

    async fn create_user(&self, user: &User) -> Result<()> {
        let id = user.id.to_string();
        let name = user.name.clone();
        let email = user.email.clone();
        let provider = user.provider.clone();
        let provider_subject = user.provider_subject.clone();
        let created_at = format_datetime(&user.created_at);
        let updated_at = format_datetime(&user.updated_at);
        let user_id = user.id.to_string();

        self.conn
            .call(move |conn| {
                conn.execute(
                    schema::INSERT_USER,
                    rusqlite::params![
                        id,
                        name,
                        email,
                        provider,
                        provider_subject,
                        created_at,
                        updated_at
                    ],
                )
                .map_err(wrap_err)?;
                Ok(())
            })
            .await
            .map_err(|e| map_tokio_rusqlite_error_with_id(e, "User", user_id))
    }

    async fn update_user(&self, user: &User) -> Result<()> {
        let id = user.id.to_string();
        let name = user.name.clone();
        let email = user.email.clone();
        let provider = user.provider.clone();
        let provider_subject = user.provider_subject.clone();
        let updated_at = format_datetime(&user.updated_at);
        let user_id = user.id.to_string();

        self.conn
            .call(move |conn| {
                let rows = conn
                    .execute(
                        schema::UPDATE_USER,
                        rusqlite::params![id, name, email, provider, provider_subject, updated_at],
                    )
                    .map_err(wrap_err)?;
                if rows == 0 {
                    Err(wrap_err(rusqlite::Error::QueryReturnedNoRows))
                } else {
                    Ok(())
                }
            })
            .await
            .map_err(|e| map_tokio_rusqlite_error_with_id(e, "User", user_id))
    }
}

// ============================================================================
// MembershipRepository implementation
// ============================================================================

#[async_trait]
impl MembershipRepository for SqliteRepository {
    async fn get_membership(
        &self,
        calendar_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CalendarMembership>> {
        let calendar_id_str = calendar_id.to_string();
        let user_id_str = user_id.to_string();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(schema::SELECT_MEMBERSHIP).map_err(wrap_err)?;
                match stmt.query_row([&calendar_id_str, &user_id_str], row_to_membership) {
                    Ok(membership) => Ok(Some(membership)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(wrap_err(e)),
                }
            })
            .await
            .map_err(|e| RepositoryError::QueryFailed(e.to_string()))
    }

    async fn get_calendars_for_user(&self, user_id: Uuid) -> Result<Vec<(Calendar, CalendarRole)>> {
        let user_id_str = user_id.to_string();

        self.conn
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(schema::SELECT_CALENDARS_FOR_USER)
                    .map_err(wrap_err)?;
                let rows = stmt
                    .query_map([&user_id_str], row_to_calendar_with_role)
                    .map_err(wrap_err)?;

                let mut results = Vec::new();
                for row_result in rows {
                    results.push(row_result.map_err(wrap_err)?);
                }
                Ok(results)
            })
            .await
            .map_err(|e| RepositoryError::QueryFailed(e.to_string()))
    }

    async fn get_users_for_calendar(&self, calendar_id: Uuid) -> Result<Vec<(User, CalendarRole)>> {
        let calendar_id_str = calendar_id.to_string();

        self.conn
            .call(move |conn| {
                let mut stmt = conn
                    .prepare(schema::SELECT_USERS_FOR_CALENDAR)
                    .map_err(wrap_err)?;
                let rows = stmt
                    .query_map([&calendar_id_str], row_to_user_with_role)
                    .map_err(wrap_err)?;

                let mut results = Vec::new();
                for row_result in rows {
                    results.push(row_result.map_err(wrap_err)?);
                }
                Ok(results)
            })
            .await
            .map_err(|e| RepositoryError::QueryFailed(e.to_string()))
    }

    async fn create_membership(&self, membership: &CalendarMembership) -> Result<()> {
        let calendar_id = membership.calendar_id.to_string();
        let user_id = membership.user_id.to_string();
        let role = role_to_string(&membership.role).to_string();
        let created_at = format_datetime(&membership.created_at);
        let updated_at = format_datetime(&membership.updated_at);

        self.conn
            .call(move |conn| {
                conn.execute(
                    schema::INSERT_MEMBERSHIP,
                    rusqlite::params![calendar_id, user_id, role, created_at, updated_at],
                )
                .map_err(wrap_err)?;
                Ok(())
            })
            .await
            .map_err(|e| RepositoryError::QueryFailed(e.to_string()))
    }

    async fn delete_membership(&self, calendar_id: Uuid, user_id: Uuid) -> Result<()> {
        let calendar_id_str = calendar_id.to_string();
        let user_id_str = user_id.to_string();
        let membership_id = format!("{}:{}", calendar_id, user_id);

        self.conn
            .call(move |conn| {
                let rows = conn
                    .execute(schema::DELETE_MEMBERSHIP, [&calendar_id_str, &user_id_str])
                    .map_err(wrap_err)?;
                if rows == 0 {
                    Err(wrap_err(rusqlite::Error::QueryReturnedNoRows))
                } else {
                    Ok(())
                }
            })
            .await
            .map_err(|e| map_tokio_rusqlite_error_with_id(e, "CalendarMembership", membership_id))
    }
}
