//! Seed command implementation.

use super::error::{DynamodbError, Result};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use calendsync_core::calendar::{CalendarEntry, EntryKind};
use chrono::{Duration, NaiveDate, NaiveTime};
use std::collections::HashMap;
use uuid::Uuid;

/// Generate mock entries using existing mock_data patterns.
pub fn generate_seed_entries(
    calendar_id: Uuid,
    center_date: NaiveDate,
    count: u32,
) -> Vec<CalendarEntry> {
    let time = |h: u32, m: u32| NaiveTime::from_hms_opt(h, m, 0).unwrap();

    // Distribution: ~15% multi-day, ~20% all-day, ~45% timed, ~20% tasks
    let multi_day_count = (count as f32 * 0.15).ceil() as u32;
    let all_day_count = (count as f32 * 0.20).ceil() as u32;
    let timed_count = (count as f32 * 0.45).ceil() as u32;
    let task_count = count.saturating_sub(multi_day_count + all_day_count + timed_count);

    let mut entries = Vec::with_capacity(count as usize);

    // Multi-day events
    let multi_day_titles = [
        "Team Retreat",
        "Conference",
        "Vacation",
        "Training Workshop",
        "Hackathon",
    ];
    let multi_day_colors = ["#8B5CF6", "#EC4899", "#10B981", "#F59E0B", "#3B82F6"];
    for i in 0..multi_day_count {
        let start = center_date + Duration::days(i as i64 * 2 - 2);
        let end = start + Duration::days(2);
        let title = multi_day_titles[i as usize % multi_day_titles.len()];
        let color = multi_day_colors[i as usize % multi_day_colors.len()];
        entries.push(
            CalendarEntry::multi_day(calendar_id, title, start, end, start).with_color(color),
        );
    }

    // All-day events
    let all_day_titles = [
        "Birthday Party",
        "Public Holiday",
        "Company Anniversary",
        "Release Day",
        "Moving Day",
    ];
    let all_day_colors = ["#EC4899", "#10B981", "#F59E0B", "#3B82F6", "#8B5CF6"];
    for i in 0..all_day_count {
        let date = center_date + Duration::days(i as i64 - 1);
        let title = all_day_titles[i as usize % all_day_titles.len()];
        let color = all_day_colors[i as usize % all_day_colors.len()];
        entries.push(CalendarEntry::all_day(calendar_id, title, date).with_color(color));
    }

    // Timed activities
    let timed_titles = [
        "Standup Meeting",
        "Lunch with Team",
        "Product Review",
        "Gym Session",
        "Team Sync",
        "Coffee with Mentor",
        "Doctor Appointment",
        "Code Review",
        "Sprint Planning",
        "1:1 Meeting",
    ];
    let timed_colors = [
        "#3B82F6", "#F97316", "#3B82F6", "#10B981", "#3B82F6", "#F97316", "#EF4444", "#8B5CF6",
        "#3B82F6", "#F59E0B",
    ];
    for i in 0..timed_count {
        let date = center_date + Duration::days((i % 7) as i64 - 3);
        let start_hour = 8 + (i % 10);
        let title = timed_titles[i as usize % timed_titles.len()];
        let color = timed_colors[i as usize % timed_colors.len()];
        entries.push(
            CalendarEntry::timed(
                calendar_id,
                title,
                date,
                time(start_hour, 0),
                time(start_hour + 1, 0),
            )
            .with_color(color),
        );
    }

    // Tasks
    let task_titles = [
        "Review PR",
        "Send Invoice",
        "Update Documentation",
        "Grocery Shopping",
        "Book Flights",
        "Reply to Emails",
        "Clean Desk",
        "Update Resume",
    ];
    for i in 0..task_count {
        let date = center_date + Duration::days((i % 5) as i64 - 2);
        let title = task_titles[i as usize % task_titles.len()];
        let completed = i % 3 == 0; // ~33% completed
        entries.push(CalendarEntry::task(calendar_id, title, date, completed));
    }

    entries
}

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

/// Format entry kind for display.
pub fn format_entry_kind(kind: &EntryKind) -> &'static str {
    match kind {
        EntryKind::MultiDay { .. } => "multi-day",
        EntryKind::AllDay => "all-day",
        EntryKind::Timed { .. } => "timed",
        EntryKind::Task { .. } => "task",
    }
}
