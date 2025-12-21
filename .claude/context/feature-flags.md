# Feature Flags Architecture

The calendsync server uses Cargo features to select storage and cache backends at compile time. This approach enables dead code elimination and avoids runtime dispatch overhead.

## Feature Matrix

| Storage   | Cache  | Features             | Use Case                    |
|-----------|--------|----------------------|-----------------------------|
| inmemory  | memory | `inmemory,memory`    | Development, zero deps      |
| sqlite    | memory | `sqlite,memory`      | Local persistence           |
| sqlite    | redis  | `sqlite,redis`       | Local with shared cache     |
| dynamodb  | memory | `dynamodb,memory`    | AWS single-instance         |
| dynamodb  | redis  | `dynamodb,redis`     | AWS production              |

## Default Configuration

The default features are `inmemory,memory`, providing a zero-dependency development experience:

```bash
# Uses inmemory storage + memory cache (no external deps)
cargo run -p calendsync

# Explicitly use SQLite
cargo run -p calendsync --no-default-features --features sqlite,memory

# Production configuration
cargo run -p calendsync --no-default-features --features dynamodb,redis
```

## Compile-Time Validation

The `state.rs` module contains compile guards that enforce valid feature combinations:

```rust
// Storage: mutually exclusive
#[cfg(all(feature = "sqlite", feature = "dynamodb"))]
compile_error!("Cannot enable both 'sqlite' and 'dynamodb' storage features");

// Cache: mutually exclusive  
#[cfg(all(feature = "memory", feature = "redis"))]
compile_error!("Cannot enable both 'memory' and 'redis' cache features");

// At least one of each required
#[cfg(not(any(feature = "inmemory", feature = "sqlite", feature = "dynamodb")))]
compile_error!("Must enable exactly one storage feature");
```

Invalid combinations fail at compile time with clear error messages.

## AppState Initialization

Each valid feature combination provides a single `AppState::new(&config)` entry point:

```rust
// Feature-gated implementation
#[cfg(all(feature = "sqlite", feature = "memory"))]
mod sqlite_memory {
    impl AppState {
        pub async fn new(config: &Config) -> Result<Self, anyhow::Error> {
            // Initialize SQLite repo + memory cache
        }
    }
}
```

The server's `main.rs` calls `AppState::new(&config)` without knowing which backend is enabled.

## Adding New Backends

To add a new storage or cache backend:

1. Add the feature to `Cargo.toml` with appropriate dependencies
2. Implement the repository/cache traits in a new module
3. Add a feature-gated module in `state.rs` with `AppState::new()`
4. Add compile guards for mutual exclusivity
5. Update this documentation

## Environment Variables

Backend-specific configuration via environment:

| Variable           | Backend   | Default                  |
|--------------------|-----------|--------------------------|
| `SQLITE_PATH`      | sqlite    | `calendsync.db`          |
| `REDIS_URL`        | redis     | `redis://localhost:6379` |
| `AWS_ENDPOINT_URL` | dynamodb  | AWS default              |
