# Lazy SSR Initialization

The SSR worker pool initializes in the background after the server starts, reducing startup time from 30+ seconds to <1 second.

## Problem

The SSR pool initialization blocks server startup:
- Each `deno_core` runtime takes 3-5 seconds to initialize
- With 4-8 workers (based on CPU cores), this blocks for 24-40 seconds
- Seeding and health checks must wait for SSR before proceeding

## Solution

Initialize SSR in background after server starts listening:

```
Before: main() → init_ssr_pool() [BLOCKS 30s] → listen()
After:  main() → listen() → spawn(init_ssr_pool)
```

## Health Endpoints

Three Kubernetes-style health endpoints differentiate availability:

| Endpoint | Purpose | SSR Required |
|----------|---------|--------------|
| `/livez` | Server accepting connections | No |
| `/healthz` | SSR pool stats (passive) | Yes |
| `/readyz` | SSR can render (active check) | Yes |

Seeding polls `/livez` instead of `/healthz`, allowing immediate startup.

## Graceful Degradation

Calendar handlers return 503 with `Retry-After: 5` header when SSR is still initializing:

```rust
let Some(ssr_pool) = state.get_ssr_pool().await else {
    return (
        StatusCode::SERVICE_UNAVAILABLE,
        [("Retry-After", "5")],
        Html(initializing_html(...)),
    ).into_response();
};
```

## Key Files

| File | Role |
|------|------|
| `handlers/health.rs` | `/livez` endpoint |
| `state.rs` | `set_ssr_pool()` async method |
| `main.rs` | Background spawn of SSR init |
| `handlers/calendar_react.rs` | 503 graceful degradation |
| `xtask/src/dev/seed.rs` | Uses `/livez` for fast startup |
