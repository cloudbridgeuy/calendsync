//! DynamoDB repository implementation.
//!
//! Implements the repository traits from `calendsync_core::storage` using DynamoDB.

use async_trait::async_trait;
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use uuid::Uuid;

use calendsync_core::calendar::{Calendar, CalendarEntry, CalendarMembership, CalendarRole, User};
use calendsync_core::storage::{
    CalendarRepository, DateRange, EntryRepository, MembershipRepository, Result, UserRepository,
};

use super::conversions::{
    calendar_to_item, entry_to_item, item_to_calendar, item_to_entry, item_to_membership,
    item_to_user, membership_to_item, user_to_item,
};
use super::error::{
    map_delete_item_error, map_get_item_error, map_put_item_error, map_query_error,
};
use super::keys;

/// DynamoDB-based repository implementation.
///
/// Provides async access to DynamoDB storage for all entity types.
pub struct DynamoDbRepository {
    client: Client,
    table_name: String,
}

impl DynamoDbRepository {
    /// Creates a new repository with the given DynamoDB client and table name.
    pub fn new(client: Client, table_name: impl Into<String>) -> Self {
        Self {
            client,
            table_name: table_name.into(),
        }
    }

    /// Creates a new repository from environment configuration.
    ///
    /// Uses AWS SDK default credential chain and reads table name from
    /// `DYNAMODB_TABLE_NAME` environment variable (defaults to "calendsync").
    pub async fn from_env() -> Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let client = Client::new(&config);
        let table_name =
            std::env::var("DYNAMODB_TABLE_NAME").unwrap_or_else(|_| "calendsync".to_string());

        Ok(Self::new(client, table_name))
    }

    /// Get the table name.
    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

// ============================================================================
// EntryRepository implementation
// ============================================================================

#[async_trait]
impl EntryRepository for DynamoDbRepository {
    async fn get_entry(&self, id: Uuid) -> Result<Option<CalendarEntry>> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(keys::entry_pk(id)))
            .key("SK", AttributeValue::S(keys::entry_sk(id)))
            .send()
            .await
            .map_err(|e| map_get_item_error(e, "CalendarEntry", id.to_string()))?;

        match result.item {
            Some(item) => Ok(Some(item_to_entry(&item)?)),
            None => Ok(None),
        }
    }

    async fn get_entries_by_calendar(
        &self,
        calendar_id: Uuid,
        date_range: DateRange,
    ) -> Result<Vec<CalendarEntry>> {
        let result = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name("GSI1")
            .key_condition_expression("GSI1PK = :pk AND GSI1SK BETWEEN :start AND :end")
            .expression_attribute_values(":pk", AttributeValue::S(keys::entry_gsi1_pk(calendar_id)))
            .expression_attribute_values(
                ":start",
                AttributeValue::S(keys::entry_gsi1_sk_start(date_range.start)),
            )
            .expression_attribute_values(
                ":end",
                AttributeValue::S(keys::entry_gsi1_sk_end(date_range.end)),
            )
            .send()
            .await
            .map_err(map_query_error)?;

        let items = result.items.unwrap_or_default();
        items.iter().map(item_to_entry).collect()
    }

    async fn create_entry(&self, entry: &CalendarEntry) -> Result<()> {
        let item = entry_to_item(entry)?;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_not_exists(PK)")
            .send()
            .await
            .map_err(|e| map_put_item_error(e, "CalendarEntry", entry.id.to_string()))?;

        Ok(())
    }

    async fn update_entry(&self, entry: &CalendarEntry) -> Result<()> {
        let item = entry_to_item(entry)?;

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_exists(PK)")
            .send()
            .await
            .map_err(|e| map_put_item_error(e, "CalendarEntry", entry.id.to_string()))?;

        Ok(())
    }

    async fn delete_entry(&self, id: Uuid) -> Result<()> {
        self.client
            .delete_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(keys::entry_pk(id)))
            .key("SK", AttributeValue::S(keys::entry_sk(id)))
            .condition_expression("attribute_exists(PK)")
            .send()
            .await
            .map_err(|e| map_delete_item_error(e, "CalendarEntry", id.to_string()))?;

        Ok(())
    }
}

// ============================================================================
// CalendarRepository implementation
// ============================================================================

#[async_trait]
impl CalendarRepository for DynamoDbRepository {
    async fn get_calendar(&self, id: Uuid) -> Result<Option<Calendar>> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(keys::calendar_pk(id)))
            .key("SK", AttributeValue::S(keys::calendar_sk(id)))
            .send()
            .await
            .map_err(|e| map_get_item_error(e, "Calendar", id.to_string()))?;

        match result.item {
            Some(item) => Ok(Some(item_to_calendar(&item)?)),
            None => Ok(None),
        }
    }

    async fn create_calendar(&self, calendar: &Calendar) -> Result<()> {
        let item = calendar_to_item(calendar);

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_not_exists(PK)")
            .send()
            .await
            .map_err(|e| map_put_item_error(e, "Calendar", calendar.id.to_string()))?;

        Ok(())
    }

    async fn update_calendar(&self, calendar: &Calendar) -> Result<()> {
        let item = calendar_to_item(calendar);

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_exists(PK)")
            .send()
            .await
            .map_err(|e| map_put_item_error(e, "Calendar", calendar.id.to_string()))?;

        Ok(())
    }

    async fn delete_calendar(&self, id: Uuid) -> Result<()> {
        self.client
            .delete_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(keys::calendar_pk(id)))
            .key("SK", AttributeValue::S(keys::calendar_sk(id)))
            .condition_expression("attribute_exists(PK)")
            .send()
            .await
            .map_err(|e| map_delete_item_error(e, "Calendar", id.to_string()))?;

        Ok(())
    }
}

