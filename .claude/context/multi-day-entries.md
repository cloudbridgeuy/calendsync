# Multi-Day Entries

This document describes the multi-day entry implementation for calendars.

## Overview

Multi-day entries span across multiple calendar days (e.g., "Spring Break" from Jan 15-20). Unlike single-day entries that have a single date, multi-day entries have a date range and appear on every day they span.

## Data Model

### CalendarEntry Structure

```rust
pub struct CalendarEntry {
    pub id: Uuid,
    pub calendar_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub kind: EntryKind,
    pub start_date: NaiveDate,  // First day of the entry
    pub end_date: NaiveDate,    // Last day of the entry (inclusive)
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### EntryKind Variants

```rust
pub enum EntryKind {
    MultiDay,                              // Spans multiple days
    AllDay,                                // Single all-day event
    Timed { start: NaiveTime, end: NaiveTime }, // Timed event
    Task { completed: bool },              // Task with completion status
}
```

For single-day entries (AllDay, Timed, Task), `start_date == end_date`.

## Date Range Queries

### Overlap Detection

When querying entries for a date range, we use overlap detection to find entries that intersect with the range:

```
entry.start_date <= query.end AND entry.end_date >= query.start
```

This ensures:
- Multi-day entries appear on all days they span
- Single-day entries appear only on their specific date
- Entries that partially overlap the query range are included

### SQLite Implementation

```sql
SELECT * FROM entries
WHERE calendar_id = ?1
  AND start_date <= ?3  -- Entry starts on or before query end
  AND end_date >= ?2    -- Entry ends on or after query start
ORDER BY start_date ASC, end_date ASC
```

### DynamoDB Implementation

DynamoDB doesn't support native overlap queries, so we use a two-phase approach:

1. **Query Phase**: Find entries where `SK <= ENTRY#{query_end}#~`
   - This gets all entries starting on or before the query end date

2. **Filter Phase**: Client-side filter where `end_date >= query_start`
   - Removes entries that end before the query start date

This over-fetches slightly but ensures correctness.

## Frontend Rendering

The backend returns one entry per multi-day event, stored in the cache by `startDate`. The frontend handles multi-day rendering through two key mechanisms:

### Entry Cache Structure

Entries are cached by their `startDate`:

```typescript
// entryCache: Map<string, ServerEntry[]>
// Key: YYYY-MM-DD (startDate), Value: entries starting on that date
```

### Filtering by Date (Calendar Component)

The `Calendar.tsx` component uses the pure `getEntriesForDate` function to filter entries for each visible day. This function checks if the queried date falls within each entry's date range:

```typescript
// From entries.ts - pure function
function getEntriesForDate(entries: ServerEntry[], dateKey: string): ServerEntry[] {
  return entries.filter((entry) => {
    if (entry.startDate === dateKey) return true
    if (entry.isMultiDay) {
      return dateKey >= entry.startDate && dateKey <= entry.endDate
    }
    return false
  })
}

// In Calendar.tsx - flatten cache and use pure function
const allEntries = useMemo(() => {
  const entries: ServerEntry[] = []
  for (const dayEntries of entryCache.values()) {
    entries.push(...dayEntries)
  }
  return entries
}, [entryCache])

const getEntriesForDate = useCallback(
  (date: Date): ServerEntry[] => {
    const key = formatDateKey(date)
    return getEntriesForDatePure(allEntries, key)
  },
  [allEntries],
)
```

**Important**: The Calendar component must use the pure `getEntriesForDate` function from `@core/calendar/entries`. A simple cache lookup by date key would miss multi-day entries stored under different start dates.

### Bulk Expansion (expandMultiDayEntries)

For pre-computing entries across a view range:

```typescript
function expandMultiDayEntries(
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
      addToDay(dayMap, entry.startDate, entry)
    }
  }

  return dayMap
}
```

## API Response Format

The server returns entries with these date-related fields:

```typescript
interface ServerEntry {
  id: string
  startDate: string      // YYYY-MM-DD format
  endDate: string        // YYYY-MM-DD format
  isMultiDay: boolean    // true if startDate !== endDate
  isAllDay: boolean      // true for AllDay kind
  isTimed: boolean       // true for Timed kind
  isTask: boolean        // true for Task kind
  startTime: string | null  // HH:MM for timed entries
  endTime: string | null    // HH:MM for timed entries
  // ... other fields
}
```

## Creating Multi-Day Entries

### Rust

```rust
let entry = CalendarEntry::multi_day(
    calendar_id,
    "Spring Break",
    NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),  // start
    NaiveDate::from_ymd_opt(2024, 3, 22).unwrap(),  // end
);
```

### API Request

```typescript
const payload = {
  calendar_id: "uuid",
  title: "Spring Break",
  entry_type: "multi_day",
  start_date: "2024-03-15",
  end_date: "2024-03-22",
}
```

## Testing Considerations

When testing multi-day entries:

1. **Boundary conditions**: Test entries at query range boundaries
2. **Partial overlap**: Entry starts before and ends within range
3. **Full containment**: Entry completely within query range
4. **Range extension**: Entry starts within and ends after range
5. **No overlap**: Entry completely outside query range

Example test:

```rust
#[test]
fn test_overlap_detection() {
    let entry = CalendarEntry::multi_day(
        calendar_id,
        "Vacation",
        date(2024, 1, 10),
        date(2024, 1, 20),
    );

    // Query for Jan 15-18 should include this entry
    let range = DateRange::new(date(2024, 1, 15), date(2024, 1, 18));
    assert!(entry.overlaps(&range));
}
```
