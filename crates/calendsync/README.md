# CalendSync Web Server

The main axum web server providing REST API and React SSR.

## Tech Stack

- **Axum** - Web framework
- **React 19** - Frontend UI with SSR
- **deno_core** - JavaScript runtime for SSR
- **Tokio** - Async runtime
- **Tower-HTTP** - Middleware (CORS, tracing, timeouts)

## Running the Server

```bash
# Standard mode
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

The server uses React SSR via deno_core:

1. Request comes in for `/calendar/{id}`
2. Server fetches calendar data from in-memory store
3. `SsrPool` renders React to HTML using deno_core workers
4. HTML returned with embedded initial data
5. Client-side React hydrates for interactivity
6. SSE connection provides real-time updates

See `crates/ssr/` and `crates/ssr_core/` for SSR implementation details.

## Project Structure

```
src/
├── main.rs         # Entry point, graceful shutdown
├── app.rs          # Router, middleware
├── state.rs        # AppState, SSE support
├── mock_data.rs    # Demo data generation
├── handlers/
│   ├── entries.rs      # Entry CRUD
│   ├── calendar_react.rs  # React SSR handler
│   ├── events.rs       # SSE handler
│   └── health.rs       # Health endpoints
└── models/
    └── entry.rs    # Request types
```

## Environment Variables

- `RUST_LOG` - Logging level (default: info)

```bash
RUST_LOG=debug cargo run -p calendsync
```

## Testing

```bash
# Run all tests
cargo test -p calendsync

# Run with output
cargo test -p calendsync -- --nocapture
```
