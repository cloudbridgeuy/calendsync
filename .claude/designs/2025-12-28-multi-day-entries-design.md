# Multi-Day Entries Design

## Overview

This design addresses the problem of querying and rendering multi-day calendar entries. The current implementation stores a single `date` field per entry, which fails to capture entries that span multiple days when querying a date range.

## Problem Statement

When querying entries between a time range, multi-day events on the edges of the query are missed. For example:

- Entry: "Pride Month" spanning November 1-30
- Query: October 25 to November 7
- Current behavior: Entry is missed because its `date` field (November 1) may not match the query logic correctly

Additional issues:

1. Frontend renders multi-day entries only on their first day
2. Edits to multi-day entries do not propagate across all displayed days
3. No foundation for future recurrence patterns

## Solution

Replace point-in-time queries with range overlap queries. All entries have a `start_date` and `end_date`. The query condition becomes:

```
entry.start_date <= query.end AND entry.end_date >= query.start
```

The backend returns one entry per multi-day event. The frontend expands it across all days in its range.

## Data Model Changes

### CalendarEntry Refactor

The `date` field is replaced by `start_date` and `end_date`:

```rust
pub struct CalendarEntry {
    pub id: Uuid,
    pub calendar_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub kind: EntryKind,
    pub start_date: NaiveDate,  // renamed from `date`
    pub end_date: NaiveDate,    // new field
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### EntryKind Simplification

Remove the date range from `MultiDay` since it now lives in the parent struct:

```rust
pub enum EntryKind {
    MultiDay,                              // was: MultiDay { start, end }
    AllDay,
    Timed { start: NaiveTime, end: NaiveTime },
    Task { completed: bool },
}
```

### Invariants

- Single-day entries: `start_date == end_date`
- Multi-day entries: `start_date < end_date`
- Timed entries: `start_date == end_date` (times are within a single day)

## Query Strategy

### The Overlap Condition

For an entry with range `[entry_start, entry_end]` to overlap with query range `[query_start, query_end]`:

```
entry_start <= query_end AND entry_end >= query_start
```

### SQLite Implementation

```sql
SELECT id, calendar_id, title, description, location, kind,
       start_date, end_date, color, created_at, updated_at
FROM entries
WHERE calendar_id = ?1
  AND start_date <= ?3  -- entry starts before query ends
  AND end_date >= ?2    -- entry ends after query starts
ORDER BY start_date ASC, end_date ASC
```

### DynamoDB Implementation

Change the SK pattern from `ENTRY#{date}#{id}` to `ENTRY#{start_date}#{id}`.

Query strategy:

1. `KeyConditionExpression`: `PK = :pk AND SK <= :max_sk` where `:max_sk = ENTRY#{query_end}#ZZZZZZ`
2. `FilterExpression`: `end_date >= :query_start`

This fetches all entries starting before the query ends, then filters to those ending after the query starts. Over-fetching is acceptable; future caching mechanisms will mitigate scanning costs.

### InMemory Implementation

```rust
entries.iter().filter(|e| {
    e.start_date <= date_range.end && e.end_date >= date_range.start
})
```

## Storage Layer Changes

### SQLite Schema

Replace the current `entries` table definition:

```sql
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

CREATE INDEX IF NOT EXISTS idx_entries_calendar_id ON entries(calendar_id);
CREATE INDEX IF NOT EXISTS idx_entries_calendar_range ON entries(calendar_id, start_date, end_date);
```

The `date` column is replaced by `start_date` and `end_date`. No migration needed since SQLite is for development only.

### DynamoDB Attributes

| Attribute | Type | Description |
|-----------|------|-------------|
| `start_date` | String (ISO date) | Entry start date |
| `end_date` | String (ISO date) | Entry end date |

The SK pattern changes to `ENTRY#{start_date}#{id}`.

## API Changes

### Request Types

```rust
#[derive(Debug, Deserialize)]
pub struct CreateEntry {
    pub calendar_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub color: Option<String>,
    pub entry_type: String,        // "all_day", "timed", "task", "multi_day"
    pub start_date: NaiveDate,     // renamed from `date`
    pub end_date: Option<NaiveDate>, // required for multi_day
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub completed: Option<bool>,
}
```

### Response Format

```rust
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
```

Removed fields: `date`, `multiDayStart`, `multiDayEnd`, `multiDayStartDate`, `multiDayEndDate`.

## Frontend Changes

### ServerEntry Type

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

### Expansion Logic

Add pure function to expand multi-day entries:

