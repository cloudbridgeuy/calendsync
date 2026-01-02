# Multi-Day Entries Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Replace point-in-time date queries with range overlap queries so multi-day entries appear on all days they span.

**Architecture:** All entries get `start_date` and `end_date` fields. Backend queries use overlap detection (`start <= query.end AND end >= query.start`). Frontend expands multi-day entries across days for display.

**Tech Stack:** Rust (calendsync_core, calendsync), TypeScript (frontend), SQLite, DynamoDB

---

## Phase 1: Core Type Changes

### Task 1.1: Update EntryKind enum

**File:** `crates/core/src/calendar/types.rs`

Remove the date fields from `MultiDay` variant since dates will live in `CalendarEntry`.

**Step 1:** Open file and locate `EntryKind` enum (around line 188).

**Step 2:** Change `MultiDay { start: NaiveDate, end: NaiveDate }` to just `MultiDay`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    /// An event spanning multiple days.
    MultiDay,
    /// An all-day event (no specific time).
    AllDay,
    /// A timed activity with start and end times.
    Timed { start: NaiveTime, end: NaiveTime },
    /// A task that can be marked as completed.
    Task { completed: bool },
}
```

**Step 3:** Remove these methods from `impl EntryKind`:
- `multi_day_start(&self) -> Option<NaiveDate>`
- `multi_day_end(&self) -> Option<NaiveDate>`

**Step 4:** Run `cargo check -p calendsync_core` to see what breaks.

---

### Task 1.2: Update CalendarEntry struct

**File:** `crates/core/src/calendar/types.rs`

**Step 1:** Replace `date: NaiveDate` with `start_date` and `end_date`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEntry {
    pub id: Uuid,
    pub calendar_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub kind: EntryKind,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Step 2:** Run `cargo check -p calendsync_core` to see all compilation errors.

---

### Task 1.3: Update CalendarEntry constructors

**File:** `crates/core/src/calendar/types.rs`

**Step 1:** Update `multi_day` constructor:

```rust
pub fn multi_day(
    calendar_id: Uuid,
    title: impl Into<String>,
    start: NaiveDate,
    end: NaiveDate,
) -> Self {
    let now = Utc::now();
    Self {
        id: Uuid::new_v4(),
        calendar_id,
        title: title.into(),
        description: None,
        location: None,
        kind: EntryKind::MultiDay,
        start_date: start,
        end_date: end,
        color: None,
        created_at: now,
        updated_at: now,
    }
}
```

**Step 2:** Update `all_day` constructor (single day: `start_date == end_date`):

```rust
pub fn all_day(calendar_id: Uuid, title: impl Into<String>, date: NaiveDate) -> Self {
    let now = Utc::now();
    Self {
        id: Uuid::new_v4(),
        calendar_id,
        title: title.into(),
        description: None,
        location: None,
        kind: EntryKind::AllDay,
        start_date: date,
        end_date: date,
        color: None,
        created_at: now,
        updated_at: now,
    }
}
```

**Step 3:** Update `timed` constructor similarly:

```rust
pub fn timed(
    calendar_id: Uuid,
    title: impl Into<String>,
    date: NaiveDate,
    start: NaiveTime,
    end: NaiveTime,
) -> Self {
    let now = Utc::now();
    Self {
        id: Uuid::new_v4(),
        calendar_id,
        title: title.into(),
        description: None,
        location: None,
        kind: EntryKind::Timed { start, end },
        start_date: date,
        end_date: date,
        color: None,
        created_at: now,
        updated_at: now,
    }
}
```

**Step 4:** Update `task` constructor similarly:

```rust
pub fn task(
    calendar_id: Uuid,
    title: impl Into<String>,
    date: NaiveDate,
    completed: bool,
) -> Self {
    let now = Utc::now();
    Self {
        id: Uuid::new_v4(),
        calendar_id,
        title: title.into(),
        description: None,
        location: None,
        kind: EntryKind::Task { completed },
        start_date: date,
        end_date: date,
        color: None,
        created_at: now,
        updated_at: now,
    }
}
```

**Step 5:** Run `cargo check -p calendsync_core`.

---

### Task 1.4: Update CalendarEntry tests

**File:** `crates/core/src/calendar/types.rs`

**Step 1:** Update tests to use `start_date` and `end_date`:

```rust
#[test]
fn test_calendar_entry_builder() {
    let calendar_id = Uuid::new_v4();
    let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    let entry = CalendarEntry::all_day(calendar_id, "Birthday", date)
        .with_description("John's birthday party")
        .with_location("123 Main St")
        .with_color("#F97316");

    assert_eq!(entry.calendar_id, calendar_id);
    assert_eq!(entry.title, "Birthday");
    assert_eq!(entry.start_date, date);
    assert_eq!(entry.end_date, date);
    assert!(entry.kind.is_all_day());
}

