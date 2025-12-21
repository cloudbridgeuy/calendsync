# TypeScript Hot-Reload for SSR

This document describes the hot-reload feature for TypeScript/CSS changes during development.

## Overview

When running `cargo xtask dev server`, the system watches for changes in `crates/frontend/src/` and automatically:

1. Rebuilds the frontend (`bun run build:dev`)
2. Reloads the SSR worker pool with the new bundle
3. Signals the browser to refresh automatically

## Architecture

```
cargo xtask dev server
├── Starts server (cargo run -p calendsync) with DEV_MODE=1
├── Waits for server health check (/healthz)
├── Watches crates/frontend/src/ for changes
└── On change (debounced 500ms):
    ├── Runs bun run build:dev
    │   ├── Builds server JS (calendsync-server.js)
    │   ├── Builds client JS (calendsync-client-[hash].js)
    │   ├── Builds CSS (calendsync-[hash].css)
    │   └── Updates manifest.json with new filenames
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
- `POST /_dev/reload`: Detects change type (none/css_only/client_only/full), conditionally swaps SSR pool
- `POST /_dev/error`: Receives build errors from xtask, broadcasts to browsers
- `GET /_dev/events`: SSE endpoint with events: `reload`, `css-reload`, `build-error`

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
- Handles `reload` event: full page refresh
- Handles `css-reload` event: hot-swaps stylesheet without reload
- Handles `build-error` event: displays error overlay with dismiss button

### xtask (`xtask/src/dev/web.rs`)

- Spawns server with `DEV_MODE=1`
- File watcher using `notify-debouncer-mini`
- Triggers rebuild and reload on changes

### Build Scripts (`crates/frontend/scripts`)

**`build-css.ts`**: Builds CSS with content hashing
- Computes SHA256 hash of CSS content
- Outputs `calendsync-[hash].css`
- Removes old CSS files
- Updates `manifest.json` with CSS entry

**`update-manifest.ts`**: Updates manifest with JS filenames
- Scans dist directory for latest assets
- Removes old hashed files (cleanup)
- Updates `manifest.json` with all entries (server JS, client JS, CSS)

**Why both scripts?** During `cargo build`, `build.rs` handles manifest generation. But hot-reload runs `bun run build:dev` directly, which bypasses `build.rs`. The `update-manifest.ts` script ensures the manifest stays current during development.

## Usage

```bash
# With hot-reload + auto-refresh (default)
cargo xtask dev server

# With hot-reload, but manual browser refresh
cargo xtask dev server --no-auto-refresh

# Without hot-reload (Rust changes only)
cargo xtask dev server --no-hot-reload

# Release mode (no hot-reload)
cargo xtask dev server --release
```

## Workflow

1. Start: `cargo xtask dev server`
2. Wait for "Ready for changes" message
3. Browser shows "[Dev] Auto-refresh connected" in console
4. Edit TypeScript/CSS in `crates/frontend/src/`
5. See "🔄 Change detected, rebuilding..."
6. See "✓ Reloaded!"
7. Browser automatically refreshes with new changes

## Edge Cases

| Scenario | Behavior |
|----------|----------|
| Build fails | Shows error overlay in browser, keeps old bundle |
| CSS-only change | Hot-swap CSS without full reload or pool swap |
| Client JS change | Page reload without pool swap |
| Server JS change | Pool swap + page reload |
| No changes (same hashes) | Skip reload entirely |
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
| `crates/frontend/scripts/update-manifest.ts` | Updates manifest.json with latest asset filenames |
| `crates/calendsync/src/handlers/static_files.rs` | CSS cache-busting |
| `crates/calendsync/src/app.rs` | Conditional dev routes |
| `crates/calendsync/src/main.rs` | Runtime manifest |
| `crates/frontend/src/calendsync/App.tsx` | Auto-refresh script injection |
| `crates/frontend/src/calendsync/types.ts` | devMode in InitialData |
| `xtask/src/dev/web.rs` | File watcher, reload orchestration |

## Tauri (Desktop/iOS) - Not Affected

The hot-reload system is **completely isolated from Tauri apps**:

| Aspect | Web (`cargo xtask dev server`) | Tauri (`cargo xtask dev desktop/ios`) |
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

## SSE Event Types

| Event | Data | Browser Action |
|-------|------|----------------|
| `connected` | `{}` | Log connection |
| `reload` | `{}` | Full page refresh |
| `css-reload` | `{"filename": "calendsync-xxx.css"}` | Hot-swap stylesheet |
| `build-error` | `{"error": "..."}` | Show error overlay |

## Change Detection Matrix

The reload endpoint tracks three assets and determines the minimal action:

| Server JS | Client JS | CSS | SSR Pool Swap | Browser Action |
|-----------|-----------|-----|---------------|----------------|
| No | No | No | No | Skip entirely |
| No | No | Yes | No | CSS hot-swap |
| No | Yes | * | No | Full reload |
| Yes | * | * | Yes | Full reload |

**Optimization**: SSR pool swap is expensive (~100ms). Only server JS changes require it since:
- CSS has no effect on SSR output
- Client JS is not used during SSR (hydration-only)

## Limitations

- **Not true HMR**: Full page refresh (state is lost), except CSS hot-swap
- **Debug builds only**: Dev endpoints not available in release builds
- **Web only**: Desktop/iOS apps use Tauri's Vite-based HMR instead
