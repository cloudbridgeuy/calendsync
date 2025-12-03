# SSR Worker Pool Architecture

This document describes the SSR (Server-Side Rendering) worker pool implementation for React calendar rendering using `deno_core`.

## Overview

The SSR system is split into two crates following the **Functional Core - Imperative Shell** pattern:

| Crate | Purpose | Dependencies |
|-------|---------|--------------|
| `calendsync_ssr_core` | Pure functions (config, validation, polyfills) | serde, thiserror |
| `calendsync_ssr` | I/O operations (worker pool, threading, runtime) | deno_core, tokio |

## Why a Worker Pool?

`deno_core::JsRuntime` is **not `Send`**, meaning it cannot be moved between threads. This prevents direct use in async Tokio handlers. The solution:

1. Each worker runs in a **dedicated OS thread**
2. Workers have their own single-threaded Tokio runtime
3. Main server communicates via **channels** (mpsc for requests, oneshot for responses)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Axum Handler                            │
│  calendar_react_ssr(State, Path) -> Response                │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                     SsrPool                                 │
│  - Round-robin worker selection                             │
│  - Backpressure (Overloaded error if no capacity)           │
│  - Timeout handling (10s default)                           │
└─────────────────────┬───────────────────────────────────────┘
                      │ mpsc::channel
                      ▼
┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
│ Worker 0 │  │ Worker 1 │  │ Worker 2 │  │ Worker N │
│ (Thread) │  │ (Thread) │  │ (Thread) │  │ (Thread) │
└────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘
     │             │             │             │
     ▼             ▼             ▼             ▼
┌──────────────────────────────────────────────────────────┐
│              JsRuntime (per worker)                      │
│  - Polyfills injected                                    │
│  - React bundle executed                                 │
│  - HTML returned via thread-local storage                │
└──────────────────────────────────────────────────────────┘
```

## Crate Structure

### calendsync_ssr_core (Functional Core)

```
crates/ssr_core/
├── Cargo.toml
└── src/
    ├── lib.rs           # Public API exports
    ├── config.rs        # SsrConfig, SsrPoolConfig with validation
    ├── error.rs         # SsrCoreError (validation errors only)
    └── polyfills.rs     # Pure polyfill generation (14 tests)
```

**Key Types:**
- `SsrConfig` - Render request config with 5MB payload limit
- `SsrPoolConfig` - Pool settings (workers, timeout, max_pending)
- `generate_polyfills()` - Pure function generating JS polyfills

### calendsync_ssr (Imperative Shell)

```
crates/ssr/
├── Cargo.toml
└── src/
    ├── lib.rs           # Re-exports core + shell types
    ├── error.rs         # SsrError with I/O variants, sanitize_error()
    ├── runtime.rs       # JsRuntime execution, thread-local HTML
    ├── worker.rs        # SsrWorker thread management
    └── pool.rs          # SsrPool orchestration, health checks
```

**Key Types:**
- `SsrPool` - Main entry point, manages worker threads
- `SsrWorker` - Dedicated thread with JsRuntime
- `HealthStatus` - Active health check result
- `SsrPoolStats` - Passive statistics

## Usage

### Initialization (main.rs)

```rust
use calendsync_ssr::{SsrPool, SsrPoolConfig};

// Determine worker count from CPU parallelism
let worker_count = std::thread::available_parallelism()
    .map(|p| p.get())
    .unwrap_or(4);

// Create pool config (10s timeout, production mode)
let pool_config = SsrPoolConfig::with_defaults(worker_count)?;

// Create pool (reads bundle, spawns threads)
let pool = SsrPool::new(pool_config, &bundle_path)?;

// Add to AppState
let state = AppState::with_demo_data().with_ssr_pool(pool);
```

### Rendering (handler)

```rust
use calendsync_ssr::{SsrConfig, SsrError, sanitize_error};

// Build initial data
let initial_data = serde_json::json!({
    "calendarId": calendar_id.to_string(),
    "highlightedDay": today.to_string(),
    "days": days,
    "clientBundleUrl": client_bundle_url,
});

// Create config with validation
let config = SsrConfig::new(serde_json::json!({
    "initialData": initial_data,
}))?;

// Render
match state.ssr_pool.as_ref().unwrap().render(config).await {
    Ok(html) => Html(html).into_response(),
    Err(SsrError::Overloaded { retry_after_secs }) => {
        // Return 503 with Retry-After header
        (StatusCode::SERVICE_UNAVAILABLE,
         [("Retry-After", retry_after_secs.to_string())],
         Html(error_html)).into_response()
    }
    Err(e) => Html(error_html(&sanitize_error(&e))).into_response(),
}
```

## Health Check Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/health/ssr` | GET | Active probe - renders minimal page, returns latency |
| `/health/ssr/stats` | GET | Passive stats - worker count, capacity (fast) |

**Example responses:**

```bash
# Passive stats
curl /health/ssr/stats
{"worker_count":8,"workers_with_capacity":8}

# Active probe
curl /health/ssr
{"healthy":true,"latency_ms":45,"worker_idx":2,"stats":{"worker_count":8,"workers_with_capacity":7}}
```

**Kubernetes probe configuration:**
```yaml
livenessProbe:
  httpGet:
    path: /health/ssr/stats  # Fast, passive
    port: 3000
readinessProbe:
  httpGet:
    path: /health/ssr        # Active render probe
    port: 3000
  initialDelaySeconds: 5
```

## Security Features

1. **Safe JSON injection** - Config is double-encoded via `JSON.parse()` to prevent XSS
2. **Payload size limit** - 5MB max for initial data
3. **Error sanitization** - `sanitize_error()` hides internal details from clients
4. **Bundle path validation** - Must be a `.js` file, canonicalized path

## Error Handling

| Error | Client Message | HTTP Status |
|-------|----------------|-------------|
| `Timeout(ms)` | "Render timed out after {ms}ms" | 200 (with error HTML) |
| `Overloaded` | "Service busy, retry after {n}s" | 503 + Retry-After |
| `ChannelClosed` | "Service temporarily unavailable" | 200 (with error HTML) |
| `BundleLoad` | "Internal configuration error" | 200 (with error HTML) |
| `JsExecution` | "Render failed" | 200 (with error HTML) |

## Testing

```bash
# Run unit tests (14 tests in core, integration tests in shell)
cargo test -p calendsync_ssr_core
cargo test -p calendsync_ssr

# Health check endpoints
curl http://localhost:3000/health/ssr/stats
curl http://localhost:3000/health/ssr
```

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `worker_count` | CPU parallelism | Number of worker threads |
| `max_pending` | 100 | Max queued requests per worker |
| `render_timeout_ms` | 10,000 | Render timeout in milliseconds |
| `node_env` | "production" | NODE_ENV for React |

## Polyfills

The following Web APIs are polyfilled for React 19 SSR in deno_core:

- `console` (log, error, warn, info, debug)
- `performance.now()`
- `MessageChannel` (React scheduler)
- `TextEncoder` / `TextDecoder`
- `ReadableStream` / `WritableStream` / `TransformStream`
- `process.env.NODE_ENV`
- `queueMicrotask`

## Related Files

- Handler: `crates/calendsync/src/handlers/calendar_react.rs`
- State: `crates/calendsync/src/state.rs` (ssr_pool field)
- Initialization: `crates/calendsync/src/main.rs` (init_ssr_pool)
- Health checks: `crates/calendsync/src/handlers/health.rs`
