# CalendSync Web Server

The main axum web server providing REST API and React SSR.

## Tech Stack

- **Axum** - Web framework
- **React 19** - Frontend UI with SSR
- **deno_core** - JavaScript runtime for SSR
- **Tokio** - Async runtime
- **Tower-HTTP** - Middleware (CORS, tracing, timeouts)

## Storage Backends

The server supports two storage backends, selected at compile time via feature flags:

| Feature | Backend | Dependencies | Use Case |
|---------|---------|--------------|----------|
| `sqlite` (default) | SQLite | `rusqlite`, `tokio-rusqlite` | Local development, single-instance deployments |
| `dynamodb` | AWS DynamoDB | `aws-sdk-dynamodb`, `aws-config` | Production, AWS deployments |

### Building with Different Backends

```bash
# SQLite (default)
cargo build -p calendsync

# DynamoDB
cargo build -p calendsync --no-default-features --features dynamodb
```

### DynamoDB Configuration

When using the DynamoDB backend, set these environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `AWS_ENDPOINT_URL` | DynamoDB endpoint | AWS default |
| `AWS_REGION` | AWS region | `us-east-1` |
| `AWS_PROFILE` | AWS credentials profile | default |

For local development with DynamoDB Local:

```bash
# Start DynamoDB Local
docker compose up -d

# Run with DynamoDB backend
AWS_ENDPOINT_URL=http://localhost:8000 \
cargo run -p calendsync --no-default-features --features dynamodb
```

## Cache Backends

The application supports two cache backends, selected at compile time:

| Feature | Backend | Default | Use Case |
|---------|---------|---------|----------|
| `memory` | In-Memory | Yes | Single-instance, local development |
| `redis` | Redis | No | Multi-instance, production |

### Feature Combinations

```bash
# Local development (default: sqlite + memory)
cargo build -p calendsync

# SQLite with Redis cache
cargo build -p calendsync --no-default-features --features sqlite,redis

# DynamoDB with memory cache
cargo build -p calendsync --no-default-features --features dynamodb,memory

# Production (DynamoDB + Redis)
cargo build -p calendsync --release --no-default-features --features dynamodb,redis
```

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `REDIS_URL` | Redis connection URL | `redis://localhost:6379` |

## Running the Server

```bash
# Standard mode (SQLite)
cargo run -p calendsync

# With auto-reload
systemfd --no-pid -s http::3000 -- cargo watch -x 'run -p calendsync'
```

Server runs at `http://localhost:3000`

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/calendar/{id}` | React SSR calendar page |
| GET | `/calendar/{id}/entry` | Calendar with create modal |
| GET | `/calendar/{id}/entry?entry_id={id}` | Calendar with edit modal |
| GET | `/api/calendar-entries` | Get entries for date range |
| POST | `/api/entries` | Create entry |
| PUT | `/api/entries/{id}` | Update entry |
| DELETE | `/api/entries/{id}` | Delete entry |
| GET | `/api/events?calendar_id={id}` | SSE event stream |
| GET | `/healthz` | Health check |

## Architecture

The server uses React SSR via deno_core with a repository-based storage layer:

1. Request comes in for `/calendar/{id}`
2. Server fetches calendar data from repository (SQLite/DynamoDB with cache)
3. `SsrPool` renders React to HTML using deno_core workers
4. HTML returned with embedded initial data
5. Client-side React hydrates for interactivity
6. SSE connection provides real-time updates via CachePubSub

See `crates/ssr/` and `crates/ssr_core/` for SSR implementation details.
See `.claude/context/storage-layer.md` for storage architecture details.

## Project Structure

```
src/
├── main.rs         # Entry point, graceful shutdown
├── app.rs          # Router, middleware
├── config.rs       # Environment-based configuration
├── state.rs        # AppState with repository trait objects
├── mock_data.rs    # Demo data generation
├── handlers/
│   ├── entries.rs      # Entry CRUD (uses repositories)
│   ├── calendar_react.rs  # React SSR handler
│   ├── events.rs       # SSE handler
│   ├── error.rs        # AppError type
│   └── health.rs       # Health endpoints
├── models/
│   └── entry.rs    # Request types
├── cache/          # Cache backend implementations
│   ├── mod.rs          # Feature-gated exports
│   ├── memory/         # In-memory LRU cache
│   └── redis_impl/     # Redis cache + pub/sub
└── storage/        # Storage backend implementations
    ├── mod.rs          # Feature-gated module exports
    ├── sqlite/         # SQLite implementation
    ├── dynamodb/       # DynamoDB implementation
    ├── inmemory/       # In-memory for testing
    └── cached/         # Cache-aside decorators
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Logging level | `info` |
| `CACHE_TTL_SECONDS` | Cache TTL | `300` |
| `CACHE_MAX_ENTRIES` | Max cache entries | `10000` |
| `EVENT_HISTORY_MAX_SIZE` | SSE event history | `1000` |
| `SQLITE_PATH` | SQLite database path | `calendsync.db` |
| `REDIS_URL` | Redis connection URL | `redis://localhost:6379` |

```bash
RUST_LOG=debug cargo run -p calendsync
```

## Testing

```bash
# Run all tests (SQLite backend)
cargo test -p calendsync

# Run DynamoDB tests
cargo test -p calendsync --no-default-features --features dynamodb

# Run with output
cargo test -p calendsync -- --nocapture

# Run integration tests (includes Docker management)
cargo xtask integration

# Run only SQLite integration tests
cargo xtask integration --sqlite

# Run only DynamoDB integration tests
cargo xtask integration --dynamodb
```
