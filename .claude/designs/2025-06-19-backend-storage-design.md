# Backend Storage Design

This document describes the persistence layer architecture for calendsync, supporting multiple storage backends and caching strategies.

## Overview

The persistence layer introduces a repository abstraction that supports DynamoDB and SQLite as storage backends, with Redis and in-memory options for caching. The architecture follows the Functional Core - Imperative Shell pattern: trait definitions and pure functions live in `calendsync_core`, while implementations reside in `calendsync`.

Feature flags control backend selection at compile time. Each category (storage and cache) enforces mutual exclusivity, preventing conflicting implementations from building together. The default configuration uses SQLite with in-memory caching, requiring no external dependencies for local development. Production deployments enable DynamoDB and Redis through explicit feature flags.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Handlers (axum)                          │
└─────────────────────────────┬───────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│                  CachedRepository<R, C>                     │
│            (decorator: cache layer wraps storage)           │
│         Publishes SSE events on write operations            │
└─────────────────────────────┬───────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│                    Repository Traits                        │
│   EntryRepository, CalendarRepository, UserRepository,      │
│                   MembershipRepository                      │
└─────────────────────────────┬───────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
┌─────────────────────────┐     ┌─────────────────────────┐
│   DynamoDbRepository    │     │    SqliteRepository     │
│  (feature: dynamodb)    │     │    (feature: sqlite)    │
└─────────────────────────┘     └─────────────────────────┘
```

## Design Decisions

### Database Selection

The system supports two storage backends to accommodate different deployment scenarios. DynamoDB serves as the primary target for AWS/SaaS deployments, offering managed infrastructure and horizontal scaling. SQLite provides a zero-dependency option for local development and simpler deployments, with SQL syntax that translates to PostgreSQL if migration becomes necessary.

### Caching Strategy

The application's read-heavy workload (users view calendars constantly, but write only tens of updates per week) justifies aggressive caching. Redis provides distributed caching and pub/sub messaging for multi-instance deployments. An in-memory cache serves single-instance development scenarios.

Cache invalidation follows a strict sequence on writes: persist to storage first, invalidate cache second, publish event third. This ordering ensures database consistency even when cache operations fail.

### Consistency Guarantees

Writers see their changes immediately. Other users receive updates within three seconds via SSE propagation. The cache layer invalidates affected entries on every write, and pub/sub messaging broadcasts events across server instances.

### Repository Pattern

Each repository trait represents a bounded domain. The `async-trait` crate enables trait object usage, allowing runtime selection of implementations without generic parameter propagation through the codebase. A single `RepositoryError` enum abstracts backend-specific errors, enabling handlers to pattern-match on failure modes for appropriate HTTP status codes.

### Decorator Pattern

The cache layer wraps repositories without their knowledge. Repository implementations remain pure storage concerns, unaware of caching logic. This separation allows testing storage implementations in isolation and composing cache behavior optionally.

## Repository Traits

### Entry Repository

```rust
#[async_trait]
pub trait EntryRepository: Send + Sync {
    async fn get_entry(&self, id: Uuid) -> Result<Option<CalendarEntry>, RepositoryError>;
    
    async fn get_entries_by_calendar(
        &self,
        calendar_id: Uuid,
        date_range: DateRange,
    ) -> Result<Vec<CalendarEntry>, RepositoryError>;
    
    async fn create_entry(&self, entry: &CalendarEntry) -> Result<(), RepositoryError>;
    
    async fn update_entry(&self, entry: &CalendarEntry) -> Result<(), RepositoryError>;
    
    async fn delete_entry(&self, id: Uuid) -> Result<(), RepositoryError>;
}
```

### Calendar Repository

```rust
#[async_trait]
pub trait CalendarRepository: Send + Sync {
    async fn get_calendar(&self, id: Uuid) -> Result<Option<Calendar>, RepositoryError>;
    
    async fn create_calendar(&self, calendar: &Calendar) -> Result<(), RepositoryError>;
    
    async fn update_calendar(&self, calendar: &Calendar) -> Result<(), RepositoryError>;
    