#[test]
fn test_multi_day_entry_dates() {
    let calendar_id = Uuid::new_v4();
    let start = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 1, 20).unwrap();
    let entry = CalendarEntry::multi_day(calendar_id, "Vacation", start, end);

    assert_eq!(entry.start_date, start);
    assert_eq!(entry.end_date, end);
    assert!(entry.start_date < entry.end_date);
    assert!(entry.kind.is_multi_day());
}
```

**Step 2:** Run `cargo test -p calendsync_core calendar::types`.

---

### Task 1.5: Update sorting.rs

**File:** `crates/core/src/calendar/sorting.rs`

**Step 1:** Update `group_entries_by_date` to use `start_date`:

```rust
pub fn group_entries_by_date(entries: &[CalendarEntry]) -> HashMap<NaiveDate, Vec<&CalendarEntry>> {
    let mut grouped: HashMap<NaiveDate, Vec<&CalendarEntry>> = HashMap::new();

    for entry in entries {
        grouped.entry(entry.start_date).or_default().push(entry);
    }

    grouped
}
```

**Step 2:** Update `expand_multi_day_entries` to use new struct fields:

```rust
pub fn expand_multi_day_entries(entries: Vec<CalendarEntry>) -> Vec<CalendarEntry> {
    let mut expanded = Vec::new();

    for entry in entries {
        if entry.kind.is_multi_day() {
            let mut current = entry.start_date;
            while current <= entry.end_date {
                let day_entry = entry.clone();
                expanded.push(day_entry);
                current += Duration::days(1);
            }
        } else {
            expanded.push(entry);
        }
    }

    expanded
}
```

**Step 3:** Update tests to use new field names.

**Step 4:** Run `cargo test -p calendsync_core calendar::sorting`.

---

### Task 1.6: Update requests.rs

**File:** `crates/core/src/calendar/requests.rs`

**Step 1:** Update `CreateEntryRequest` to rename `date` to `start_date`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEntryRequest {
    pub calendar_id: Uuid,
    pub title: String,
    pub start_date: NaiveDate,  // renamed from `date`
    pub entry_type: EntryType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time: Option<NaiveTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_time: Option<NaiveTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}
```

**Step 2:** Update all constructor methods (`all_day`, `timed`, `task`, `multi_day`) to use `start_date`.

**Step 3:** Update `into_entry` method:

```rust
pub fn into_entry(self) -> Option<CalendarEntry> {
    let end_date = match self.entry_type {
        EntryType::MultiDay => self.end_date?,
        _ => self.start_date,
    };

    let kind = match self.entry_type {
        EntryType::AllDay => EntryKind::AllDay,
        EntryType::Timed => {
            let start = self.start_time?;
            let end = self.end_time?;
            EntryKind::Timed { start, end }
        }
        EntryType::Task => EntryKind::Task { completed: false },
        EntryType::MultiDay => EntryKind::MultiDay,
    };

    let now = Utc::now();
    Some(CalendarEntry {
        id: Uuid::new_v4(),
        calendar_id: self.calendar_id,
        title: self.title,
        description: self.description,
        location: self.location,
        kind,
        start_date: self.start_date,
        end_date,
        color: self.color,
        created_at: now,
        updated_at: now,
    })
}
```

