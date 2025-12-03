# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

calendsync is a Rust web application for creating calendars to sync with friends. It uses axum as the web framework with htmx for frontend interactivity.

This is a technical exploration of the **"Rust full-stack" pattern**: server-side rendering React with `deno_core`, real-time updates via SSE, and a pure Rust backend - demonstrating that modern web apps can be built entirely in Rust without Node.js.

**Current State**: Working web application with:
- REST API for users, calendars, and calendar entries
- htmx-powered HTML pages
- React SSR calendar with real-time SSE updates
- In-memory state with demo data for development
- Graceful shutdown support

## Build Commands

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Run the server (default port 3000)
cargo run -p calendsync

# Run with auto-reload (requires systemfd and cargo-watch)
systemfd --no-pid -s http::3000 -- cargo watch -x 'run -p calendsync'

# Run code quality checks (fmt, check, clippy, test, machete)
cargo xtask lint

# Auto-fix formatting issues
cargo xtask lint --fix

# Install git pre-commit hooks
cargo xtask lint --install-hooks

# Run a single test
cargo test <test_name>

# Run tests for a specific package
cargo test -p calendsync_core

# Run the React SSR example
cargo run --example react-ssr -p calendsync
```

## Architecture

### Workspace Structure

- **crates/calendsync** - Main binary application (web server)
- **crates/core** - Pure business logic library (Functional Core)
- **crates/client** - CLI client for calendsync API
- **crates/frontend** - TypeScript build crate (bun bundler)
- **crates/ssr_core** - Pure SSR functions (config, validation, polyfills)
- **crates/ssr** - SSR worker pool (threading, JsRuntime execution)
- **xtask/** - Development automation tasks (cargo-xtask pattern)

### Web Application Structure (crates/calendsync)

```
src/
├── main.rs          # Entry point, server setup, graceful shutdown
├── app.rs           # Router configuration, middleware (CORS, tracing, timeout)
├── state.rs         # AppState with in-memory data stores, SSE support
├── error.rs         # Error types and conversions
├── mock_data.rs     # Demo data generation
├── handlers/        # HTTP request handlers
│   ├── api.rs       # User CRUD endpoints
│   ├── calendars.rs # Calendar CRUD endpoints
│   ├── entries.rs   # Entry CRUD endpoints
│   ├── pages.rs     # HTML page handlers
│   ├── calendar.rs  # Calendar demo page (htmx)
│   ├── calendar_react.rs # React SSR calendar handler (uses SsrPool)
│   ├── events.rs    # SSE events handler for real-time updates
│   └── health.rs    # SSR pool health check endpoints
└── models/          # Data models
    ├── user.rs
    ├── calendar.rs
    └── entry.rs
templates/           # Askama HTML templates
```

### xtask Commands

The project uses the [cargo-xtask](https://github.com/matklad/cargo-xtask/) pattern for development tasks:

- `cargo xtask lint` - Run all code quality checks (fmt, check, clippy, test, cargo-machete)
- `cargo xtask install` - Build and install the binary locally
- `cargo xtask release create <version>` - Create a new release
- `cargo xtask dynamodb deploy` - Deploy DynamoDB table infrastructure
- `cargo xtask dynamodb seed` - Seed calendar with mock entries

### Tech Stack

- **axum** - Web framework
- **htmx** - Frontend interactivity
- **askama** - HTML templating
- **tokio** - Async runtime
- **bun** - TypeScript bundler

Reference documentation available in `.claude/context/AXUM.md` and `.claude/context/HTMX.md`.

## TypeScript Development (crates/frontend)

### Build Commands

```bash
cd crates/frontend

# Install dependencies
bun install

# Production build (minified, no sourcemaps)
bun run build

# Development build (with sourcemaps)
bun run build:dev

# TypeScript type checking
bun run typecheck

# Run tests
bun test

