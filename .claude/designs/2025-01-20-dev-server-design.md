# Enhanced `cargo xtask dev server` Command

This document describes the design for enhancing the development server command with configurable storage and cache backends, automatic container orchestration, and HTTP-based data seeding.

## Context

The current `cargo xtask dev web` command runs the calendsync server with hardcoded SQLite storage and in-memory cache. Developers who want to test against DynamoDB or Redis must manually start containers and configure environment variables. The server also seeds demo data on every startup, coupling data generation to the application binary.

This design addresses three problems. First, developers lack an easy way to switch between storage and cache backends during development. Second, container management requires external tooling like `docker-compose.yml`, which fragments the development experience. Third, demo data generation pollutes the server codebase and runs unconditionally.

## Design Principles

This implementation follows two core patterns from the project's architecture.

**Functional Core - Imperative Shell** separates pure logic from side effects. Seed data generation, container command construction, and feature flag resolution are pure functions that receive inputs and return outputs. Container orchestration, HTTP requests, and process management form the imperative shell that performs actual I/O.

**Progressive Disclosure** keeps high-level documentation concise while detailed context lives in dedicated files. This design document provides implementation phases and task breakdowns. The resulting `.claude/context/dev-server.md` file will document usage patterns for future reference.

## Command Interface

The renamed command `cargo xtask dev server` accepts these flags:

```
Storage/Cache Options:
  --storage <TYPE>     Storage backend: inmemory (default), sqlite, dynamodb
                       [env: CALENDSYNC_STORAGE]
  --cache <TYPE>       Cache backend: memory (default), redis
                       [env: CALENDSYNC_CACHE]

Container Options:
  --podman             Use podman instead of docker [env: CALENDSYNC_PODMAN]
  --flush              Remove existing volumes before starting containers

Data Options:
  --seed               Seed the database with demo data via HTTP after startup

Existing Options:
  -p, --port <PORT>    Port to run the server on [default: 3000] [env: PORT]
  --release            Build in release mode
  --no-hot-reload      Disable TypeScript hot-reload
  --no-auto-refresh    Disable browser auto-refresh on hot-reload
```

When invoked without storage or cache flags, the command runs with `inmemory` storage and `memory` cache. This configuration requires no external dependencies and provides a zero-friction development experience.

## Execution Flow

The command executes in five stages.

**Stage 1: Option Resolution.** Parse command-line arguments and environment variables. Determine which containers are required based on storage and cache selections.

**Stage 2: Container Management.** If DynamoDB or Redis is required, start the appropriate containers. With `--flush`, remove existing volumes first. Always stop and remove stale containers from previous sessions before starting fresh ones.

**Stage 3: Infrastructure Setup.** For DynamoDB, deploy the table schema after the container becomes healthy. For SQLite, ensure the data directory exists.

**Stage 4: Server Execution.** Build and run the server with appropriate Cargo features. Pass environment variables for backend configuration.

**Stage 5: Seeding and Cleanup.** If `--seed` is specified, wait for the server's health endpoint and then create demo data via HTTP. On shutdown, stop and remove containers while preserving volumes.

## Container Specifications

The xtask command manages containers directly, eliminating the need for `docker-compose.yml`.

**DynamoDB Local** uses the `amazon/dynamodb-local:latest` image with container name `calendsync-dynamodb`. It maps port 8000 and mounts a named volume `calendsync-dynamodb-data` at `/data`. The container runs with `-jar DynamoDBLocal.jar -sharedDb -dbPath /data`.

**Redis** uses the `redis:7-alpine` image with container name `calendsync-redis`. It maps port 6379 and mounts a named volume `calendsync-redis-data` at `/data`. The container runs with default Redis configuration and the `--appendonly yes` flag for persistence.

## Feature Flag Architecture

The server uses Cargo features to select backends at compile time. This approach enables dead code elimination and avoids runtime dispatch overhead.

Valid feature combinations form a matrix of storage and cache options:

| Storage | Cache | Features |
|---------|-------|----------|
| inmemory | memory | `inmemory,memory` |
| sqlite | memory | `sqlite,memory` |
| sqlite | redis | `sqlite,redis` |
| dynamodb | memory | `dynamodb,memory` |
| dynamodb | redis | `dynamodb,redis` |

The server exposes a single `AppState::new(&config)` entry point. Compile-time `#[cfg]` attributes select the appropriate implementation based on enabled features. Invalid combinations, such as enabling both `sqlite` and `dynamodb`, produce compile-time errors via `compile_error!` macros.