**Step 4:** Update `UpdateEntryRequest` similarly - rename `date` to `start_date`.

**Step 5:** Update `apply_to` method to handle the new fields properly.

**Step 6:** Update all tests in the file.

**Step 7:** Run `cargo test -p calendsync_core calendar::requests`.

---

### Task 1.7: Update mock_data.rs

**File:** `crates/core/src/calendar/mock_data.rs`

**Step 1:** Update `generate_seed_entries` to use new constructor signatures.

**Step 2:** Update `format_entry_kind` if it references multi-day dates.

**Step 3:** Run `cargo test -p calendsync_core`.

---

### Task 1.8: Verify core compiles

**Step 1:** Run `cargo check -p calendsync_core`.

**Step 2:** Run `cargo test -p calendsync_core`.

**Step 3:** Fix any remaining issues.

---

## Phase 2: Storage Layer Changes

### Task 2.1: Update SQLite schema

**File:** `crates/calendsync/src/storage/sqlite/schema.rs`

**Step 1:** Update `CREATE_TABLES` - replace `date` with `start_date` and `end_date`:

```rust
pub const CREATE_TABLES: &str = r#"
-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Calendars table
CREATE TABLE IF NOT EXISTS calendars (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    color TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Calendar entries table
CREATE TABLE IF NOT EXISTS entries (
    id TEXT PRIMARY KEY,
    calendar_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    location TEXT,
    kind TEXT NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    color TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (calendar_id) REFERENCES calendars(id) ON DELETE CASCADE
);

-- Calendar memberships table
CREATE TABLE IF NOT EXISTS memberships (
    calendar_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (calendar_id, user_id),
    FOREIGN KEY (calendar_id) REFERENCES calendars(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_entries_calendar_id ON entries(calendar_id);
CREATE INDEX IF NOT EXISTS idx_entries_calendar_range ON entries(calendar_id, start_date, end_date);
CREATE INDEX IF NOT EXISTS idx_memberships_user_id ON memberships(user_id);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
"#;
```

**Step 2:** Update `INSERT_ENTRY`:

```rust
pub const INSERT_ENTRY: &str = r#"
INSERT INTO entries (id, calendar_id, title, description, location, kind, start_date, end_date, color, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
"#;
```

**Step 3:** Update `SELECT_ENTRY_BY_ID`:

```rust
pub const SELECT_ENTRY_BY_ID: &str = r#"
SELECT id, calendar_id, title, description, location, kind, start_date, end_date, color, created_at, updated_at
FROM entries
WHERE id = ?1
"#;
```

**Step 4:** Update `SELECT_ENTRIES_BY_CALENDAR_AND_DATE_RANGE` for overlap queries:

```rust
pub const SELECT_ENTRIES_BY_CALENDAR_AND_DATE_RANGE: &str = r#"
SELECT id, calendar_id, title, description, location, kind, start_date, end_date, color, created_at, updated_at
FROM entries
WHERE calendar_id = ?1
  AND start_date <= ?3
  AND end_date >= ?2
ORDER BY start_date ASC, end_date ASC
"#;
```

**Step 5:** Update `UPDATE_ENTRY`:

```rust
pub const UPDATE_ENTRY: &str = r#"
UPDATE entries
SET title = ?2, description = ?3, location = ?4, kind = ?5, start_date = ?6, end_date = ?7, color = ?8, updated_at = ?9
WHERE id = ?1
"#;
```

**Step 6:** Run `cargo check -p calendsync --features sqlite,memory`.

---

### Task 2.2: Update SQLite conversions

**File:** `crates/calendsync/src/storage/sqlite/conversions.rs`

**Step 1:** Update `row_to_entry` to read `start_date` and `end_date` at correct indices:

The column order is now:
- 0: id
- 1: calendar_id
- 2: title
- 3: description
- 4: location
- 5: kind
- 6: start_date
- 7: end_date
- 8: color
- 9: created_at
- 10: updated_at

**Step 2:** Update the function to parse these correctly and create `CalendarEntry` with `start_date` and `end_date`.

