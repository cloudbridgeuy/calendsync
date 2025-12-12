# TypeScript Hot-Reload for SSR

This document describes the hot-reload feature for TypeScript/CSS changes during development.

## Overview

When running `cargo xtask dev web`, the system watches for changes in `crates/frontend/src/` and automatically:

1. Rebuilds the frontend (`bun run build:dev`)
2. Reloads the SSR worker pool with the new bundle
3. Signals the browser to refresh automatically

## Architecture

```
cargo xtask dev web
├── Starts server (cargo run -p calendsync) with DEV_MODE=1
├── Waits for server health check (/healthz)
├── Watches crates/frontend/src/ for changes
└── On change (debounced 500ms):
    ├── Runs bun run build:dev
    └── POST /_dev/reload → Server swaps SSR pool → Broadcasts reload event
```

## Components

### Server-side (`crates/calendsync`)

**`state.rs`**: SSR pool wrapped in `Arc<RwLock<...>>` for atomic swap

```rust
pub ssr_pool: Arc<TokioRwLock<Option<Arc<SsrPool>>>>
pub dev_reload_tx: broadcast::Sender<()>  // For browser auto-refresh
```

**`handlers/dev.rs`**: Dev-only endpoints (debug builds only)
- `POST /_dev/reload`: Swaps SSR pool and broadcasts reload signal
- `GET /_dev/events`: SSE endpoint for browser auto-refresh

**`main.rs`**: Runtime manifest reading when `DEV_MODE` is set
- Dev mode: reads manifest from `crates/frontend/manifest.json`
- Production: uses compiled-in manifest

**`handlers/calendar_react.rs`**: Runtime client bundle URL + devMode flag
- Passes `devMode: true` to frontend when DEV_MODE is set
- Frontend injects auto-refresh script when devMode is true

**`handlers/static_files.rs`**: CSS cache-busting
- Dev mode: `Cache-Control: no-cache` for CSS files
- Production: normal caching rules

### Frontend (`crates/frontend/src/calendsync`)

**`App.tsx`**: Conditionally injects auto-refresh script
- When `initialData.devMode` is true, adds SSE listener
- On "reload" event, calls `location.reload()`

### xtask (`xtask/src/dev/web.rs`)

- Spawns server with `DEV_MODE=1`
- File watcher using `notify-debouncer-mini`
- Triggers rebuild and reload on changes

## Usage

```bash
# With hot-reload + auto-refresh (default)
cargo xtask dev web

# With hot-reload, but manual browser refresh
cargo xtask dev web --no-auto-refresh

# Without hot-reload (Rust changes only)
cargo xtask dev web --no-hot-reload

# Release mode (no hot-reload)
cargo xtask dev web --release
```

## Workflow

1. Start: `cargo xtask dev web`
2. Wait for "Ready for changes" message
3. Browser shows "[Dev] Auto-refresh connected" in console
4. Edit TypeScript/CSS in `crates/frontend/src/`
5. See "🔄 Change detected, rebuilding..."
6. See "✓ Reloaded!"
7. Browser automatically refreshes with new changes

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| Build fails | Log error, keep old bundle, server continues |
| Reload endpoint fails | Log error, keep old bundle, server continues |
| Server crashes | xtask detects exit and terminates |
| Rapid changes | Debouncer coalesces (500ms window) |
| Concurrent requests during swap | RwLock ensures safe access |
| SSE connection lost | Browser auto-reconnects (EventSource default) |

## Configuration

| Environment Variable | Effect |
|---------------------|--------|
| `DEV_MODE` | Enables dev routes, runtime manifest reading |
| `DEV_NO_AUTO_REFRESH` | Disables browser auto-refresh (set by --no-auto-refresh) |
| `PORT` | Server port (default: 3000) |

## Files Modified

| File | Purpose |
|------|---------|
| `crates/calendsync/src/state.rs` | Swappable SSR pool + reload broadcast |
| `crates/calendsync/src/handlers/dev.rs` | Reload endpoint + SSE events endpoint |
| `crates/calendsync/src/handlers/calendar_react.rs` | Runtime bundle URLs (JS + CSS) + devMode |
| `crates/frontend/scripts/build-css.ts` | CSS content hashing during build |
| `crates/calendsync/src/handlers/static_files.rs` | CSS cache-busting |
| `crates/calendsync/src/app.rs` | Conditional dev routes |
| `crates/calendsync/src/main.rs` | Runtime manifest |
| `crates/frontend/src/calendsync/App.tsx` | Auto-refresh script injection |
| `crates/frontend/src/calendsync/types.ts` | devMode in InitialData |
| `xtask/src/dev/web.rs` | File watcher, reload orchestration |

## Tauri (Desktop/iOS) - Not Affected

The hot-reload system is **completely isolated from Tauri apps**:

| Aspect | Web (`cargo xtask dev web`) | Tauri (`cargo xtask dev desktop/ios`) |
|--------|------------------------------|---------------------------------------|
| Dev server | calendsync SSR (port 3000) | Vite dev server (port 5173) |
| Rendering | Server-side (React SSR) | Client-side (Vite + React) |
| HMR mechanism | Custom SSE-based reload | Vite's built-in HMR |
| `DEV_MODE` env | ✅ Set | ❌ Not set |
| Auto-refresh script | ✅ Injected | ❌ Never injected |

**Why they don't interfere:**
1. `cargo xtask dev desktop` and `cargo xtask dev ios` do NOT set `DEV_MODE`
2. Tauri uses `devUrl: "http://localhost:5173"` (Vite), not the calendsync web server
3. The auto-refresh script is only injected when `devMode: true` in SSR data
4. Tauri apps use client-side rendering, not SSR

## Limitations

- **Not true HMR**: Full page refresh (state is lost)
- **Debug builds only**: Dev endpoints not available in release builds
- **Web only**: Desktop/iOS apps use Tauri's Vite-based HMR instead