---

## Phase 1: Server Refactoring

This phase prepares the server for the new development workflow by removing demo data coupling and adding calendar endpoints.

### Context

The server currently seeds demo data in `AppState::with_demo_data()`, which runs on every startup. Five implementations of this method exist, one for each feature combination. The `mock_data.rs` file in the calendsync crate generates entries that should instead live in xtask.

The server lacks calendar CRUD endpoints. Only entry endpoints exist at `/api/entries`. The seeding workflow requires `POST /api/calendars` to create a calendar before populating it with entries.

### Tasks

These tasks can run in parallel since they modify independent parts of the codebase.

**Task 1.1: Add Calendar CRUD Endpoints**

Create `crates/calendsync/src/handlers/calendars.rs` with four handlers:

- `create_calendar` handles `POST /api/calendars` and accepts form data with `name`, `color`, and optional `description`. It returns the created calendar as JSON.
- `get_calendar` handles `GET /api/calendars/{id}` and returns a calendar or 404.
- `update_calendar` handles `PUT /api/calendars/{id}` and accepts the same form data as create.
- `delete_calendar` handles `DELETE /api/calendars/{id}` and returns 200 on success.

Create `crates/calendsync/src/models/calendar.rs` with `CreateCalendar` and `UpdateCalendar` request types following the pattern in `models/entry.rs`.

Update `crates/calendsync/src/handlers/mod.rs` to export the new module. Update `crates/calendsync/src/app.rs` to register routes under `/api/calendars`.

**Task 1.2: Add Compile-Time Feature Guards**

Update `crates/calendsync/src/state.rs` to add compile-time validation for feature combinations. Add these guards at the module level:

```rust
#[cfg(all(feature = "sqlite", feature = "dynamodb"))]
compile_error!("Cannot enable both 'sqlite' and 'dynamodb' storage features");

#[cfg(all(feature = "memory", feature = "redis"))]
compile_error!("Cannot enable both 'memory' and 'redis' cache features");

#[cfg(not(any(feature = "inmemory", feature = "sqlite", feature = "dynamodb")))]
compile_error!("Must enable exactly one storage feature: 'inmemory', 'sqlite', or 'dynamodb'");

#[cfg(not(any(feature = "memory", feature = "redis")))]
compile_error!("Must enable exactly one cache feature: 'memory' or 'redis'");
```

**Task 1.3: Unify AppState Initialization**

Refactor `crates/calendsync/src/state.rs` to provide a single `AppState::new(&config)` entry point. Each feature-gated module (`sqlite_memory`, `sqlite_redis`, `dynamodb_memory`, `dynamodb_redis`, `inmemory_memory`) should implement `AppState::new()` instead of `AppState::with_demo_data()`.

Remove the `with_demo_data()` methods entirely. Remove the `DEMO_CALENDAR_ID` constant since seeding will generate fresh UUIDs.

Update `crates/calendsync/src/main.rs` to call `AppState::new(&config).await?` instead of `AppState::with_demo_data(&config).await?`.

---

The following task depends on Task 1.3 completing first.

**Task 1.4: Remove Demo Data from Server**

Delete `crates/calendsync/src/mock_data.rs`. Remove its `mod mock_data;` declaration from `main.rs`. Remove any imports of `generate_mock_entries` from the state module.

**Task 1.5: Update Default Features**

Update `crates/calendsync/Cargo.toml` to change default features:

```toml
[features]
default = ["inmemory", "memory"]
```

This ensures `cargo run -p calendsync` starts an ephemeral in-memory server.

---

## Phase 2: Xtask Refactoring

This phase renames the web command to server and adds the container management infrastructure.

### Context

The current `xtask/src/dev/web.rs` implements the development server with hot-reload support. The `xtask/src/integration/mod.rs` file contains container management patterns for starting DynamoDB and Redis via Docker Compose. The new implementation will manage containers directly without Docker Compose.

The `docker-compose.yml` file at the repository root defines container configurations that will move into Rust code.

### Tasks

These first three tasks can run in parallel.

**Task 2.1: Rename Web to Server**

Rename `xtask/src/dev/web.rs` to `xtask/src/dev/server.rs`. Update `xtask/src/dev/mod.rs` to change:

- `pub mod web;` to `pub mod server;`
- `DevTarget::Web(web::WebOptions)` to `DevTarget::Server(server::ServerOptions)`
- The match arm in `run()` from `DevTarget::Web` to `DevTarget::Server`
- The clap subcommand comment from "Run the Axum web server" to "Run the development server"