    async fn delete_calendar(&self, id: Uuid) -> Result<(), RepositoryError>;
}
```

### User Repository

```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn get_user(&self, id: Uuid) -> Result<Option<User>, RepositoryError>;
    
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, RepositoryError>;
    
    async fn create_user(&self, user: &User) -> Result<(), RepositoryError>;
    
    async fn update_user(&self, user: &User) -> Result<(), RepositoryError>;
}
```

### Membership Repository

```rust
#[async_trait]
pub trait MembershipRepository: Send + Sync {
    async fn get_membership(
        &self,
        calendar_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<CalendarMembership>, RepositoryError>;
    
    async fn get_calendars_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(Calendar, CalendarRole)>, RepositoryError>;
    
    async fn get_users_for_calendar(
        &self,
        calendar_id: Uuid,
    ) -> Result<Vec<(User, CalendarRole)>, RepositoryError>;
    
    async fn create_membership(
        &self,
        membership: &CalendarMembership,
    ) -> Result<(), RepositoryError>;
    
    async fn delete_membership(
        &self,
        calendar_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), RepositoryError>;
}
```

### Supporting Types

```rust
/// Date range for entry queries.
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl DateRange {
    pub fn new(start: NaiveDate, end: NaiveDate) -> Self {
        Self { start, end }
    }
    
    /// Creates a range for a single month.
    pub fn month(year: i32, month: u32) -> Self {
        let start = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let end = start
            .checked_add_months(chrono::Months::new(1))
            .unwrap()
            .pred_opt()
            .unwrap();
        Self { start, end }
    }
}
```

## Error Types

### Repository Error

```rust
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("entity not found: {entity_type} with id {id}")]
    NotFound {
        entity_type: &'static str,
        id: String,
    },
    
    #[error("entity already exists: {entity_type} with id {id}")]
    AlreadyExists {
        entity_type: &'static str,
        id: String,
    },
    
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("query failed: {0}")]
    QueryFailed(String),
    
    #[error("serialization error: {0}")]
    Serialization(String),
    
    #[error("invalid data: {0}")]
    InvalidData(String),
}

pub type Result<T> = std::result::Result<T, RepositoryError>;
```

### Cache Error

```rust
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("operation failed: {0}")]
    OperationFailed(String),
    
    #[error("serialization error: {0}")]
    Serialization(String),
    
    #[error("publish failed: {0}")]
    PublishFailed(String),
}

pub type Result<T> = std::result::Result<T, CacheError>;
```

## Cache Traits

### Cache Trait (Key-Value Storage)

```rust
#[async_trait]
pub trait Cache: Send + Sync {
    /// Retrieves a value by key, returning None if not found or expired.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    
    /// Stores a value with an optional time-to-live.
    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<()>;
    
    /// Removes a value by key.
    async fn delete(&self, key: &str) -> Result<()>;
    
    /// Removes all values matching a key pattern (e.g., "calendar:123:*").
    async fn delete_pattern(&self, pattern: &str) -> Result<()>;
}
```

### PubSub Trait (Cross-Instance Messaging)

```rust
#[async_trait]
pub trait CachePubSub: Send + Sync {
    /// Publishes a calendar event to all subscribers.
    async fn publish(&self, calendar_id: Uuid, event: &CalendarEvent) -> Result<()>;
    
    /// Subscribes to calendar events, returning a broadcast receiver.
    async fn subscribe(&self, calendar_id: Uuid) -> Result<broadcast::Receiver<CalendarEvent>>;
}
```

### Combined Trait

```rust
/// A cache that provides both storage and pub/sub capabilities.
pub trait FullCache: Cache + CachePubSub {}

impl<T: Cache + CachePubSub> FullCache for T {}
```

### Cache Key Functions

```rust
/// Generates cache key for a single entry.
pub fn entry_key(entry_id: Uuid) -> String {
    format!("entry:{}", entry_id)
}

/// Generates cache key for calendar entries within a date range.
pub fn calendar_entries_key(calendar_id: Uuid, start: NaiveDate, end: NaiveDate) -> String {
    format!("calendar:{}:entries:{}:{}", calendar_id, start, end)
}

/// Generates cache key pattern for all entries in a calendar.
pub fn calendar_entries_pattern(calendar_id: Uuid) -> String {
    format!("calendar:{}:entries:*", calendar_id)
}

/// Generates cache key for a calendar.
pub fn calendar_key(calendar_id: Uuid) -> String {
    format!("calendar:{}", calendar_id)
}

/// Generates cache key for a user.
pub fn user_key(user_id: Uuid) -> String {
    format!("user:{}", user_id)
}

/// Generates pub/sub channel name for calendar events.
pub fn calendar_channel(calendar_id: Uuid) -> String {
    format!("channel:calendar:{}", calendar_id)
}
```

## Cached Repository Decorator

The decorator wraps any repository implementation, intercepting operations to manage cache coherency.

```rust
pub struct CachedEntryRepository<R, C> {
    repository: Arc<R>,
    cache: Arc<C>,
    ttl: Duration,
}