**Step 3:** Run `cargo check -p calendsync --features sqlite,memory`.

---

### Task 2.3: Update SQLite repository

**File:** `crates/calendsync/src/storage/sqlite/repository.rs`

**Step 1:** Update `create_entry` to use `start_date` and `end_date`:

```rust
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

    self.conn
        .call(move |conn| {
            conn.execute(
                schema::INSERT_ENTRY,
                rusqlite::params![
                    id, calendar_id, title, description, location,
                    kind_json, start_date, end_date, color, created_at, updated_at
                ],
            )
            .map_err(wrap_err)?;
            Ok(())
        })
        .await
        .map_err(|e| RepositoryError::QueryFailed(e.to_string()))
}
```

**Step 2:** Update `update_entry` similarly.

**Step 3:** Run `cargo test -p calendsync --features sqlite,memory sqlite::`.

---

### Task 2.4: Update InMemory repository

**File:** `crates/calendsync/src/storage/inmemory/repository.rs`

**Step 1:** Update `get_entries_by_calendar` for overlap detection:

```rust
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
```

**Step 2:** Update tests to use new struct fields.

**Step 3:** Run `cargo test -p calendsync --features inmemory,memory inmemory::`.

---

### Task 2.5: Update DynamoDB keys

**File:** `crates/calendsync/src/storage/dynamodb/keys.rs`

**Step 1:** Update `entry_gsi1_sk` to use `start_date` (parameter rename for clarity):

```rust
/// Generate GSI1 sort key for Entry (date-sorted lookup).
///
/// Pattern: `ENTRY#<start_date>#<entry_id>`
pub fn entry_gsi1_sk(start_date: NaiveDate, entry_id: Uuid) -> String {
    format!("{ENTRY_PREFIX}{}#{entry_id}", start_date.format("%Y-%m-%d"))
}
```

**Step 2:** Add a max SK function for overlap queries:

```rust
/// Generate the maximum sort key for overlap queries.
/// Used to find all entries starting on or before a given date.
///
/// Pattern: `ENTRY#<date>#~`
pub fn entry_gsi1_sk_max(date: NaiveDate) -> String {
    format!("{ENTRY_PREFIX}{}#~", date.format("%Y-%m-%d"))
}
```

**Step 3:** Update tests.

**Step 4:** Run `cargo check -p calendsync --features dynamodb,memory`.

---

### Task 2.6: Update DynamoDB conversions

**File:** `crates/calendsync/src/storage/dynamodb/conversions.rs`

**Step 1:** Update `entry_to_item` to include `start_date` and `end_date` attributes and update GSI1SK:

```rust
// Add these attributes
item.insert("start_date".to_string(), AttributeValue::S(entry.start_date.to_string()));
item.insert("end_date".to_string(), AttributeValue::S(entry.end_date.to_string()));

// Update GSI1SK to use start_date
item.insert("GSI1SK".to_string(), AttributeValue::S(entry_gsi1_sk(entry.start_date, entry.id)));
```

**Step 2:** Update `item_to_entry` to read `start_date` and `end_date`:

```rust
let start_date = get_date(item, "start_date")?;
let end_date = get_date(item, "end_date")?;