Rename `WebOptions` to `ServerOptions` in the server module.

**Task 2.2: Create Container Management Module**

Create `xtask/src/dev/containers.rs` with pure functions and I/O functions following the Functional Core - Imperative Shell pattern.

Pure functions (Functional Core):

- `container_run_args(spec: &ContainerSpec, runtime: ContainerRuntime) -> Vec<String>` builds the argument list for `docker run` or `podman run`
- `required_containers(storage: Storage, cache: Cache) -> Vec<ContainerSpec>` returns which containers are needed for a given configuration

I/O functions (Imperative Shell):

- `start_container(runtime, spec) -> Result<()>` executes the container start command
- `stop_container(runtime, name) -> Result<()>` stops and removes a container
- `remove_volume(runtime, name) -> Result<()>` removes a named volume
- `wait_for_health(runtime, name, timeout) -> Result<()>` polls until healthy
- `is_container_running(runtime, name) -> Result<bool>` checks container state

Define these types:

```rust
#[derive(Debug, Clone, Copy)]
pub enum ContainerRuntime {
    Docker,
    Podman,
}

#[derive(Debug, Clone)]
pub struct ContainerSpec {
    pub name: &'static str,
    pub image: &'static str,
    pub port: u16,
    pub volume_name: &'static str,
    pub volume_path: &'static str,
    pub command: Option<&'static str>,
    pub health_check: HealthCheck,
}

#[derive(Debug, Clone)]
pub enum HealthCheck {
    Http { port: u16 },
    Redis,
}
```

**Task 2.3: Create Seed Module**

Create `xtask/src/dev/seed.rs` with seed generation and HTTP seeding functions.

Pure functions (Functional Core):

- `generate_seed_entries(calendar_id: Uuid, center_date: NaiveDate) -> Vec<SeedEntry>` generates mock calendar entries. Consolidate logic from the existing `xtask/src/dynamodb/seed.rs` and the now-deleted `calendsync/src/mock_data.rs`.
- `generate_seed_calendar() -> SeedCalendar` returns a default calendar definition.

I/O functions (Imperative Shell):

- `seed_via_http(base_url: &str) -> Result<Uuid>` creates a calendar and entries via HTTP, returning the calendar ID.
- `wait_for_server(base_url: &str, timeout: Duration) -> Result<()>` polls the health endpoint.

Define request types that match the server's expected form data:

```rust
#[derive(Debug, Serialize)]
pub struct SeedCalendar {
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SeedEntry {
    pub calendar_id: Uuid,
    pub title: String,
    pub date: NaiveDate,
    pub kind: String,
    // ... other fields matching CreateEntry
}
```

---

The following task depends on Tasks 2.1, 2.2, and 2.3.

**Task 2.4: Enhance Server Command Options**

Update `xtask/src/dev/server.rs` to add new options to `ServerOptions`:

```rust
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum Storage {
    #[default]
    Inmemory,
    Sqlite,
    Dynamodb,
}

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum Cache {
    #[default]
    Memory,
    Redis,
}

#[derive(Debug, clap::Args)]
pub struct ServerOptions {
    // Existing options...

    #[arg(long, default_value = "inmemory", env = "CALENDSYNC_STORAGE")]
    pub storage: Storage,

    #[arg(long, default_value = "memory", env = "CALENDSYNC_CACHE")]
    pub cache: Cache,

    #[arg(long, env = "CALENDSYNC_PODMAN")]
    pub podman: bool,

    #[arg(long)]
    pub flush: bool,

    #[arg(long)]
    pub seed: bool,
}
```

**Task 2.5: Implement Enhanced Run Function**

Rewrite the `run()` function in `xtask/src/dev/server.rs` to implement the five-stage execution flow:

1. Determine required containers from storage and cache options
2. Handle `--flush` by removing volumes if requested
3. Start containers and wait for health
4. For DynamoDB, run table deployment
5. Build the server with appropriate features via `cargo run -p calendsync --no-default-features --features {storage},{cache}`
6. Set environment variables: `PORT`, `DEV_MODE`, `AWS_ENDPOINT_URL`, `REDIS_URL`, `SQLITE_PATH`
7. If `--seed`, wait for server health and call `seed_via_http()`
8. On shutdown, stop containers (preserve volumes)

**Task 2.6: Update Dev Error Types**

