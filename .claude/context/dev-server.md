# Development Server Command

The `cargo xtask dev server` command manages the full development stack: storage backends, cache backends, container orchestration, TypeScript hot-reload, and HTTP-based data seeding.

## Command Interface

```bash
cargo xtask dev server [OPTIONS]

Storage/Cache Options:
  --storage <TYPE>     Storage backend: inmemory (default), sqlite, dynamodb
  --cache <TYPE>       Cache backend: memory (default), redis

Container Options:
  --podman             Use podman instead of docker
  --flush              Remove existing volumes before starting containers

Data Options:
  --seed               Seed the database with demo data via HTTP after startup
  --open               Open browser to calendar URL after seeding (macOS only)

Debugging Options:
  --keep-containers    Keep containers running on error (default: stop containers)
  --verbose            Print container command output with color coding

Existing Options:
  -p, --port <PORT>    Port to run the server on [default: 3000]
  --release            Build in release mode
  --no-hot-reload      Disable TypeScript hot-reload
  --no-auto-refresh    Disable browser auto-refresh on hot-reload
```

## Environment Variables

All options support environment variable equivalents:

| Option      | Environment Variable    | Default    |
|-------------|------------------------|------------|
| `--port`    | `PORT`                 | `3000`     |
| `--storage` | `CALENDSYNC_STORAGE`   | `inmemory` |
| `--cache`   | `CALENDSYNC_CACHE`     | `memory`   |
| `--podman`  | `CALENDSYNC_PODMAN`    | `false`    |

## Execution Flow

The command executes in five stages.

**Stage 1: Option Resolution.** Parse CLI arguments and environment variables. Detect container runtime (Docker or Podman) if containers are required.

**Stage 2: Container Management.** Start DynamoDB and/or Redis containers based on storage and cache selection. With `--flush`, remove existing volumes first. Always stop stale containers before starting fresh ones.

**Stage 3: Infrastructure Setup.** For DynamoDB, deploy the table schema via `cargo xtask dynamodb deploy`. For SQLite, ensure the `.local/data/` directory exists.

**Stage 4: Server Execution.** Build and run the server with `cargo run -p calendsync --no-default-features --features {storage},{cache}`. Set environment variables for backend configuration.

**Stage 5: Seeding and Cleanup.** If `--seed` is specified, wait for the server's health endpoint, create a demo calendar via `POST /api/calendars`, and populate it with entries via `POST /api/entries`. On shutdown, stop containers while preserving volumes.

## Storage and Cache Combinations

| Storage   | Cache  | Features           | Containers Required |
|-----------|--------|--------------------|--------------------|
| inmemory  | memory | `inmemory,memory`  | None               |
| sqlite    | memory | `sqlite,memory`    | None               |
| sqlite    | redis  | `sqlite,redis`     | Redis              |
| dynamodb  | memory | `dynamodb,memory`  | DynamoDB           |
| dynamodb  | redis  | `dynamodb,redis`   | DynamoDB, Redis    |

## Container Lifecycle

Containers follow a predictable lifecycle:

1. **Start on run** - Required containers start automatically when the command runs
2. **Health checks** - Command waits for containers to become healthy before proceeding
3. **Stop on exit** - Containers stop when the server exits (Ctrl+C or process termination)
4. **Volumes persist** - Data volumes persist across runs for faster iteration
5. **Flush on demand** - `--flush` removes volumes before starting for a clean state

Container specifications:

| Service  | Image                        | Container Port | Volume                     |
|----------|------------------------------|----------------|----------------------------|
| DynamoDB | `amazon/dynamodb-local:latest` | 8000         | `calendsync-dynamodb-data` |
| Redis    | `redis:7-alpine`             | 6379           | `calendsync-redis-data`    |

**Dynamic Port Allocation**: Containers start with dynamic host port allocation (`-p 0:{port}`). After startup, the command queries the actual assigned port using `docker port` and configures environment variables accordingly. This prevents port conflicts when running multiple instances or when default ports are in use.

## Examples

```bash
# Default: inmemory storage + memory cache (no containers)
cargo xtask dev server

# With seeding - creates demo calendar and entries
cargo xtask dev server --seed

# With seeding and auto-open browser (macOS)
cargo xtask dev server --seed --open

# SQLite storage (creates .local/data/calendsync.db)
cargo xtask dev server --storage sqlite --seed

# DynamoDB (auto-starts container, deploys table)
cargo xtask dev server --storage dynamodb --seed

# Redis cache (auto-starts container)
cargo xtask dev server --cache redis --seed

# Full stack: DynamoDB + Redis (both containers)
cargo xtask dev server --storage dynamodb --cache redis --seed

# Fresh start: remove volumes before starting
cargo xtask dev server --storage dynamodb --flush --seed

# Use Podman instead of Docker
cargo xtask dev server --storage dynamodb --podman --seed

# Keep containers running for debugging
cargo xtask dev server --storage dynamodb --keep-containers

# Verbose mode: see all container commands and output
cargo xtask dev server --storage dynamodb --verbose
```

## Architecture

The implementation follows Functional Core - Imperative Shell:

```
xtask/src/dev/
├── mod.rs          # DevCommand, DevTarget enum
├── error.rs        # DevError types (container, seeding errors)
├── server.rs       # ServerOptions, five-stage run() function
├── containers.rs   # Pure: container_run_args(), cargo_features()
│                   # I/O: start_container(), wait_for_health()
├── seed.rs         # Pure: generate_seed_calendar(), convert_entry_to_seed()
│                   # I/O: seed_via_http(), wait_for_server()
├── desktop.rs      # Desktop app logic
└── ios.rs          # iOS app logic
```

**Pure functions** (Functional Core):
- `container_run_args(spec)` - Build Docker/Podman run arguments
- `required_containers(storage, cache)` - Determine which containers are needed
- `cargo_features(storage, cache)` - Generate feature string for cargo
- `environment_variables(storage, cache, port)` - Generate env vars for server
- `generate_seed_calendar()` - Create demo calendar data
- `convert_entry_to_seed(entry)` - Convert CalendarEntry to HTTP form data

**I/O functions** (Imperative Shell):
- `detect_runtime(prefer_podman)` - Check for Docker/Podman availability
- `start_container(runtime, spec)` - Start a container
- `stop_container(runtime, name)` - Stop and remove a container
- `wait_for_health(runtime, spec, timeout)` - Poll until container is healthy
- `seed_via_http(base_url, silent)` - Create calendar and entries via HTTP

## Error Handling

The command cleans up on error by default:

- If container startup fails, all started containers are stopped
- If DynamoDB table deployment fails, containers are stopped
- If server fails to start, containers are stopped
- Use `--keep-containers` to preserve containers for debugging

Error types in `DevError`:
- `ContainerRuntimeNotFound` - Neither Docker nor Podman available
- `ContainerNotHealthy` - Container failed health check within timeout
- `ContainerStartFailed` - Container failed to start
- `SeedingFailed` - HTTP seeding request failed
- `ServerNotHealthy` - Server health check timed out

## Seeding Details

The `--seed` flag creates a demo calendar with 25 entries via HTTP:

1. Wait for server health endpoint (`/healthz`)
2. Create calendar via `POST /api/calendars` with form data
3. Generate 25 mock entries using `calendsync_core::calendar::generate_seed_entries`
4. Create each entry via `POST /api/entries` with form data
5. Print the calendar URL for browser access

Entry distribution:
- ~15% multi-day events (conferences, vacations)
- ~20% all-day events (birthdays, holidays)
- ~45% timed activities (meetings, appointments)
- ~20% tasks (todos)