Ok(CalendarEntry {
    // ...
    start_date,
    end_date,
    // ...
})
```

**Step 3:** Run `cargo check -p calendsync --features dynamodb,memory`.

---

### Task 2.7: Update DynamoDB repository

**File:** `crates/calendsync/src/storage/dynamodb/repository.rs`

**Step 1:** Update `get_entries_by_calendar` for overlap queries:

```rust
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
        .key_condition_expression("GSI1PK = :pk AND GSI1SK <= :max_sk")
        .filter_expression("end_date >= :query_start")
        .expression_attribute_values(":pk", AttributeValue::S(entry_gsi1_pk(calendar_id)))
        .expression_attribute_values(":max_sk", AttributeValue::S(entry_gsi1_sk_max(date_range.end)))
        .expression_attribute_values(":query_start", AttributeValue::S(date_range.start.to_string()))
        .send()
        .await
        .map_err(map_sdk_error)?;

    let items = result.items.unwrap_or_default();
    items.iter().map(item_to_entry).collect()
}
```

**Step 2:** Run `cargo check -p calendsync --features dynamodb,memory`.

---

### Task 2.8: Update cached repository

**File:** `crates/calendsync/src/storage/cached/entry.rs`

**Step 1:** Verify cache key generation still works (uses date range, should be fine).

**Step 2:** Run `cargo check -p calendsync`.

---

## Phase 3: API Layer Changes

### Task 3.1: Update server entry model

**File:** `crates/calendsync/src/models/entry.rs`

**Step 1:** Update `CreateEntry` - rename `date` to `start_date`:

```rust
#[derive(Debug, Deserialize)]
pub struct CreateEntry {
    pub calendar_id: Uuid,
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub location: Option<String>,
    pub start_date: NaiveDate,  // renamed from `date`
    pub entry_type: ServerEntryType,
    #[serde(default, deserialize_with = "deserialize_optional_time")]
    pub start_time: Option<NaiveTime>,
    #[serde(default, deserialize_with = "deserialize_optional_time")]
    pub end_time: Option<NaiveTime>,
    #[serde(default, deserialize_with = "deserialize_optional_date")]
    pub end_date: Option<NaiveDate>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub color: Option<String>,
}
```

**Step 2:** Update `into_entry` method:

```rust
pub fn into_entry(self) -> Option<CalendarEntry> {
    let end_date = match self.entry_type {
        ServerEntryType::MultiDay => self.end_date?,
        _ => self.start_date,
    };

    let kind = match self.entry_type {
        ServerEntryType::AllDay => EntryKind::AllDay,
        ServerEntryType::Timed => {
            let start = self.start_time?;
            let end = self.end_time?;
            EntryKind::Timed { start, end }
        }
        ServerEntryType::Task => EntryKind::Task { completed: false },
        ServerEntryType::MultiDay => EntryKind::MultiDay,
    };

    let now = Utc::now();
    Some(CalendarEntry {
        id: Uuid::new_v4(),
        calendar_id: self.calendar_id,
        title: self.title,
        description: self.description,
        location: self.location,
        kind,
        start_date: self.start_date,
        end_date,
        color: self.color,
        created_at: now,
        updated_at: now,
    })
}
```

**Step 3:** Update `UpdateEntry` similarly - rename `date` to `start_date` and update `apply_to`.

**Step 4:** Run `cargo check -p calendsync`.

---

### Task 3.2: Update entry handlers

**File:** `crates/calendsync/src/handlers/entries.rs`

**Step 1:** Update `entry_to_server_entry`:

```rust
pub fn entry_to_server_entry(entry: &CalendarEntry) -> serde_json::Value {
    let (kind, completed, is_multi_day, is_all_day, is_timed, is_task) = match &entry.kind {
        EntryKind::AllDay => ("all-day", false, false, true, false, false),
        EntryKind::Timed { .. } => ("timed", false, false, false, true, false),
        EntryKind::Task { completed } => ("task", *completed, false, false, false, true),
        EntryKind::MultiDay => ("multi-day", false, true, false, false, false),
    };

    let start_time = entry.kind.start_time().map(|t| t.format("%H:%M").to_string());
    let end_time = entry.kind.end_time().map(|t| t.format("%H:%M").to_string());

    serde_json::json!({
        "id": entry.id.to_string(),
        "calendarId": entry.calendar_id.to_string(),
        "kind": kind,
        "completed": completed,
        "isMultiDay": is_multi_day,
        "isAllDay": is_all_day,
        "isTimed": is_timed,
        "isTask": is_task,
        "title": entry.title,
        "description": entry.description,
        "location": entry.location,
        "color": entry.color,
        "startDate": entry.start_date.to_string(),
        "endDate": entry.end_date.to_string(),
        "startTime": start_time,
        "endTime": end_time,
    })
}
```

**Step 2:** Update `entries_to_server_days` to use `start_date`:

```rust
pub fn entries_to_server_days(
    entries: &[&CalendarEntry],
    start: NaiveDate,
    end: NaiveDate,
) -> Vec<serde_json::Value> {
    let mut days_map: BTreeMap<NaiveDate, Vec<serde_json::Value>> = BTreeMap::new();

    // Initialize all dates in the range
    let mut current = start;
    while current <= end {
        days_map.insert(current, Vec::new());
        current += chrono::Duration::days(1);
    }

    // Add entries - use start_date for grouping
    // (Frontend will expand multi-day entries)
    for entry in entries {
        if entry.start_date >= start && entry.start_date <= end {
            let server_entry = entry_to_server_entry(entry);
            days_map.entry(entry.start_date).or_default().push(server_entry);
        }
    }

    days_map
        .into_iter()
        .map(|(date, entries)| {
            serde_json::json!({
                "date": date.to_string(),
                "entries": entries,
            })
        })
        .collect()
}
```

**Step 3:** Run `cargo check -p calendsync`.

---

### Task 3.3: Update calendar_react handler

**File:** `crates/calendsync/src/handlers/calendar_react.rs`

**Step 1:** Search for any usage of `.date` field and update to `.start_date`.

**Step 2:** Run `cargo check -p calendsync`.

---

### Task 3.4: Verify server compiles and tests pass

**Step 1:** Run `cargo check -p calendsync`.

**Step 2:** Run `cargo test -p calendsync`.

**Step 3:** Fix any remaining issues.

---

## Phase 4: Frontend Changes

### Task 4.1: Update ServerEntry type

**File:** `crates/frontend/src/core/calendar/types.ts`

**Step 1:** Update the interface:

```typescript
export interface ServerEntry {
  id: string
  calendarId: string
  kind: string
  completed: boolean
  isMultiDay: boolean
  isAllDay: boolean
  isTimed: boolean
  isTask: boolean
  title: string
  description: string | null
  location: string | null
  color: string | null
  startDate: string
  endDate: string
  startTime: string | null
  endTime: string | null
}
```

**Step 2:** Run `bun run typecheck` in `crates/frontend` to see what breaks.

---

### Task 4.2: Add date range helpers

**File:** `crates/frontend/src/core/calendar/dates.ts`

**Step 1:** Add string-based date helpers:

```typescript
/**
 * Generate all ISO date strings from start to end (inclusive).
 * Assumes YYYY-MM-DD format.
 */
