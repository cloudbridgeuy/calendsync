# CalendSync Web Application

A web application built with Axum, Askama templates, and HTMX for dynamic user interface interactions.

## Tech Stack

- **Axum** - Web framework
- **Askama** - Type-safe templating engine
- **HTMX** - Frontend interactivity without JavaScript
- **Tokio** - Async runtime
- **Tower-HTTP** - HTTP middleware (CORS, tracing, timeouts)

## Development Setup

### Prerequisites

- Rust toolchain (1.70+)
- `systemfd` and `cargo-watch` for auto-reload (optional but recommended)

### Installation

Install development tools for auto-reload:

```bash
cargo install systemfd

# macOS (recommended - avoids compilation issues)
brew install watchexec

# Other platforms
cargo install watchexec-cli
```

## Running the Server

### Standard Mode

```bash
cargo run -p calendsync
```

The server will start at `http://127.0.0.1:3000`.

### With Auto-Reload (Recommended for Development)

**Using watchexec (recommended):**

```bash
# Watches all files in crates/calendsync (including templates)
watchexec -w crates/calendsync -r -- cargo run -p calendsync
```

**With socket preservation (avoids connection drops during restart):**

```bash
systemfd --no-pid -s http::3000 -- watchexec -w crates/calendsync -r -- cargo run -p calendsync
```

The `-r` flag restarts the process on file changes. The `-w` flag specifies the directory to watch.

## Architecture Patterns

### Graceful Shutdown

The server handles shutdown signals gracefully:

- **Ctrl+C** - Interactive termination
- **SIGTERM** - Process termination (e.g., from container orchestrators)

In-flight requests are allowed to complete with a 10-second timeout before the server fully shuts down.

### Error Handling

- Uses `anyhow` for flexible error handling
- `AppError` wrapper converts any error into an HTTP 500 response
- Errors are logged via `tracing` for debugging

```rust
// Example: Any function returning Result<_, anyhow::Error> can use ?
async fn handler() -> Result<String, AppError> {
    let value = some_fallible_operation()?;
    Ok(value)
}
```

### Dependency Injection

Application state is managed via Axum's `State` extractor:

```rust
#[derive(Clone)]
pub struct AppState {
    pub users: Arc<RwLock<HashMap<Uuid, User>>>,
}

async fn handler(State(state): State<AppState>) -> impl IntoResponse {
    // Access shared state
}
```

### HTMX Integration

The server returns HTML fragments for HTMX requests, enabling dynamic UI updates without full page reloads:

- `HX-Request` header is used to detect HTMX calls
- Form submissions return new table rows directly
- Delete operations return empty responses (HTMX removes the element)

Example HTMX attributes used:

```html
<form hx-post="/api/users" hx-target="#user-table-body" hx-swap="beforeend">
  <!-- Form fields -->
</form>

<button hx-delete="/api/users/{id}" hx-target="closest tr" hx-swap="outerHTML">
  Delete
</button>
```

### CORS

API endpoints support cross-origin requests via `tower-http::cors::CorsLayer`:

- Allows any origin
- Supports GET, POST, DELETE methods
- Accepts Content-Type header

## API Endpoints

| Method | Path              | Description                        |
| ------ | ----------------- | ---------------------------------- |
| GET    | `/`               | Main page with user table and form |
| GET    | `/api/users`      | List all users (JSON)              |
| POST   | `/api/users`      | Create a new user                  |
| GET    | `/api/users/{id}` | Get a single user by ID            |
| DELETE | `/api/users/{id}` | Delete a user by ID                |

### Example API Usage

```bash
# List all users
curl http://localhost:3000/api/users

# Create a user
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "name=John&email=john@example.com"

# Get a specific user
curl http://localhost:3000/api/users/{uuid}

# Delete a user
curl -X DELETE http://localhost:3000/api/users/{uuid}
```

## Testing

```bash
# Run all tests
cargo test -p calendsync

# Run with output
cargo test -p calendsync -- --nocapture

# Run a specific test
cargo test -p calendsync test_create_and_get_user
```

## Project Structure

```
crates/calendsync/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs           # Entry point, server setup, graceful shutdown
│   ├── app.rs            # Router construction, middleware, tests
│   ├── error.rs          # AppError type with anyhow integration
│   ├── state.rs          # AppState with user repository
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── pages.rs      # HTML page handlers
│   │   └── api.rs        # JSON/HTML API handlers
│   └── models/
│       ├── mod.rs
│       └── user.rs       # User model
└── templates/
    ├── base.html         # Base layout with HTMX
    ├── index.html        # Main page
    └── partials/
        ├── user_row.html     # Single user row
        └── user_table.html   # User table body
```

## Environment Variables

- `RUST_LOG` - Configure logging level (default: `calendsync=debug,tower_http=debug`)

Example:

```bash
RUST_LOG=calendsync=info,tower_http=warn cargo run -p calendsync
```