impl<R, C> CachedEntryRepository<R, C>
where
    R: EntryRepository,
    C: Cache + CachePubSub,
{
    pub fn new(repository: Arc<R>, cache: Arc<C>, ttl: Duration) -> Self {
        Self { repository, cache, ttl }
    }
}

#[async_trait]
impl<R, C> EntryRepository for CachedEntryRepository<R, C>
where
    R: EntryRepository,
    C: Cache + CachePubSub,
{
    async fn get_entry(&self, id: Uuid) -> Result<Option<CalendarEntry>> {
        let cache_key = keys::entry_key(id);
        
        // Check cache first
        if let Ok(Some(bytes)) = self.cache.get(&cache_key).await {
            if let Ok(entry) = serde_json::from_slice(&bytes) {
                return Ok(Some(entry));
            }
        }
        
        // Cache miss: fetch from repository
        let entry = self.repository.get_entry(id).await?;
        
        // Populate cache on hit
        if let Some(ref e) = entry {
            if let Ok(bytes) = serde_json::to_vec(e) {
                let _ = self.cache.set(&cache_key, &bytes, Some(self.ttl)).await;
            }
        }
        
        Ok(entry)
    }
    
    async fn create_entry(&self, entry: &CalendarEntry) -> Result<()> {
        // 1. Persist to storage
        self.repository.create_entry(entry).await?;
        
        // 2. Invalidate calendar entries cache
        let pattern = keys::calendar_entries_pattern(entry.calendar_id);
        let _ = self.cache.delete_pattern(&pattern).await;
        
        // 3. Publish event for cross-instance propagation
        let event = CalendarEvent::entry_added(entry.clone());
        let _ = self.cache.publish(entry.calendar_id, &event).await;
        
        Ok(())
    }
    
    // Similar implementations for update_entry, delete_entry, get_entries_by_calendar
}
```

## Feature Flag Configuration

### Cargo.toml (calendsync)

```toml
[features]
default = ["sqlite", "memory"]

# Storage backends (mutually exclusive)
sqlite = ["dep:rusqlite", "dep:tokio-rusqlite"]
dynamodb = ["dep:aws-sdk-dynamodb", "dep:aws-config"]

# Cache backends (mutually exclusive)
memory = []
redis = ["dep:redis"]

[dependencies]
calendsync_core = { path = "../core" }

# SQLite (optional)
rusqlite = { version = "0.31", features = ["bundled"], optional = true }
tokio-rusqlite = { version = "0.5", optional = true }

# DynamoDB (optional)
aws-sdk-dynamodb = { version = "1.0", optional = true }
aws-config = { version = "1.0", optional = true }

# Redis (optional)
redis = { version = "0.25", features = ["tokio-comp", "connection-manager"], optional = true }
```

### Compile-Time Exclusivity

```rust
// calendsync/src/storage/mod.rs

#[cfg(all(feature = "sqlite", feature = "dynamodb"))]
compile_error!("Features 'sqlite' and 'dynamodb' are mutually exclusive.");

#[cfg(not(any(feature = "sqlite", feature = "dynamodb")))]
compile_error!("No storage backend selected. Enable 'sqlite' or 'dynamodb'.");

#[cfg(all(feature = "memory", feature = "redis"))]
compile_error!("Features 'memory' and 'redis' are mutually exclusive.");

#[cfg(not(any(feature = "memory", feature = "redis")))]
compile_error!("No cache backend selected. Enable 'memory' or 'redis'.");
```

### Valid Combinations

| Deployment | Storage | Cache | Use Case |
|------------|---------|-------|----------|
| Local dev (simple) | `sqlite` | `memory` | Single instance, no Docker deps |
| Local dev (Redis) | `sqlite` | `redis` | Testing Redis integration |
| DynamoDB testing | `dynamodb` | `memory` | Integration tests with DynamoDB Local |
| Production | `dynamodb` | `redis` | AWS deployment |

### Build Commands

```bash
# Local development (default)
cargo build -p calendsync

# Local development with Redis
cargo build -p calendsync --no-default-features --features sqlite,redis

# DynamoDB integration testing
cargo build -p calendsync --no-default-features --features dynamodb,memory

