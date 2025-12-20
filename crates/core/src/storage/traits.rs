use async_trait::async_trait;
use uuid::Uuid;

use crate::calendar::{Calendar, CalendarEntry, CalendarMembership, CalendarRole, User};

use super::{DateRange, Result};

/// Repository for calendar entry operations.
#[async_trait]
pub trait EntryRepository: Send + Sync {
    /// Gets an entry by its ID.
    async fn get_entry(&self, id: Uuid) -> Result<Option<CalendarEntry>>;

    /// Gets all entries for a calendar within a date range.
    async fn get_entries_by_calendar(
        &self,
        calendar_id: Uuid,
        date_range: DateRange,
    ) -> Result<Vec<CalendarEntry>>;

    /// Creates a new entry.
    async fn create_entry(&self, entry: &CalendarEntry) -> Result<()>;

    /// Updates an existing entry.
    async fn update_entry(&self, entry: &CalendarEntry) -> Result<()>;

    /// Deletes an entry by its ID.
    async fn delete_entry(&self, id: Uuid) -> Result<()>;
}

/// Repository for calendar operations.
#[async_trait]
pub trait CalendarRepository: Send + Sync {
    /// Gets a calendar by its ID.
    async fn get_calendar(&self, id: Uuid) -> Result<Option<Calendar>>;

    /// Creates a new calendar.
    async fn create_calendar(&self, calendar: &Calendar) -> Result<()>;

    /// Updates an existing calendar.
    async fn update_calendar(&self, calendar: &Calendar) -> Result<()>;

    /// Deletes a calendar by its ID.
    async fn delete_calendar(&self, id: Uuid) -> Result<()>;
}

/// Repository for user operations.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Gets a user by their ID.
    async fn get_user(&self, id: Uuid) -> Result<Option<User>>;

    /// Gets a user by their email address.
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;

    /// Creates a new user.
    async fn create_user(&self, user: &User) -> Result<()>;

    /// Updates an existing user.
    async fn update_user(&self, user: &User) -> Result<()>;
}

/// Repository for calendar membership operations.
#[async_trait]
pub trait MembershipRepository: Send + Sync {
    /// Gets a membership by calendar and user IDs.
    async fn get_membership(
        &self,
        calendar_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CalendarMembership>>;

    /// Gets all calendars for a user with their roles.
    async fn get_calendars_for_user(&self, user_id: Uuid) -> Result<Vec<(Calendar, CalendarRole)>>;

    /// Gets all users for a calendar with their roles.
    async fn get_users_for_calendar(&self, calendar_id: Uuid) -> Result<Vec<(User, CalendarRole)>>;

    /// Creates a new membership.
    async fn create_membership(&self, membership: &CalendarMembership) -> Result<()>;

    /// Deletes a membership.
    async fn delete_membership(&self, calendar_id: Uuid, user_id: Uuid) -> Result<()>;
}