// ============================================================================
// UserRepository implementation
// ============================================================================

#[async_trait]
impl UserRepository for DynamoDbRepository {
    async fn get_user(&self, id: Uuid) -> Result<Option<User>> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(keys::user_pk(id)))
            .key("SK", AttributeValue::S(keys::user_sk(id)))
            .send()
            .await
            .map_err(|e| map_get_item_error(e, "User", id.to_string()))?;

        match result.item {
            Some(item) => Ok(Some(item_to_user(&item)?)),
            None => Ok(None),
        }
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let result = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name("GSI2")
            .key_condition_expression("GSI2PK = :pk")
            .expression_attribute_values(":pk", AttributeValue::S(keys::user_gsi2_pk(email)))
            .send()
            .await
            .map_err(map_query_error)?;

        let items = result.items.unwrap_or_default();
        match items.first() {
            Some(item) => Ok(Some(item_to_user(item)?)),
            None => Ok(None),
        }
    }

    async fn create_user(&self, user: &User) -> Result<()> {
        let item = user_to_item(user);

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_not_exists(PK)")
            .send()
            .await
            .map_err(|e| map_put_item_error(e, "User", user.id.to_string()))?;

        Ok(())
    }

    async fn update_user(&self, user: &User) -> Result<()> {
        let item = user_to_item(user);

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_exists(PK)")
            .send()
            .await
            .map_err(|e| map_put_item_error(e, "User", user.id.to_string()))?;

        Ok(())
    }
}

// ============================================================================
// MembershipRepository implementation
// ============================================================================

#[async_trait]
impl MembershipRepository for DynamoDbRepository {
    async fn get_membership(
        &self,
        calendar_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CalendarMembership>> {
        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(keys::membership_pk(calendar_id)))
            .key("SK", AttributeValue::S(keys::membership_sk(user_id)))
            .send()
            .await
            .map_err(|e| {
                map_get_item_error(
                    e,
                    "CalendarMembership",
                    format!("{}:{}", calendar_id, user_id),
                )
            })?;

        match result.item {
            Some(item) => Ok(Some(item_to_membership(&item)?)),
            None => Ok(None),
        }
    }

    async fn get_calendars_for_user(&self, user_id: Uuid) -> Result<Vec<(Calendar, CalendarRole)>> {
        // First, get all memberships for the user via GSI1
        let membership_result = self
            .client
            .query()
            .table_name(&self.table_name)
            .index_name("GSI1")
            .key_condition_expression("GSI1PK = :pk AND begins_with(GSI1SK, :sk_prefix)")
            .expression_attribute_values(
                ":pk",
                AttributeValue::S(keys::membership_gsi1_pk(user_id)),
            )
            .expression_attribute_values(
                ":sk_prefix",
                AttributeValue::S(keys::calendar_gsi1_sk_prefix().to_string()),
            )
            .send()
            .await
            .map_err(map_query_error)?;

        let membership_items = membership_result.items.unwrap_or_default();
        if membership_items.is_empty() {
            return Ok(Vec::new());
        }

        // Parse memberships to get calendar IDs and roles
        let memberships: Vec<CalendarMembership> = membership_items
            .iter()
            .filter_map(|item| item_to_membership(item).ok())
            .collect();

        // Batch get all calendars
        let mut results = Vec::with_capacity(memberships.len());
        for membership in memberships {
            if let Ok(Some(calendar)) = self.get_calendar(membership.calendar_id).await {
                results.push((calendar, membership.role));
            }
        }

        Ok(results)
    }

    async fn get_users_for_calendar(&self, calendar_id: Uuid) -> Result<Vec<(User, CalendarRole)>> {
        // Get all memberships for the calendar
        let membership_result = self
            .client
            .query()
            .table_name(&self.table_name)
            .key_condition_expression("PK = :pk AND begins_with(SK, :sk_prefix)")
            .expression_attribute_values(":pk", AttributeValue::S(keys::membership_pk(calendar_id)))
            .expression_attribute_values(
                ":sk_prefix",
                AttributeValue::S(keys::membership_sk_prefix().to_string()),
            )
            .send()
            .await
            .map_err(map_query_error)?;

        let membership_items = membership_result.items.unwrap_or_default();
        if membership_items.is_empty() {
            return Ok(Vec::new());
        }

        // Parse memberships to get user IDs and roles
        let memberships: Vec<CalendarMembership> = membership_items
            .iter()
            .filter_map(|item| item_to_membership(item).ok())
            .collect();

        // Batch get all users
        let mut results = Vec::with_capacity(memberships.len());
        for membership in memberships {
            if let Ok(Some(user)) = self.get_user(membership.user_id).await {
                results.push((user, membership.role));
            }
        }

        Ok(results)
    }

    async fn create_membership(&self, membership: &CalendarMembership) -> Result<()> {
        let item = membership_to_item(membership);

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .condition_expression("attribute_not_exists(PK)")
            .send()
            .await
            .map_err(|e| {
                map_put_item_error(
                    e,
                    "CalendarMembership",
                    format!("{}:{}", membership.calendar_id, membership.user_id),
                )
            })?;

        Ok(())
    }

    async fn delete_membership(&self, calendar_id: Uuid, user_id: Uuid) -> Result<()> {
        self.client
            .delete_item()
            .table_name(&self.table_name)
            .key("PK", AttributeValue::S(keys::membership_pk(calendar_id)))
            .key("SK", AttributeValue::S(keys::membership_sk(user_id)))
            .condition_expression("attribute_exists(PK)")
            .send()
            .await
            .map_err(|e| {
                map_delete_item_error(
                    e,
                    "CalendarMembership",
                    format!("{}:{}", calendar_id, user_id),
                )
            })?;

        Ok(())
    }
}
