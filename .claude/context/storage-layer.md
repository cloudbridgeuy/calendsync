# Storage Layer

This document describes the persistence layer architecture for calendsync.

## Overview

The storage layer follows the Repository Pattern with feature-gated backend implementations. Repository traits are defined in `calendsync_core::storage` (Functional Core), while concrete implementations live in `calendsync::storage` (Imperative Shell).

```
crates/core/src/storage/      # Trait definitions (pure)
├── mod.rs                    # Public exports
├── traits.rs                 # Repository traits
├── types.rs                  # DateRange, supporting types
└── error.rs                  # RepositoryError enum

crates/calendsync/src/storage/ # Implementations (I/O)
├── mod.rs                    # Feature-gated exports
├── sqlite/                   # SQLite backend
│   ├── schema.rs             # SQL DDL and queries
│   ├── conversions.rs        # Row ↔ domain conversions
│   ├── error.rs              # Error mapping
│   └── repository.rs         # Trait implementations
└── dynamodb/                 # DynamoDB backend
    ├── keys.rs               # Key generation (PK, SK, GSI)
    ├── conversions.rs        # Item ↔ domain conversions
    ├── error.rs              # AWS SDK error mapping
    └── repository.rs         # Trait implementations
```

## Feature Flags

Storage backends are mutually exclusive at compile time:

| Feature | Backend | Default | Use Case |
|---------|---------|---------|----------|
| `sqlite` | SQLite | Yes | Local development, testing |
| `dynamodb` | AWS DynamoDB | No | Production deployments |

```bash
# SQLite (default)
cargo build -p calendsync

# DynamoDB
cargo build -p calendsync --no-default-features --features dynamodb
```

The module enforces exclusivity with compile-time checks:

```rust
#[cfg(all(feature = "sqlite", feature = "dynamodb"))]
compile_error!("Features 'sqlite' and 'dynamodb' are mutually exclusive.");

#[cfg(not(any(feature = "sqlite", feature = "dynamodb")))]
compile_error!("No storage backend selected.");
```

## Repository Traits

Four repository traits handle different entity types:

### EntryRepository

```rust
#[async_trait]
pub trait EntryRepository: Send + Sync {
    async fn get_entry(&self, id: Uuid) -> Result<Option<CalendarEntry>>;
    async fn get_entries_by_calendar(&self, calendar_id: Uuid, date_range: DateRange) -> Result<Vec<CalendarEntry>>;
    async fn create_entry(&self, entry: &CalendarEntry) -> Result<()>;
    async fn update_entry(&self, entry: &CalendarEntry) -> Result<()>;
    async fn delete_entry(&self, id: Uuid) -> Result<()>;
}
```

### CalendarRepository

```rust
#[async_trait]
pub trait CalendarRepository: Send + Sync {
    async fn get_calendar(&self, id: Uuid) -> Result<Option<Calendar>>;
    async fn create_calendar(&self, calendar: &Calendar) -> Result<()>;
    async fn update_calendar(&self, calendar: &Calendar) -> Result<()>;
    async fn delete_calendar(&self, id: Uuid) -> Result<()>;
}
```

### UserRepository

```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn get_user(&self, id: Uuid) -> Result<Option<User>>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn create_user(&self, user: &User) -> Result<()>;
    async fn update_user(&self, user: &User) -> Result<()>;
}
```

### MembershipRepository

```rust
#[async_trait]
pub trait MembershipRepository: Send + Sync {
    async fn get_membership(&self, calendar_id: Uuid, user_id: Uuid) -> Result<Option<CalendarMembership>>;
    async fn get_calendars_for_user(&self, user_id: Uuid) -> Result<Vec<(Calendar, CalendarRole)>>;
    async fn get_users_for_calendar(&self, calendar_id: Uuid) -> Result<Vec<(User, CalendarRole)>>;
    async fn create_membership(&self, membership: &CalendarMembership) -> Result<()>;
    async fn delete_membership(&self, calendar_id: Uuid, user_id: Uuid) -> Result<()>;
}
```

## Error Handling

`RepositoryError` abstracts backend-specific errors:

```rust
pub enum RepositoryError {
    NotFound { entity_type: &'static str, id: String },
    AlreadyExists { entity_type: &'static str, id: String },
    ConnectionFailed(String),
    QueryFailed(String),
    Serialization(String),
    InvalidData(String),
}
```

Each backend maps its native errors to `RepositoryError`:

- **SQLite**: Maps `rusqlite::Error` and `tokio_rusqlite::Error`
- **DynamoDB**: Maps AWS SDK errors, with special handling for condition check failures

## SQLite Implementation

### Schema

Tables mirror the domain types:

```sql
CREATE TABLE IF NOT EXISTS calendars (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS entries (
    id TEXT PRIMARY KEY,
    calendar_id TEXT NOT NULL REFERENCES calendars(id),
    date TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    kind TEXT NOT NULL,  -- JSON for EntryKind
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS memberships (
    calendar_id TEXT NOT NULL REFERENCES calendars(id),
    user_id TEXT NOT NULL REFERENCES users(id),
    role TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (calendar_id, user_id)
);
```

### Usage

```rust
// File-based (persistent)
let repo = SqliteRepository::new("data.db").await?;

// In-memory (for tests)
let repo = SqliteRepository::new_in_memory().await?;
```

## DynamoDB Implementation

### Key Design

Single-table design with composite keys:

| Entity | PK | SK | GSI1 PK | GSI1 SK | GSI2 PK |
|--------|----|----|---------|---------|---------|
| Calendar | `CALENDAR#{id}` | `#METADATA` | - | - | - |
| Entry | `CALENDAR#{calendar_id}` | `ENTRY#{date}#{id}` | - | - | - |
| User | `USER#{id}` | `#METADATA` | - | - | `EMAIL#{email}` |
| Membership | `CALENDAR#{calendar_id}` | `MEMBER#{user_id}` | `USER#{user_id}` | `CALENDAR#{calendar_id}` | - |

### GSI Usage

- **GSI1**: User's calendar memberships (`get_calendars_for_user`)
- **GSI2**: User lookup by email (`get_user_by_email`)

### Usage

```rust
let client = aws_sdk_dynamodb::Client::new(&config);
let repo = DynamoDbRepository::new(client, "calendsync".to_string());
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `AWS_ENDPOINT_URL` | DynamoDB endpoint | AWS default |
| `AWS_REGION` | AWS region | `us-east-1` |
| `AWS_PROFILE` | Credentials profile | default |

## Testing

### Unit Tests

Each implementation has unit tests that run independently:

```bash
# SQLite tests (default)
cargo test -p calendsync

# DynamoDB tests
cargo test -p calendsync --no-default-features --features dynamodb
```

### Integration Tests

The `cargo xtask integration` command manages Docker and runs end-to-end tests:

```bash
# All backends
cargo xtask integration

# SQLite only
cargo xtask integration --sqlite

# DynamoDB only (starts DynamoDB Local container)
cargo xtask integration --dynamodb
```

## Adding a New Backend

1. Create `crates/calendsync/src/storage/{backend}/`
2. Implement all four repository traits
3. Add feature flag to `Cargo.toml`
4. Update `storage/mod.rs` with conditional compilation
5. Add integration tests

Follow the SQLite implementation as a reference for structure and error handling patterns.