Update `xtask/src/dev/error.rs` to add error variants:

```rust
#[derive(Debug, Error)]
pub enum DevError {
    // Existing variants...

    #[error("Container runtime not found: {0}")]
    ContainerRuntimeNotFound(String),

    #[error("Container '{name}' failed to become healthy within {timeout_secs}s")]
    ContainerNotHealthy { name: String, timeout_secs: u64 },

    #[error("Failed to start container: {0}")]
    ContainerStartFailed(String),

    #[error("Seeding failed: {0}")]
    SeedingFailed(String),

    #[error("Port {port} is already in use")]
    PortInUse { port: u16 },
}
```

---

## Phase 3: Cleanup and Documentation

This phase removes obsolete files and updates documentation.

### Context

The `docker-compose.yml` file becomes obsolete since xtask manages containers directly. The integration test module contains Docker Compose references that need updating. Documentation across multiple files references `cargo xtask dev web`.

### Tasks

These tasks can run in parallel.

**Task 3.1: Delete docker-compose.yml**

Remove `docker-compose.yml` from the repository root.

**Task 3.2: Update Integration Tests**

Update `xtask/src/integration/mod.rs` to use the new container management module instead of Docker Compose commands. Import and use functions from `xtask/src/dev/containers.rs`.

**Task 3.3: Update DynamoDB Seed Command**

Update `xtask/src/dynamodb/seed.rs` to use the shared `generate_seed_entries()` function from `xtask/src/dev/seed.rs`. Remove the duplicated seed generation logic.

**Task 3.4: Update Documentation**

Update these files to replace `web` with `server` and document new options:

- `CLAUDE.md`: Update command examples in Build Commands section
- `.claude/context/running-apps.md`: Update development workflow
- `.claude/context/xtask-dev.md`: Update command documentation

**Task 3.5: Create Dev Server Context File**

Create `.claude/context/dev-server.md` documenting:

- Command usage and all options
- Storage and cache backend combinations
- Container lifecycle (start on run, stop on shutdown, volumes persist)
- The `--flush` flag for clearing persisted data
- The `--seed` flag for populating demo data
- Environment variable equivalents for all options
- Feature flag architecture and compile-time validation

---

## Phase 4: Testing and Validation

This phase validates the implementation works correctly.

### Context

The enhanced command touches multiple subsystems: container orchestration, HTTP endpoints, feature flag compilation, and the existing hot-reload functionality. Each combination of storage and cache requires verification.

### Tasks

These tasks must run sequentially since they test progressively more complex configurations.

**Task 4.1: Verify Default Configuration**

Run `cargo xtask dev server` without flags. Verify:

- Server starts with inmemory storage and memory cache
- No containers are started
- Server responds to health checks
- Hot-reload functions correctly

**Task 4.2: Verify Seeding**

Run `cargo xtask dev server --seed`. Verify:

- Calendar is created via HTTP
- Entries are created via HTTP
- Calendar URL is printed
- Calendar page renders with entries

**Task 4.3: Verify SQLite Configuration**

Run `cargo xtask dev server --storage sqlite --seed`. Verify:

- No containers are started
- SQLite database file is created in `.local/data/`
- Data persists across restarts (without `--flush`)

**Task 4.4: Verify DynamoDB Configuration**

Run `cargo xtask dev server --storage dynamodb --seed`. Verify:

- DynamoDB container starts automatically
- Table is created
- Data persists across restarts
- Container stops on Ctrl+C
- `--flush` removes volume and starts fresh

**Task 4.5: Verify Redis Configuration**

Run `cargo xtask dev server --storage sqlite --cache redis --seed`. Verify:

- Redis container starts automatically
- Cache operations work
- Container stops on Ctrl+C

**Task 4.6: Verify Full Stack**

Run `cargo xtask dev server --storage dynamodb --cache redis --seed`. Verify:

- Both containers start
- All operations work correctly
- Clean shutdown stops both containers

---

## Summary

This design transforms `cargo xtask dev server` into a comprehensive development command that manages the full stack. Developers can switch between backend configurations with simple flags while xtask handles container orchestration automatically. The HTTP-based seeding approach validates API endpoints as a side effect, improving confidence in the implementation.

The phased approach allows incremental progress. Phase 1 can begin immediately since it modifies only the server crate. Phase 2 depends on Phase 1's calendar endpoints for seeding. Phase 3 performs cleanup after the core functionality works. Phase 4 validates everything functions correctly.