export function dateRangeStrings(start: string, end: string): string[] {
  const dates: string[] = []
  let current = start
  while (current <= end) {
    dates.push(current)
    current = addDaysToString(current, 1)
  }
  return dates
}

/**
 * Add days to an ISO date string.
 */
export function addDaysToString(dateStr: string, days: number): string {
  const date = parseDateKey(dateStr)
  const result = addDays(date, days)
  return formatDateKey(result)
}

/**
 * Return the later of two ISO date strings.
 */
export function maxDateString(a: string, b: string): string {
  return a > b ? a : b
}

/**
 * Return the earlier of two ISO date strings.
 */
export function minDateString(a: string, b: string): string {
  return a < b ? a : b
}
```

**Step 2:** Run `bun test` in `crates/frontend`.

---

### Task 4.3: Add multi-day expansion function

**File:** `crates/frontend/src/core/calendar/entries.ts`

**Step 1:** Add the expansion function:

```typescript
import { dateRangeStrings, maxDateString, minDateString } from "./dates"
import type { ServerEntry } from "./types"

/**
 * Expand multi-day entries into a map of date -> entries.
 * Multi-day entries appear on every day they span (clipped to view bounds).
 * Single-day entries appear only on their start date.
 */
export function expandMultiDayEntries(
  entries: ServerEntry[],
  viewStart: string,
  viewEnd: string
): Map<string, ServerEntry[]> {
  const dayMap = new Map<string, ServerEntry[]>()

  for (const entry of entries) {
    if (entry.isMultiDay) {
      // Clip to view bounds
      const start = maxDateString(entry.startDate, viewStart)
      const end = minDateString(entry.endDate, viewEnd)

      for (const date of dateRangeStrings(start, end)) {
        addToDay(dayMap, date, entry)
      }
    } else {
      // Single-day entry
      if (entry.startDate >= viewStart && entry.startDate <= viewEnd) {
        addToDay(dayMap, entry.startDate, entry)
      }
    }
  }

  return dayMap
}

