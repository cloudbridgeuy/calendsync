# Shared Types in calendsync_core

The `calendsync_core` crate provides shared types used by both the server (`calendsync`) and client (`calendsync_client`) crates. This enables type-safe API communication and integration testing.

## Module Structure

```
crates/core/src/calendar/
├── mod.rs          # Re-exports all public types
├── types.rs        # Domain types (Calendar, CalendarEntry, CalendarEvent, etc.)
├── requests.rs     # API request/response types
├── mock_data.rs    # Pure mock data generation
├── operations.rs   # Pure calendar operations
├── sorting.rs      # Pure sorting functions
└── error.rs        # CalendarError enum
```

## Domain Types (`types.rs`)

### Core Entities

| Type | Description |
|------|-------------|
| `User` | User with id, name, email |
| `Calendar` | Named calendar with color and description |
| `CalendarEntry` | Event/task with title, date, kind, location |
| `CalendarMembership` | Links user to calendar with role |

### Entry Kinds

```rust
pub enum EntryKind {
    MultiDay { start: NaiveDate, end: NaiveDate },
    AllDay,
    Timed { start: NaiveTime, end: NaiveTime },
    Task { completed: bool },
}
```

### SSE Events

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CalendarEvent {
    EntryAdded { entry: CalendarEntry, date: String },
    EntryUpdated { entry: CalendarEntry, date: String },
    EntryDeleted { entry_id: Uuid, date: String },
}
```

The `CalendarEvent` enum has both `Serialize` and `Deserialize`, enabling:
- Server to serialize events for SSE streams
- Client to deserialize events for integration testing
- Type-safe round-trip verification

## API Request Types (`requests.rs`)

All request types derive `Serialize` + `Deserialize` for bidirectional use.

### Calendar Requests

```rust
// Create
let req = CreateCalendarRequest::new("Work")
    .with_color("#FF0000")
    .with_description("Work calendar");
let calendar = req.into_calendar();

// Update
let update = UpdateCalendarRequest::new()
    .with_name("New Name")
    .with_color("#00FF00");
update.apply_to(&mut calendar);
```

### Entry Requests

```rust
// Create all-day event
let req = CreateEntryRequest::all_day(calendar_id, "Birthday", date)
    .with_description("Party at 7pm")
    .with_location("123 Main St");
let entry = req.into_entry().unwrap();

// Create timed event
let req = CreateEntryRequest::timed(
    calendar_id,
    "Meeting",
    date,
    NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
    NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
);

// Create task
let req = CreateEntryRequest::task(calendar_id, "Buy groceries", date);

// Create multi-day event
let req = CreateEntryRequest::multi_day(calendar_id, "Vacation", start, end);

// Update
let update = UpdateEntryRequest::new()
    .with_title("Updated Title")
    .with_completed(true);
update.apply_to(&mut entry);
```

### Query Parameters

```rust
let query = ListEntriesQuery::new()
    .for_calendar(calendar_id)
    .with_range(start_date, end_date);

// Or with highlighted day
let query = ListEntriesQuery::new()
    .for_calendar(calendar_id)
    .with_highlighted_day(today, 3, 3); // 3 days before, 3 after
```

### Entry Type Enum

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryType {
    AllDay,
    Timed,
    Task,
    MultiDay,
}

// Convert from EntryKind
let entry_type = EntryType::from_kind(&entry.kind);
```

## Mock Data Generation (`mock_data.rs`)

Pure functions for generating test data:

```rust
// Generate 100 entries spread around today
let entries = generate_seed_entries(calendar_id, today, 100);

// Distribution: ~15% multi-day, ~20% all-day, ~45% timed, ~20% tasks
// Date range: 30 days before to 30 days after center_date

// Format entry kind for display
let display = format_entry_kind(&entry.kind); // "multi-day", "all-day", "timed", "task"
```

## Server vs Client Usage

### Server (calendsync)

The server has wrapper types with custom deserializers for form handling:

```rust
// Server uses CreateEntry (form-specific deserializers)
// - Empty strings → None
// - Default color when not provided

pub struct CreateEntry {
    pub entry_type: ServerEntryType, // CLI-compatible enum
    // ... with custom deserializers
}

impl CreateEntry {
    pub fn into_entry(self) -> Option<CalendarEntry> { ... }
}
```

### Client (calendsync_client)

The client imports types directly from core:

```rust
use calendsync_core::calendar::{
    CreateCalendarRequest, UpdateCalendarRequest,
    CreateEntryRequest, UpdateEntryRequest,
    ListEntriesQuery, EntryType,
    CalendarEvent,
};
```

## Integration Testing Power

With shared types, the client CLI enables integration testing:

```bash
# Create calendar and entry
calendsync-client calendars create --name "Test"
calendsync-client entries create --calendar-id $CAL --title "Event" --date 2024-01-15 --entry-type all_day

# Watch for SSE events (type-safe deserialization)
calendsync-client events watch $CAL --timeout 5s

# Verify round-trip
calendsync-client entries list --calendar-id $CAL | jq '.[] | .title'
```

## Adding New Types

When adding new shared types:

1. Add to appropriate file in `crates/core/src/calendar/`
2. Derive `Serialize, Deserialize` for API types
3. Export from `mod.rs`
4. Add builder methods for ergonomic construction
5. Add `into_*` or `apply_to` conversion methods
6. Write unit tests in the same file

Example:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRequest {
    pub required_field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional_field: Option<String>,
}

impl NewRequest {
    pub fn new(required: impl Into<String>) -> Self {
        Self {
            required_field: required.into(),
            optional_field: None,
        }
    }

    pub fn with_optional(mut self, value: impl Into<String>) -> Self {
        self.optional_field = Some(value.into());
        self
    }
}
```
