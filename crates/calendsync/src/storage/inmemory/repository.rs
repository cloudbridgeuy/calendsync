//! In-memory repository implementation.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use calendsync_core::calendar::{Calendar, CalendarEntry, CalendarMembership, CalendarRole, User};
use calendsync_core::storage::{
    CalendarRepository, DateRange, EntryRepository, MembershipRepository, RepositoryError, Result,
    UserRepository,
};

/// In-memory storage backend for testing.
///
/// Uses HashMaps wrapped in `Arc<RwLock<_>>` for thread-safe access.
/// Data is not persisted and will be lost when the repository is dropped.
#[derive(Debug, Clone)]
pub struct InMemoryRepository {
    entries: Arc<RwLock<HashMap<Uuid, CalendarEntry>>>,
    calendars: Arc<RwLock<HashMap<Uuid, Calendar>>>,
    users: Arc<RwLock<HashMap<Uuid, User>>>,
    memberships: Arc<RwLock<HashMap<(Uuid, Uuid), CalendarMembership>>>,
}

impl Default for InMemoryRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryRepository {
    /// Creates a new empty in-memory repository.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            calendars: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            memberships: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl EntryRepository for InMemoryRepository {
    async fn get_entry(&self, id: Uuid) -> Result<Option<CalendarEntry>> {
        let entries = self.entries.read().await;
        Ok(entries.get(&id).cloned())
    }