# Watch mode for development
bun run watch
```

### Biome (Format + Lint)

The project uses [Biome](https://biomejs.dev/) for TypeScript formatting and linting:

```bash
# Format and lint with auto-fix
bunx biome check --write --unsafe
```

Configuration is in `crates/frontend/biome.json`:
- **Indent**: 4 spaces (matches Rust)
- **Line width**: 100 (matches Rust)
- **Quotes**: double
- **Semicolons**: as-needed
- **Linting**: recommended rules enabled

### Architecture

Follows Functional Core - Imperative Shell pattern:

```
crates/frontend/
├── lib.rs                  # Rust placeholder (triggers build.rs)
└── src/                    # Pure TypeScript
    ├── calendar/           # htmx calendar (vanilla JS)
    │   ├── api.ts          # Fetch calls to /api/entries
    │   ├── render.ts       # DOM manipulation
    │   ├── events.ts       # Event handlers (touch, scroll)
    │   ├── entryModal.ts   # Modal UI controller
    │   └── index.ts        # Entry point (exports window.initCalendar)
    ├── calendar-react/     # React SSR calendar
    │   ├── server.tsx      # SSR entry point (prerender)
    │   ├── client.tsx      # Client hydration
    │   ├── styles.css      # Component styles
    │   ├── hooks/          # React hooks (useCalendar, useSSE, useNotifications)
    │   └── components/     # React components
    └── core/calendar/      # Functional Core (pure, testable)
        ├── __tests__/      # Unit tests (89 tests)
        ├── types.ts        # Data types (no DOM types)
        ├── dates.ts        # Pure date calculations
        ├── entries.ts      # Pure entry filtering/sorting
        ├── layout.ts       # Pure layout calculations
        └── index.ts        # Re-exports
```

### Adding New Pages

1. Create entry point: `src/[page]/index.ts`
2. Update build scripts in `package.json` to include the new entry
3. Build generates `[page]-[hash].js` with content-hashed filename
4. Manifest auto-updates with filename mappings

### Build Integration

The TypeScript build is triggered by Cargo:
1. `cargo build -p calendsync` triggers `calendsync/build.rs`
2. `calendsync/build.rs` depends on `calendsync_frontend` crate
3. `frontend/build.rs` runs `bun build`, outputs to `frontend/dist/`
4. `frontend/build.rs` generates `frontend/manifest.json` with hashed filenames
5. `calendsync/build.rs` creates symlink and generates `assets.rs`

### Dependencies (from workspace Cargo.toml)

- `clap` with derive feature for CLI parsing
- `serde`/`serde_json` for serialization
- `chrono` for date/time handling
- `tracing`/`tracing-subscriber` for logging
- `thiserror` for error types

## Lint Checks

The `cargo xtask lint` command runs these checks in order:

**Rust checks:**
1. `cargo fmt --check` - Code formatting
2. `cargo check --all-targets` - Compilation
3. `cargo clippy --all-targets -- -D warnings` - Linting (warnings are errors)
4. `cargo test --all-targets` - Tests
5. `cargo machete` - Unused dependencies detection

**TypeScript checks (crates/frontend):**
6. `biome check --write --unsafe` - Format and lint with auto-fix
7. `bun run typecheck` - TypeScript type checking
8. `bun test` - Run TypeScript tests

**TypeScript checks (examples/hello-world):**
9. `biome check --write --unsafe` - Format and lint example TypeScript
10. `bun run typecheck` - Example TypeScript type checking

Pre-commit hooks can be installed with `cargo xtask lint --install-hooks`.

### calendsync_core Crate Requirements

The `calendsync_core` crate contains pure business logic following the Functional Core pattern. When working with this crate:

**STRICT RULES:**

1. **No Async Functions**: All functions MUST be synchronous. No `async fn` allowed.
   - Core logic should not perform I/O operations
   - Use regular functions that can be called from sync or async contexts

2. **No Side Effects**: Functions must be pure:
   - No file system operations (no reading/writing files)
   - No network requests (no API calls)
   - No external command execution
   - No printing to stdout/stderr
   - No accessing environment variables
   - No global state mutations

3. **Configuration via Arguments**:
   - All configuration must be passed as function parameters
   - No reading from config files inside core functions
   - If a function requires more than 5 arguments, create a config struct

4. **Workspace Dependencies**:
   - Use `{ workspace = true }` for all dependencies in `calendsync_core/Cargo.toml`

5. **Error Handling**:
   - Use domain-specific error types per module (e.g., `RegistryError`, `IamError`)
   - Each module should define its own `Result<T>` type alias
   - Only include error variants for parsing, validation, and transformation failures
   - I/O-related errors belong in the `calendsync` crate
   - The shell should implement `From<DomainError>` to convert core errors

6. **Naming Conventions**:
   - **Module files**: Use operation-based names that describe what the code does
     - ✅ `parsing.rs`, `validation.rs`, `formatting.rs`, `comparison.rs`
     - ❌ `core.rs`, `utils.rs`, `helpers.rs` (too generic)
   - **Error types**: Use domain-specific error names
     - ✅ `RegistryError`, `IamError`, `ClusterValidationError`
     - ❌ `CoreError`, `Error` (too generic inside calendsync_core)
   - **Types**: Use nouns that describe the data they hold
     - ✅ `ImageComponents`, `ClusterInfo`, `RoleComparison`
     - ❌ `Result`, `Data`, `Info` (too vague)

7. **API Design Best Practices**:
   - Prefer named structs over tuples for return types with multiple values
     - Use `ImageComponents { name, tag }` instead of `(String, String)`
     - Named fields are self-documenting and easier to extend
   - Use builder pattern for structs with many optional fields
   - Provide utility methods on types (e.g., `is_empty()`, `is_valid()`)
   - Include `Default` derive when sensible defaults exist

8. **Testability First**:
   - Every public function should have unit tests
   - Tests should not require mocks, external services, or credentials
   - Tests should use simple, in-memory data structures

**Example Pattern:**

```rust
// GOOD - Pure function in calendsync_core
pub fn filter_metrics_by_threshold(
    metrics: &[Metric],
    threshold: f64
) -> Vec<&Metric> {
    metrics.iter()
        .filter(|m| m.value > threshold)
        .collect()
}