```typescript
export function expandMultiDayEntries(
  entries: ServerEntry[],
  viewStart: string,
  viewEnd: string
): Map<string, ServerEntry[]> {
  const dayMap = new Map<string, ServerEntry[]>()

  for (const entry of entries) {
    if (entry.isMultiDay) {
      const start = maxDate(entry.startDate, viewStart)
      const end = minDate(entry.endDate, viewEnd)

      for (const date of dateRange(start, end)) {
        addToDay(dayMap, date, entry)
      }
    } else {
      addToDay(dayMap, entry.startDate, entry)
    }
  }

  return dayMap
}
```

### Date Helpers

```typescript
export function dateRange(start: string, end: string): string[] {
  const dates: string[] = []
  let current = start
  while (current <= end) {
    dates.push(current)
    current = addDays(current, 1)
  }
  return dates
}

export function maxDate(a: string, b: string): string {
  return a > b ? a : b
}

export function minDate(a: string, b: string): string {
  return a < b ? a : b
}
```

## Testing Strategy

### Core Unit Tests

Test type invariants and constructors:

```rust
#[test]
fn test_single_day_entry_has_equal_dates() {
    let entry = CalendarEntry::all_day(
        Uuid::new_v4(),
        "Meeting",
        NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
    );
    assert_eq!(entry.start_date, entry.end_date);
}

#[test]
fn test_multi_day_entry_has_different_dates() {
    let start = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 6, 20).unwrap();
    let entry = CalendarEntry::multi_day(Uuid::new_v4(), "Vacation", start, end);

    assert!(entry.start_date < entry.end_date);
}
```

### Storage Overlap Tests

Test overlap queries for each backend:

```rust
#[tokio::test]
async fn test_query_finds_multi_day_spanning_entire_range() {
    let repo = create_test_repo().await;
    let calendar_id = Uuid::new_v4();

    // Entry: Jan 1-31 (entire month)
    let entry = CalendarEntry::multi_day(
        calendar_id, "Project",
        date(2024, 1, 1), date(2024, 1, 31),
    );
    repo.create_entry(&entry).await.unwrap();

    // Query: Jan 15-16 (completely inside entry range)
    let range = DateRange::new(date(2024, 1, 15), date(2024, 1, 16)).unwrap();
    let results = repo.get_entries_by_calendar(calendar_id, range).await.unwrap();

    assert_eq!(results.len(), 1);
}

#[tokio::test]
async fn test_query_excludes_non_overlapping_entries() {
    let repo = create_test_repo().await;
    let calendar_id = Uuid::new_v4();

    // Entry: Jan 1-5
    let entry = CalendarEntry::multi_day(
        calendar_id, "Past Event",
        date(2024, 1, 1), date(2024, 1, 5),
    );
    repo.create_entry(&entry).await.unwrap();

    // Query: Jan 10-15 (no overlap)
    let range = DateRange::new(date(2024, 1, 10), date(2024, 1, 15)).unwrap();
    let results = repo.get_entries_by_calendar(calendar_id, range).await.unwrap();

    assert_eq!(results.len(), 0);
}
```

### Frontend Unit Tests

Test expansion logic:

```typescript
describe("expandMultiDayEntries", () => {
  test("expands multi-day entry across all days", () => {
    const entry = createServerEntry({
      isMultiDay: true,
      startDate: "2024-01-15",
      endDate: "2024-01-18",
    })

    const result = expandMultiDayEntries([entry], "2024-01-01", "2024-01-31")

    expect(result.get("2024-01-15")).toContainEqual(entry)
    expect(result.get("2024-01-16")).toContainEqual(entry)
    expect(result.get("2024-01-17")).toContainEqual(entry)
    expect(result.get("2024-01-18")).toContainEqual(entry)
  })

  test("clips expansion to view bounds", () => {
    const entry = createServerEntry({
      isMultiDay: true,
      startDate: "2024-01-10",
      endDate: "2024-01-20",
    })

    const result = expandMultiDayEntries([entry], "2024-01-15", "2024-01-18")

    expect(result.size).toBe(4)
    expect(result.has("2024-01-10")).toBe(false)
  })
})
```

## Future Considerations

This design lays the foundation for rule-based entries (recurrence patterns). The key insight is that entries are defined by a validity range, not a single point in time. Future work could:

1. Add a `RecurrenceRule` type that generates occurrences within a date range
2. Query recurrence rules using the same overlap logic
3. Expand rules to individual occurrences in the frontend

The pattern of "query by range overlap, expand in frontend" scales to more complex recurrence patterns.