function addToDay(map: Map<string, ServerEntry[]>, date: string, entry: ServerEntry): void {
  const existing = map.get(date) ?? []
  existing.push(entry)
  map.set(date, existing)
}
```

**Step 2:** Export from `crates/frontend/src/core/calendar/index.ts`.

---

### Task 4.4: Add expansion tests

**File:** `crates/frontend/src/core/calendar/__tests__/entries.test.ts`

**Step 1:** Add tests for the expansion function:

```typescript
import { describe, expect, test } from "bun:test"
import { expandMultiDayEntries } from "../entries"
import type { ServerEntry } from "../types"

function createEntry(overrides: Partial<ServerEntry>): ServerEntry {
  return {
    id: "test-id",
    calendarId: "cal-id",
    kind: "all-day",
    completed: false,
    isMultiDay: false,
    isAllDay: true,
    isTimed: false,
    isTask: false,
    title: "Test",
    description: null,
    location: null,
    color: null,
    startDate: "2024-01-15",
    endDate: "2024-01-15",
    startTime: null,
    endTime: null,
    ...overrides,
  }
}

describe("expandMultiDayEntries", () => {
  test("expands multi-day entry across all days", () => {
    const entry = createEntry({
      isMultiDay: true,
      isAllDay: false,
      startDate: "2024-01-15",
      endDate: "2024-01-18",
    })

    const result = expandMultiDayEntries([entry], "2024-01-01", "2024-01-31")

    expect(result.get("2024-01-15")).toContainEqual(entry)
    expect(result.get("2024-01-16")).toContainEqual(entry)
    expect(result.get("2024-01-17")).toContainEqual(entry)
    expect(result.get("2024-01-18")).toContainEqual(entry)
    expect(result.has("2024-01-14")).toBe(false)
    expect(result.has("2024-01-19")).toBe(false)
  })

  test("clips expansion to view bounds", () => {
    const entry = createEntry({
      isMultiDay: true,
      isAllDay: false,
      startDate: "2024-01-10",
      endDate: "2024-01-20",
    })

    const result = expandMultiDayEntries([entry], "2024-01-15", "2024-01-18")

    expect(result.size).toBe(4)
    expect(result.has("2024-01-10")).toBe(false)
    expect(result.has("2024-01-15")).toBe(true)
    expect(result.has("2024-01-18")).toBe(true)
    expect(result.has("2024-01-20")).toBe(false)
  })

  test("single-day entries are not expanded", () => {
    const entry = createEntry({
      startDate: "2024-01-15",
      endDate: "2024-01-15",
    })

    const result = expandMultiDayEntries([entry], "2024-01-01", "2024-01-31")

    expect(result.get("2024-01-15")).toContainEqual(entry)
    expect(result.has("2024-01-16")).toBe(false)
  })
})
```

**Step 2:** Run `bun test` in `crates/frontend`.

---

### Task 4.5: Update modal.ts

**File:** `crates/frontend/src/core/calendar/modal.ts`

**Step 1:** Update `EntryFormData`:

```typescript
export interface EntryFormData {
  title: string
  startDate: string
  endDate?: string
  isAllDay: boolean
  description?: string
  location?: string
  entryType: "all_day" | "timed" | "task" | "multi_day"
  startTime?: string
  endTime?: string
  completed?: boolean
}
```

**Step 2:** Update `entryToFormData`:

```typescript
export function entryToFormData(entry: ServerEntry): EntryFormData {
  let entryType: EntryFormData["entryType"] = "all_day"
  if (entry.isTimed) entryType = "timed"
  else if (entry.isTask) entryType = "task"
  else if (entry.isMultiDay) entryType = "multi_day"

  return {
    title: entry.title,
    startDate: entry.startDate,
    endDate: entry.isMultiDay ? entry.endDate : undefined,
    isAllDay: entry.isAllDay,
    description: entry.description ?? undefined,
    location: entry.location ?? undefined,
    entryType,
    startTime: entry.startTime ?? undefined,
    endTime: entry.endTime ?? undefined,
    completed: entry.isTask ? entry.completed : undefined,
  }
}
```

**Step 3:** Update `formDataToApiPayload`:

```typescript
export function formDataToApiPayload(data: EntryFormData, calendarId: string): URLSearchParams {
  const params = new URLSearchParams()
  params.set("calendar_id", calendarId)
  params.set("title", data.title)
  params.set("entry_type", data.entryType)
  params.set("start_date", data.startDate)

  if (data.entryType === "multi_day" && data.endDate) {
    params.set("end_date", data.endDate)
  }

  // ... rest unchanged
  return params
}
```

**Step 4:** Update `validateFormData` to use `startDate`.

**Step 5:** Update modal tests.

**Step 6:** Run `bun test` in `crates/frontend`.

---

### Task 4.6: Update useEntryForm hook

**File:** `crates/frontend/src/calendsync/hooks/useEntryForm.ts`

**Step 1:** Update any references from `date` to `startDate`.

**Step 2:** Run `bun run typecheck`.

---

### Task 4.7: Update remaining components

**Step 1:** Search for `entry.date` or `.date` in frontend components and update to `.startDate`.

**Step 2:** Run `bun run typecheck`.

**Step 3:** Run `bun test`.

---

### Task 4.8: Build frontend

**Step 1:** Run `bun run build` in `crates/frontend`.

**Step 2:** Fix any build errors.

---

## Phase 5: Integration Testing

### Task 5.1: Run full lint

**Step 1:** Run `cargo xtask lint`.

**Step 2:** Fix any issues.

---

### Task 5.2: Run integration tests

**Step 1:** Run `cargo xtask integration --sqlite`.

**Step 2:** Run `cargo xtask integration --dynamodb` if available.

**Step 3:** Fix any failures.

---

### Task 5.3: Manual testing

**Step 1:** Run `cargo xtask dev server --seed`.

**Step 2:** Open browser to calendar.

**Step 3:** Create a multi-day entry (e.g., "Vacation" from Jan 15-20).

**Step 4:** Verify it appears on all days in the range.

**Step 5:** Navigate to a date range that partially overlaps (e.g., Jan 18-25).

**Step 6:** Verify the multi-day entry still appears on Jan 18-20.

---

## Phase 6: Documentation

### Task 6.1: Update context documentation

**File:** `.claude/context/storage-layer.md`

**Step 1:** Update the EntryRepository section to reflect the new overlap query behavior.

**Step 2:** Update the SQLite schema section.

**Step 3:** Update the DynamoDB section.

---

## Summary

**Files to modify:**

Core:
- `crates/core/src/calendar/types.rs`
- `crates/core/src/calendar/sorting.rs`
- `crates/core/src/calendar/requests.rs`
- `crates/core/src/calendar/mock_data.rs`

Storage:
- `crates/calendsync/src/storage/sqlite/schema.rs`
- `crates/calendsync/src/storage/sqlite/conversions.rs`
- `crates/calendsync/src/storage/sqlite/repository.rs`
- `crates/calendsync/src/storage/inmemory/repository.rs`
- `crates/calendsync/src/storage/dynamodb/keys.rs`
- `crates/calendsync/src/storage/dynamodb/conversions.rs`
- `crates/calendsync/src/storage/dynamodb/repository.rs`

API:
- `crates/calendsync/src/models/entry.rs`
- `crates/calendsync/src/handlers/entries.rs`
- `crates/calendsync/src/handlers/calendar_react.rs`

Frontend:
- `crates/frontend/src/core/calendar/types.ts`
- `crates/frontend/src/core/calendar/dates.ts`
- `crates/frontend/src/core/calendar/entries.ts`
- `crates/frontend/src/core/calendar/modal.ts`
- `crates/frontend/src/calendsync/hooks/useEntryForm.ts`