// BAD - Has side effects, belongs in calendsync
pub async fn fetch_and_filter_metrics(
    client: &ApiClient,
    threshold: f64
) -> Result<Vec<Metric>> {
    let metrics = client.get_metrics().await?; // I/O operation
    Ok(filter_metrics_by_threshold(&metrics, threshold))
}
```

**Module Organization:**

```
crates/core/src/
├── lib.rs           # Public API exports
└── calendar/
    ├── mod.rs       # Module exports and re-exports
    ├── types.rs     # CalendarEntry, EntryStatus, etc.
    ├── sorting.rs   # Pure sorting functions
    ├── operations.rs # Pure calendar operations
    └── error.rs     # CalendarError enum
```

## Release Process

Releases are managed via `cargo xtask release`:

1. **Create Release**:
   ```bash
   cargo xtask release create 1.2.3
   ```
   This will:
   - Validate you're on main branch with clean working directory
   - Check CI status
   - Update version in all Cargo.toml files
   - Create version bump commit
   - Create git tag `v1.2.3`
   - Push to GitHub
   - Monitor GitHub Actions release workflow
   - Optionally upgrade local binary with `--auto-upgrade`

2. **GitHub Actions** (`.github/workflows/release.yml`):
   - Triggered on tag push (`v*`)
   - Builds for multiple platforms: Linux x86_64, macOS Intel, macOS ARM64
   - Strips binaries
   - Creates GitHub release with assets

3. **Cleanup Failed Release**:
   ```bash
   cargo xtask release cleanup v1.2.3
   ```

## CI/CD

GitHub Actions workflows in `.github/workflows/`:

- **ci.yml** - Runs on push/PR to main/develop:
  - Tests with `cargo test`
  - Clippy with `-D warnings`
  - Format check with `cargo fmt`
  - Unused dependencies check with `cargo-machete`
  - Typo check with `typos`
  - Build check on Ubuntu and macOS

- **release.yml** - Triggered on version tags:
  - Multi-platform builds (Linux x86_64, macOS x86_64, macOS ARM64)
  - Creates GitHub release with binaries

- **dependabot.yml** - Automated dependency updates

## Web Application Patterns

When working with this codebase:

1. **Adding New Handlers**:
   - Add handler functions in `handlers/` module
   - Register routes in `app.rs` using axum's routing macros
   - Use `State<AppState>` extractor for shared state access
   - Return appropriate response types (`Json`, `Html`, `StatusCode`)

2. **Error Handling**:
   - Return `Result<T, AppError>` from handlers
   - Use `?` operator for propagation
   - Errors are converted to appropriate HTTP responses

3. **Adding New Models**:
   - Define model structs in `models/` module
   - Derive `Serialize`, `Deserialize`, `Clone` as needed
   - Add corresponding storage in `AppState`

## Calendar UI

The main calendar view is implemented in `templates/calendar.html` as a single-page application using vanilla JavaScript modules.

### Architecture

The UI consists of two JavaScript modules:
- **`calendar`** - Manages the calendar grid, navigation, and day rendering
- **`entryModal`** - Handles the modal for creating/editing calendar entries

### Highlighted Day

The **highlighted day** (`centerDate`) is the currently focused date in the calendar view:
- On **mobile**: The single day displayed in the center of the screen
- On **desktop**: The center day of the visible week (3, 5, or 7-day view)

The highlighted day is tracked client-side in JavaScript and exposed via `calendar.getCenterDate()`. It updates when:
- User navigates with prev/next buttons
- User taps "Today" to jump to current date
- User scrolls/swipes through the calendar

When opening the "New Entry" modal (FAB button), the date field is pre-populated with the highlighted day.

### Key Functions

- `calendar.getCenterDate()` - Returns the current highlighted date as a JavaScript Date object
- `calendar.navigateDays(offset)` - Move the highlighted day by offset days
- `calendar.goToToday()` - Reset highlighted day to today
- `entryModal.openCreate(dateStr?)` - Open modal for new entry (uses highlighted day if no date provided)
- `entryModal.openEdit(tileElement)` - Open modal to edit existing entry

## Testing Strategy

Integration tests using `tower::ServiceExt`:

```rust
use axum::{body::Body, http::{Request, StatusCode}};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn test_endpoint() {
    let state = AppState::default();
    let app = create_app(state);

    let response = app
        .oneshot(Request::builder().uri("/api/endpoint").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    // Assert on body content
}
```

## Code Style

- Follow Rust standard style (`rustfmt`)
- Use `clippy` recommendations (CI runs with `-D warnings`)
- Prefer descriptive variable names
- Add rustdoc comments for public APIs
- Use `#[allow(clippy::large_enum_variant)]` sparingly and with justification

## Issue Tracking with Beads

This project uses [beads](https://github.com/cloudbridgeuy/beads) for issue tracking. Beads stores issues in `.beads/issues.jsonl` and syncs via git.

### Common Commands

```bash
# List ready issues (no blockers)
bd ready

# List all open issues
bd list --status=open

# Show issue details
bd show <id>

# Create an issue
bd create --title="Fix bug" --type=bug
bd create --title="Add feature" --type=feature
bd create --title="Refactor code" --type=task

# Update issue status
bd update <id> --status=in_progress
bd update <id> --status=closed

# Close an issue
bd close <id>
bd close <id> --reason="Completed"

# Add dependencies
bd dep add <issue> <depends-on>

# Sync with remote
bd sync
```

### Issue Types

- `bug` - Something broken that needs fixing
- `feature` - New functionality
- `task` - Refactoring, documentation, infrastructure

### Workflow

1. Check available work: `bd ready`
2. Claim an issue: `bd update <id> --status=in_progress`
3. Work on the issue
4. Close when done: `bd close <id>`
5. Sync changes: `bd sync`

**Note**: Beads is separate from git commits. Closing an issue with `bd close` doesn't require a commit message reference. The `Closes: <id>` convention in commit messages is optional documentation.

## Functional Core - Imperative Shell

We advocate the use of this pattern when writing code for this repo.

The pattern is based on separating code into two distinct layers:

**Functional Core**: Pure, testable business logic free of side effects (no I/O, no external state mutations). It operates only on the data it's given.

**Imperative Shell**: Responsible for side effects like database calls, network requests, and sending emails. It uses the functional core to perform business logic.

### Example Transformation

**Before (mixed logic and side effects):**

```rust
async fn send_user_expiry_emails(db: &Database, email_service: &EmailService) -> Result<()> {
    let users = db.get_users().await?;

    for user in users {
        if user.subscription_end_date > Utc::now() {
            continue;
        }
        if user.is_free_trial {
            continue;
        }

        email_service
            .send(
                &user.email,
                &format!("Your account has expired {}.", user.name),
            )
            .await?;
    }

    Ok(())
}
```

**After (separated):**

**Functional Core:**
```rust
// Pure filtering logic - no side effects
fn get_expired_users(users: &[User], cutoff: DateTime<Utc>) -> Vec<&User> {
    users
        .iter()
        .filter(|user| user.subscription_end_date <= cutoff)
        .filter(|user| !user.is_free_trial)
        .collect()
}

// Pure email generation - no side effects
fn generate_expiry_emails(users: &[&User]) -> Vec<Email> {
    users
        .iter()
        .map(|user| Email {
            to: user.email.clone(),
            subject: "Account Expired".to_string(),
            body: format!("Your account has expired {}.", user.name),
        })
        .collect()
}
```

**Imperative Shell:**
```rust
// Orchestrates I/O operations using pure functions
async fn send_user_expiry_emails(db: &Database, email_service: &EmailService) -> Result<()> {
    let users = db.get_users().await?;
    let expired = get_expired_users(&users, Utc::now());
    let emails = generate_expiry_emails(&expired);
    email_service.bulk_send(&emails).await?;
    Ok(())
}
```

### Benefits

- **More testable**: Core logic can be tested in isolation without mocking I/O
- **More maintainable**: Pure functions are easier to reason about and modify
- **More reusable**: Business logic (e.g., `getExpiredUsers`) can be reused for other features like reminder emails
- **More adaptable**: Imperative shell can be swapped out (e.g., change from email to SMS) without touching core logic

### Applying to calendsync

When adding new features to `calendsync`:

1. **Separate concerns**: Extract pure logic (filtering, sorting, validation) from I/O operations (HTTP handlers, state access)

2. **Example - Calendar entry filtering:**
   ```rust
   // Functional Core (in calendsync_core) - pure filtering logic
   fn filter_entries_by_date_range(
       entries: &[CalendarEntry],
       start: NaiveDate,
       end: NaiveDate
   ) -> Vec<&CalendarEntry> {
       entries.iter()
           .filter(|e| e.date >= start && e.date <= end)
           .collect()
   }

   // Imperative Shell (in calendsync) - I/O and coordination
   pub async fn list_filtered_entries(
       State(state): State<AppState>,
       Query(params): Query<DateRangeParams>
   ) -> Result<Json<Vec<CalendarEntry>>, AppError> {
       let entries = state.entries.read().await;
       let filtered = filter_entries_by_date_range(&entries, params.start, params.end);
       Ok(Json(filtered.into_iter().cloned().collect()))
   }
   ```

3. **Test the core**: Write unit tests for pure functions without needing HTTP context or mocked state

4. **Keep shells thin**: Imperative shell should be primarily about HTTP handling and state access, delegating logic to the core

The pattern is based on [Gary Bernhardt's original talk](https://www.destroyallsoftware.com/screencasts/catalog/functional-core-imperative-shell) on the concept.

## Progressive Disclosure

Detailed documentation is kept in dedicated files. Consult these when working on related features:

| Topic | Location |
|-------|----------|
| Web Application | `crates/calendsync/README.md` |
| CLI Client | `crates/client/README.md` |
| DynamoDB Schema | `docs/dynamodb.md` |
| DynamoDB xtask | `.claude/context/dynamodb.md` |
| React SSR Example | `crates/calendsync/examples/README.md` |
| React Calendar | `.claude/context/react-calendar.md` |
| SSR Worker Pool | `.claude/context/ssr-worker-pool.md` |
| Axum Reference | `.claude/context/AXUM.md` |
| HTMX Reference | `.claude/context/HTMX.md` |
| React SSR Context | `.claude/context/react-ssr-example.md` |

### Examples

- **React SSR Example**: Minimal SSR with `deno_core`. Run: `cargo run --example react-ssr -p calendsync`
- **React Calendar**: Full SSR calendar with SSE. Run: `cargo run -p calendsync` then visit `/calendar/{calendar_id}`

## Glossary

| Term | Definition |
|------|------------|
| **calendsync** | Main binary crate - the web server application |
| **calendsync_core** | Pure business logic library following Functional Core pattern |
| **calendsync_client** | CLI client crate for interacting with calendsync API |
| **calendsync_frontend** | TypeScript build crate using bun bundler |
| **Functional Core** | Pure, testable functions with no side effects (no I/O, no state mutations) |
| **Imperative Shell** | Thin layer handling I/O (HTTP, database) that calls into the Functional Core |
| **SSR** | Server-Side Rendering - generating HTML on the server (e.g., React with deno_core) |
| **Hydration** | Client-side process of attaching event handlers to server-rendered HTML |
| **bun** | Fast JavaScript/TypeScript bundler and runtime used for frontend builds |
| **deno_core** | Minimal JavaScript runtime from Deno, used for SSR in Rust |
| **ops** | Custom Rust functions callable from JavaScript in deno_core |
| **htmx** | Library for frontend interactivity via HTML attributes (no JavaScript) |
| **askama** | Type-safe HTML templating engine for Rust |
| **xtask** | Cargo pattern for project-specific dev automation (`cargo xtask lint`) |
| **SSE** | Server-Sent Events - one-way real-time updates from server to client |
| **Notification Center** | UI component showing real-time SSE events (added/updated/deleted entries) |
| **DynamoDB** | AWS NoSQL database used for persistence (single-table design) |
| **GSI** | Global Secondary Index - alternate query pattern in DynamoDB |
| **CalendarMembership** | Entity linking users to calendars with roles (owner/writer/reader) |
