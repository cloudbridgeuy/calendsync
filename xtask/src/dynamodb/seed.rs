//! Seed command implementation.

use super::error::{DynamodbError, Result};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use calendsync_core::calendar::CalendarEntry;
use std::collections::HashMap;

// Re-export from core for backwards compatibility
pub use calendsync_core::calendar::{format_entry_kind, generate_seed_entries};

/// Convert CalendarEntry to DynamoDB item.
fn entry_to_item(entry: &CalendarEntry) -> HashMap<String, AttributeValue> {
    let mut item = HashMap::new();

    // Primary key: PK = ENTRY#<entry_id>, SK = ENTRY#<entry_id>
    item.insert(
        "PK".to_string(),
        AttributeValue::S(format!("ENTRY#{}", entry.id)),
    );
    item.insert(
        "SK".to_string(),
        AttributeValue::S(format!("ENTRY#{}", entry.id)),
    );

    // GSI1: For querying by calendar and date
    item.insert(
        "GSI1PK".to_string(),
        AttributeValue::S(format!("CAL#{}", entry.calendar_id)),
    );
    item.insert(
        "GSI1SK".to_string(),
        AttributeValue::S(format!("ENTRY#{}#{}", entry.date, entry.id)),
    );

    // Entity type
    item.insert(
        "entityType".to_string(),
        AttributeValue::S("ENTRY".to_string()),
    );

    // Entry data
    item.insert("id".to_string(), AttributeValue::S(entry.id.to_string()));
    item.insert(
        "calendarId".to_string(),
        AttributeValue::S(entry.calendar_id.to_string()),
    );
    item.insert("title".to_string(), AttributeValue::S(entry.title.clone()));
    item.insert(
        "date".to_string(),
        AttributeValue::S(entry.date.to_string()),
    );

    if let Some(desc) = &entry.description {
        item.insert("description".to_string(), AttributeValue::S(desc.clone()));
    }
    if let Some(loc) = &entry.location {
        item.insert("location".to_string(), AttributeValue::S(loc.clone()));
    }
    if let Some(color) = &entry.color {
        item.insert("color".to_string(), AttributeValue::S(color.clone()));
    }

    // Entry kind as JSON
    let kind_json = serde_json::to_string(&entry.kind).unwrap_or_default();
    item.insert("kind".to_string(), AttributeValue::S(kind_json));

    // Timestamps
    let now = chrono::Utc::now().to_rfc3339();
    item.insert("createdAt".to_string(), AttributeValue::S(now.clone()));
    item.insert("updatedAt".to_string(), AttributeValue::S(now));

    item
}

/// Insert entries into DynamoDB.
pub async fn seed_entries(
    client: &Client,
    table_name: &str,
    entries: &[CalendarEntry],
) -> Result<u32> {
    let mut inserted = 0;

    // Use batch write for efficiency (25 items per batch max)
    for chunk in entries.chunks(25) {
        let write_requests: Vec<_> = chunk
            .iter()
            .map(|entry| {
                aws_sdk_dynamodb::types::WriteRequest::builder()
                    .put_request(
                        aws_sdk_dynamodb::types::PutRequest::builder()
                            .set_item(Some(entry_to_item(entry)))
                            .build()
                            .expect("Failed to build PutRequest"),
                    )
                    .build()
            })
            .collect();

        client
            .batch_write_item()
            .request_items(table_name, write_requests)
            .send()
            .await
            .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?;

        inserted += chunk.len() as u32;
    }

    Ok(inserted)
}