    async fn get_entries_by_calendar(
        &self,
        calendar_id: Uuid,
        date_range: DateRange,
    ) -> Result<Vec<CalendarEntry>> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .filter(|e| e.calendar_id == calendar_id)
            .filter(|e| e.start_date <= date_range.end && e.end_date >= date_range.start)
            .cloned()
            .collect())
    }

    async fn create_entry(&self, entry: &CalendarEntry) -> Result<()> {
        let mut entries = self.entries.write().await;
        if entries.contains_key(&entry.id) {
            return Err(RepositoryError::AlreadyExists {
                entity_type: "CalendarEntry",
                id: entry.id.to_string(),
            });
        }
        entries.insert(entry.id, entry.clone());
        Ok(())
    }

    async fn update_entry(&self, entry: &CalendarEntry) -> Result<()> {
        let mut entries = self.entries.write().await;
        if !entries.contains_key(&entry.id) {
            return Err(RepositoryError::NotFound {
                entity_type: "CalendarEntry",
                id: entry.id.to_string(),
            });
        }
        entries.insert(entry.id, entry.clone());
        Ok(())
    }

    async fn delete_entry(&self, id: Uuid) -> Result<()> {
        let mut entries = self.entries.write().await;
        if entries.remove(&id).is_none() {
            return Err(RepositoryError::NotFound {
                entity_type: "CalendarEntry",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

#[async_trait]
impl CalendarRepository for InMemoryRepository {
    async fn get_calendar(&self, id: Uuid) -> Result<Option<Calendar>> {
        let calendars = self.calendars.read().await;
        Ok(calendars.get(&id).cloned())
    }

    async fn create_calendar(&self, calendar: &Calendar) -> Result<()> {
        let mut calendars = self.calendars.write().await;
        if calendars.contains_key(&calendar.id) {
            return Err(RepositoryError::AlreadyExists {
                entity_type: "Calendar",
                id: calendar.id.to_string(),
            });
        }
        calendars.insert(calendar.id, calendar.clone());
        Ok(())
    }

    async fn update_calendar(&self, calendar: &Calendar) -> Result<()> {
        let mut calendars = self.calendars.write().await;
        if !calendars.contains_key(&calendar.id) {
            return Err(RepositoryError::NotFound {
                entity_type: "Calendar",
                id: calendar.id.to_string(),
            });
        }
        calendars.insert(calendar.id, calendar.clone());
        Ok(())
    }

    async fn delete_calendar(&self, id: Uuid) -> Result<()> {
        let mut calendars = self.calendars.write().await;
        if calendars.remove(&id).is_none() {
            return Err(RepositoryError::NotFound {
                entity_type: "Calendar",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

#[async_trait]
impl UserRepository for InMemoryRepository {
    async fn get_user(&self, id: Uuid) -> Result<Option<User>> {
        let users = self.users.read().await;
        Ok(users.get(&id).cloned())
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let users = self.users.read().await;
        Ok(users.values().find(|user| user.email == email).cloned())
    }

    async fn create_user(&self, user: &User) -> Result<()> {
        let mut users = self.users.write().await;
        if users.contains_key(&user.id) {
            return Err(RepositoryError::AlreadyExists {
                entity_type: "User",
                id: user.id.to_string(),
            });
        }
        users.insert(user.id, user.clone());
        Ok(())
    }

    async fn update_user(&self, user: &User) -> Result<()> {
        let mut users = self.users.write().await;
        if !users.contains_key(&user.id) {
            return Err(RepositoryError::NotFound {
                entity_type: "User",
                id: user.id.to_string(),
            });
        }
        users.insert(user.id, user.clone());
        Ok(())
    }
}

#[async_trait]
impl MembershipRepository for InMemoryRepository {
    async fn get_membership(
        &self,
        calendar_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CalendarMembership>> {
        let memberships = self.memberships.read().await;
        Ok(memberships.get(&(calendar_id, user_id)).cloned())
    }

    async fn get_calendars_for_user(&self, user_id: Uuid) -> Result<Vec<(Calendar, CalendarRole)>> {
        let memberships = self.memberships.read().await;
        let calendars = self.calendars.read().await;

        let result: Vec<(Calendar, CalendarRole)> = memberships
            .values()
            .filter(|m| m.user_id == user_id)
            .filter_map(|m| calendars.get(&m.calendar_id).map(|c| (c.clone(), m.role)))
            .collect();

        Ok(result)
    }

    async fn get_users_for_calendar(&self, calendar_id: Uuid) -> Result<Vec<(User, CalendarRole)>> {
        let memberships = self.memberships.read().await;
        let users = self.users.read().await;

        let result: Vec<(User, CalendarRole)> = memberships
            .values()
            .filter(|m| m.calendar_id == calendar_id)
            .filter_map(|m| users.get(&m.user_id).map(|u| (u.clone(), m.role)))
            .collect();

        Ok(result)
    }

    async fn create_membership(&self, membership: &CalendarMembership) -> Result<()> {
        let mut memberships = self.memberships.write().await;
        let key = (membership.calendar_id, membership.user_id);
        if memberships.contains_key(&key) {
            return Err(RepositoryError::AlreadyExists {
                entity_type: "CalendarMembership",
                id: format!("{}:{}", membership.calendar_id, membership.user_id),
            });
        }
        memberships.insert(key, membership.clone());
        Ok(())
    }

    async fn delete_membership(&self, calendar_id: Uuid, user_id: Uuid) -> Result<()> {
        let mut memberships = self.memberships.write().await;
        let key = (calendar_id, user_id);
        if memberships.remove(&key).is_none() {
            return Err(RepositoryError::NotFound {
                entity_type: "CalendarMembership",
                id: format!("{calendar_id}:{user_id}"),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    // Helper to create test dates
    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    // ==================== Entry CRUD Tests ====================

    #[tokio::test]
    async fn test_entry_create_and_get() {
        let repo = InMemoryRepository::new();
        let calendar_id = Uuid::new_v4();
        let entry = CalendarEntry::all_day(calendar_id, "Test Event", date(2024, 6, 15));

        repo.create_entry(&entry).await.unwrap();

        let retrieved = repo.get_entry(entry.id).await.unwrap();
        assert_eq!(retrieved, Some(entry));
    }

    #[tokio::test]
    async fn test_entry_get_nonexistent() {
        let repo = InMemoryRepository::new();
        let result = repo.get_entry(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_entry_update() {
        let repo = InMemoryRepository::new();
        let calendar_id = Uuid::new_v4();
        let mut entry = CalendarEntry::all_day(calendar_id, "Original Title", date(2024, 6, 15));

        repo.create_entry(&entry).await.unwrap();

        entry.title = "Updated Title".to_string();
        repo.update_entry(&entry).await.unwrap();

        let retrieved = repo.get_entry(entry.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated Title");
    }

    #[tokio::test]
    async fn test_entry_update_nonexistent() {
        let repo = InMemoryRepository::new();
        let calendar_id = Uuid::new_v4();
        let entry = CalendarEntry::all_day(calendar_id, "Test", date(2024, 6, 15));

        let result = repo.update_entry(&entry).await;
        assert!(matches!(result, Err(RepositoryError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_entry_delete() {
        let repo = InMemoryRepository::new();
        let calendar_id = Uuid::new_v4();
        let entry = CalendarEntry::all_day(calendar_id, "Test Event", date(2024, 6, 15));

        repo.create_entry(&entry).await.unwrap();
        repo.delete_entry(entry.id).await.unwrap();

        let retrieved = repo.get_entry(entry.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_entry_delete_nonexistent() {
        let repo = InMemoryRepository::new();
        let result = repo.delete_entry(Uuid::new_v4()).await;
        assert!(matches!(result, Err(RepositoryError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_get_entries_by_calendar_with_date_range() {
        let repo = InMemoryRepository::new();
        let calendar_id = Uuid::new_v4();
        let other_calendar_id = Uuid::new_v4();

        // Create entries for the target calendar
        let entry1 = CalendarEntry::all_day(calendar_id, "Entry 1", date(2024, 6, 10));
        let entry2 = CalendarEntry::all_day(calendar_id, "Entry 2", date(2024, 6, 15));
        let entry3 = CalendarEntry::all_day(calendar_id, "Entry 3", date(2024, 6, 20));
        let entry4 = CalendarEntry::all_day(calendar_id, "Entry 4", date(2024, 6, 25));

        // Create entry for a different calendar
        let other_entry = CalendarEntry::all_day(other_calendar_id, "Other", date(2024, 6, 15));

        repo.create_entry(&entry1).await.unwrap();
        repo.create_entry(&entry2).await.unwrap();
        repo.create_entry(&entry3).await.unwrap();
        repo.create_entry(&entry4).await.unwrap();
        repo.create_entry(&other_entry).await.unwrap();

        // Query with date range that should include entries 2 and 3
        let date_range = DateRange::new(date(2024, 6, 12), date(2024, 6, 22)).unwrap();
        let entries = repo
            .get_entries_by_calendar(calendar_id, date_range)
            .await
            .unwrap();

        assert_eq!(entries.len(), 2);
        let titles: Vec<&str> = entries.iter().map(|e| e.title.as_str()).collect();
        assert!(titles.contains(&"Entry 2"));
        assert!(titles.contains(&"Entry 3"));
    }

    // ==================== Calendar CRUD Tests ====================

    #[tokio::test]
    async fn test_calendar_create_and_get() {
        let repo = InMemoryRepository::new();
        let calendar = Calendar::new("Work", "#3B82F6");

        repo.create_calendar(&calendar).await.unwrap();

        let retrieved = repo.get_calendar(calendar.id).await.unwrap();
        assert_eq!(retrieved, Some(calendar));
    }

    #[tokio::test]
    async fn test_calendar_get_nonexistent() {
        let repo = InMemoryRepository::new();
        let result = repo.get_calendar(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_calendar_update() {
        let repo = InMemoryRepository::new();
        let mut calendar = Calendar::new("Work", "#3B82F6");

        repo.create_calendar(&calendar).await.unwrap();

        calendar.name = "Personal".to_string();
        repo.update_calendar(&calendar).await.unwrap();

        let retrieved = repo.get_calendar(calendar.id).await.unwrap().unwrap();
        assert_eq!(retrieved.name, "Personal");
    }

    #[tokio::test]
    async fn test_calendar_update_nonexistent() {
        let repo = InMemoryRepository::new();
        let calendar = Calendar::new("Work", "#3B82F6");

        let result = repo.update_calendar(&calendar).await;
        assert!(matches!(result, Err(RepositoryError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_calendar_delete() {
        let repo = InMemoryRepository::new();
        let calendar = Calendar::new("Work", "#3B82F6");

        repo.create_calendar(&calendar).await.unwrap();
        repo.delete_calendar(calendar.id).await.unwrap();

        let retrieved = repo.get_calendar(calendar.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_calendar_delete_nonexistent() {
        let repo = InMemoryRepository::new();
        let result = repo.delete_calendar(Uuid::new_v4()).await;
        assert!(matches!(result, Err(RepositoryError::NotFound { .. })));
    }

    // ==================== User CRUD Tests ====================

    #[tokio::test]
    async fn test_user_create_and_get() {
        let repo = InMemoryRepository::new();
        let user = User::new("Alice", "alice@example.com");

        repo.create_user(&user).await.unwrap();

        let retrieved = repo.get_user(user.id).await.unwrap();
        assert_eq!(retrieved, Some(user));
    }

    #[tokio::test]
    async fn test_user_get_nonexistent() {
        let repo = InMemoryRepository::new();
        let result = repo.get_user(Uuid::new_v4()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_user_get_by_email() {
        let repo = InMemoryRepository::new();
        let user = User::new("Alice", "alice@example.com");

        repo.create_user(&user).await.unwrap();

        let retrieved = repo.get_user_by_email("alice@example.com").await.unwrap();
        assert_eq!(retrieved, Some(user));
    }

    #[tokio::test]
    async fn test_user_get_by_email_nonexistent() {
        let repo = InMemoryRepository::new();
        let result = repo
            .get_user_by_email("nonexistent@example.com")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_user_update() {
        let repo = InMemoryRepository::new();
        let mut user = User::new("Alice", "alice@example.com");

        repo.create_user(&user).await.unwrap();

        user.name = "Alice Smith".to_string();
        repo.update_user(&user).await.unwrap();

        let retrieved = repo.get_user(user.id).await.unwrap().unwrap();
        assert_eq!(retrieved.name, "Alice Smith");
    }

    #[tokio::test]
    async fn test_user_update_nonexistent() {
        let repo = InMemoryRepository::new();
        let user = User::new("Alice", "alice@example.com");

        let result = repo.update_user(&user).await;
        assert!(matches!(result, Err(RepositoryError::NotFound { .. })));
    }

    // ==================== Membership CRUD Tests ====================

    #[tokio::test]
    async fn test_membership_create_and_get() {
        let repo = InMemoryRepository::new();
        let calendar_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let membership = CalendarMembership::owner(calendar_id, user_id);

        repo.create_membership(&membership).await.unwrap();

        let retrieved = repo.get_membership(calendar_id, user_id).await.unwrap();
        assert_eq!(retrieved, Some(membership));
    }

    #[tokio::test]
    async fn test_membership_get_nonexistent() {
        let repo = InMemoryRepository::new();
        let result = repo
            .get_membership(Uuid::new_v4(), Uuid::new_v4())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_membership_delete() {
        let repo = InMemoryRepository::new();
        let calendar_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let membership = CalendarMembership::owner(calendar_id, user_id);

        repo.create_membership(&membership).await.unwrap();
        repo.delete_membership(calendar_id, user_id).await.unwrap();

        let retrieved = repo.get_membership(calendar_id, user_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_membership_delete_nonexistent() {
        let repo = InMemoryRepository::new();
        let result = repo.delete_membership(Uuid::new_v4(), Uuid::new_v4()).await;
        assert!(matches!(result, Err(RepositoryError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_get_calendars_for_user() {
        let repo = InMemoryRepository::new();

        // Create calendars
        let calendar1 = Calendar::new("Work", "#3B82F6");
        let calendar2 = Calendar::new("Personal", "#10B981");
        let calendar3 = Calendar::new("Other", "#F59E0B");

        repo.create_calendar(&calendar1).await.unwrap();
        repo.create_calendar(&calendar2).await.unwrap();
        repo.create_calendar(&calendar3).await.unwrap();

        // Create user
        let user = User::new("Alice", "alice@example.com");
        repo.create_user(&user).await.unwrap();

        // Create memberships
        let membership1 = CalendarMembership::owner(calendar1.id, user.id);
        let membership2 = CalendarMembership::writer(calendar2.id, user.id);

        repo.create_membership(&membership1).await.unwrap();
        repo.create_membership(&membership2).await.unwrap();

        // Get calendars for user
        let calendars = repo.get_calendars_for_user(user.id).await.unwrap();

        assert_eq!(calendars.len(), 2);
        let names: Vec<&str> = calendars.iter().map(|(c, _)| c.name.as_str()).collect();
        assert!(names.contains(&"Work"));
        assert!(names.contains(&"Personal"));
        assert!(!names.contains(&"Other"));

        // Check roles
        let work_entry = calendars.iter().find(|(c, _)| c.name == "Work").unwrap();
        assert_eq!(work_entry.1, CalendarRole::Owner);

        let personal_entry = calendars
            .iter()
            .find(|(c, _)| c.name == "Personal")
            .unwrap();
        assert_eq!(personal_entry.1, CalendarRole::Writer);
    }

    #[tokio::test]
    async fn test_get_users_for_calendar() {
        let repo = InMemoryRepository::new();

        // Create calendar
        let calendar = Calendar::new("Team Calendar", "#3B82F6");
        repo.create_calendar(&calendar).await.unwrap();

        // Create users
        let alice = User::new("Alice", "alice@example.com");
        let bob = User::new("Bob", "bob@example.com");
        let charlie = User::new("Charlie", "charlie@example.com");

        repo.create_user(&alice).await.unwrap();
        repo.create_user(&bob).await.unwrap();
        repo.create_user(&charlie).await.unwrap();

        // Create memberships
        let membership1 = CalendarMembership::owner(calendar.id, alice.id);
        let membership2 = CalendarMembership::writer(calendar.id, bob.id);

        repo.create_membership(&membership1).await.unwrap();
        repo.create_membership(&membership2).await.unwrap();

        // Get users for calendar
        let users = repo.get_users_for_calendar(calendar.id).await.unwrap();

        assert_eq!(users.len(), 2);
        let names: Vec<&str> = users.iter().map(|(u, _)| u.name.as_str()).collect();
        assert!(names.contains(&"Alice"));
        assert!(names.contains(&"Bob"));
        assert!(!names.contains(&"Charlie"));

        // Check roles
        let alice_entry = users.iter().find(|(u, _)| u.name == "Alice").unwrap();
        assert_eq!(alice_entry.1, CalendarRole::Owner);

        let bob_entry = users.iter().find(|(u, _)| u.name == "Bob").unwrap();
        assert_eq!(bob_entry.1, CalendarRole::Writer);
    }
}