# Production deployment
cargo build -p calendsync --release --no-default-features --features dynamodb,redis
```

## Implementation Phases

### Phase 1: Foundation (Functional Core)

Establishes trait definitions, error types, and pure functions in `calendsync_core`. All tasks execute in parallel.

**Parallel Tasks:**

1. **Storage Traits Module**
   - Create `crates/core/src/storage/mod.rs`
   - Create `crates/core/src/storage/error.rs` with `RepositoryError`
   - Create `crates/core/src/storage/traits.rs` with repository traits
   - Create `crates/core/src/storage/types.rs` with `DateRange`
   - Add unit tests for `DateRange` methods
   - Export module from `crates/core/src/lib.rs`

2. **Cache Traits Module**
   - Create `crates/core/src/cache/mod.rs`
   - Create `crates/core/src/cache/error.rs` with `CacheError`
   - Create `crates/core/src/cache/traits.rs` with cache traits
   - Create `crates/core/src/cache/keys.rs` with key generation functions
   - Add unit tests for key generation functions
   - Export module from `crates/core/src/lib.rs`

3. **Update Core Cargo.toml**
   - Add `async-trait` dependency
   - Add `thiserror` dependency if not present
   - Ensure `tokio` with `sync` feature

### Phase 2: Storage Implementations (Imperative Shell)

Implements concrete repository backends. SQLite and DynamoDB implementations execute in parallel.

**Parallel Tasks:**

1. **SQLite Repository**
   - Create `crates/calendsync/src/storage/mod.rs` with feature checks
   - Create `crates/calendsync/src/storage/sqlite/mod.rs`
   - Create `crates/calendsync/src/storage/sqlite/repository.rs`
   - Create `crates/calendsync/src/storage/sqlite/schema.rs`
   - Create `crates/calendsync/src/storage/sqlite/conversions.rs`
   - Add integration tests with in-memory SQLite
   - Update `Cargo.toml` with SQLite dependencies

2. **DynamoDB Repository**
   - Create `crates/calendsync/src/storage/dynamodb/mod.rs`
   - Create `crates/calendsync/src/storage/dynamodb/repository.rs`
   - Create `crates/calendsync/src/storage/dynamodb/keys.rs`
   - Create `crates/calendsync/src/storage/dynamodb/conversions.rs`
   - Add integration tests with DynamoDB Local
   - Update `Cargo.toml` with AWS SDK dependencies

### Phase 3: Cache Implementations (Imperative Shell)

Implements cache backends. In-Memory and Redis implementations execute in parallel.

**Parallel Tasks:**

1. **In-Memory Cache**
   - Create `crates/calendsync/src/cache/mod.rs` with feature checks
   - Create `crates/calendsync/src/cache/memory/mod.rs`
   - Create `crates/calendsync/src/cache/memory/cache.rs`
   - Create `crates/calendsync/src/cache/memory/pubsub.rs`
   - Add unit tests for cache operations and TTL
   - Add unit tests for pub/sub delivery

2. **Redis Cache**
   - Create `crates/calendsync/src/cache/redis_impl/mod.rs`
   - Create `crates/calendsync/src/cache/redis_impl/cache.rs`
   - Create `crates/calendsync/src/cache/redis_impl/pubsub.rs`
   - Add integration tests with Redis in Docker
   - Update `Cargo.toml` with Redis dependency

### Phase 4: Integration (Imperative Shell)

Wires components together. Tasks execute sequentially due to dependencies.

**Sequential Tasks:**

1. **Cached Repository Decorators**
   - Create `crates/calendsync/src/storage/cached/mod.rs`
   - Create decorators for all four repository traits
   - Add integration tests for cache invalidation

2. **AppState Refactoring**
   - Update `crates/calendsync/src/state.rs` to use traits
   - Create factory functions per feature combination
   - Maintain `with_demo_data()` compatibility
   - Update SSE handler for `CachePubSub` trait

3. **Handler Updates**
   - Update handlers to use repository traits
   - Remove direct `HashMap` access
   - Map errors to HTTP status codes

4. **Configuration**
   - Create `crates/calendsync/src/config.rs`
   - Support environment variables
   - Document options

5. **Documentation**
   - Update `crates/calendsync/README.md`
   - Update `CLAUDE.md`
   - Create `.claude/context/storage-layer.md`

### Phase Dependencies

```
Phase 1 (Foundation)
    │
    ├──────────────────┐
    ▼                  ▼
Phase 2 (Storage)  Phase 3 (Cache)
    │                  │
    └────────┬─────────┘
             ▼
      Phase 4 (Integration)
```

Phases 2 and 3 execute in parallel after Phase 1 completes. Phase 4 requires both to finish.
