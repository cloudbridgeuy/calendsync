# Running CalendSync Applications

This document explains how to run and connect the different CalendSync applications.

## Architecture Overview

CalendSync has three client types that all communicate with a central axum server:

1. **Web** - React SSR served directly by the server
2. **Desktop** - Tauri app with CSR, connects via HTTP
3. **Mobile** - Tauri iOS app with CSR, connects via HTTP

All clients use:
- REST API for CRUD operations
- SSE (Server-Sent Events) for real-time updates

## Server Details

**Location**: `crates/calendsync/`

**Start command**:
```bash
cargo run -p calendsync
```

**Default port**: 3000

**Key endpoints**:
- `GET /calendar/{id}` - SSR calendar page (web)
- `GET /api/calendar-entries` - Get entries for a date range
- `POST /api/entries` - Create entry
- `PUT /api/entries/{id}` - Update entry
- `DELETE /api/entries/{id}` - Delete entry
- `GET /api/events?calendar_id={id}` - SSE stream

**Environment variables**:
- `RUST_LOG` - Logging level (default: info)

## Web Client

No separate client needed - the server serves SSR pages.

**How it works**:
1. Browser requests `/calendar/{id}`
2. Server renders React with deno_core
3. Returns full HTML with embedded data
4. Client-side React hydrates for interactivity
5. SSE connection established for real-time updates

**Entry point**: `crates/frontend/src/calendsync/server.tsx` (SSR)
**Hydration**: `crates/frontend/src/calendsync/client.tsx`

## Desktop Client (Tauri)

**Location**: `crates/src-tauri/`

**Build type**: CSR (Client-Side Rendering)

**Start command**:
```bash
cargo tauri dev  # From workspace root
```

**How it works**:
1. Tauri opens WebView
2. Loads static HTML from `crates/frontend/dist/index.html`
3. React fetches data from `http://localhost:3000/api/*`
4. SSE connection for real-time updates

**Entry points**:
- Rust: `crates/src-tauri/src/main.rs` (desktop), `lib.rs` (mobile)
- TypeScript: `crates/frontend/src/tauri/client.tsx`

**Configuration**:
- API URL: Hardcoded in `crates/frontend/src/tauri/client.tsx`
- HTTP permissions: `crates/src-tauri/capabilities/default.json`
- CSP rules: `crates/src-tauri/tauri.conf.json`

**Dev server**:
- Runs on port 5173 (`crates/frontend/dev-server.ts`)
- Hot reload via WebSocket on port 5174

## Mobile Client (iOS)

**Location**: `crates/src-tauri/` (same as desktop)

**First-time setup**:
```bash
cargo tauri ios init
```

This generates Xcode project in `crates/src-tauri/gen/apple/`.

**Simulator**:
```bash
cargo tauri ios dev
```

**Physical device**:
```bash
cargo tauri ios dev --host
```

The `--host` flag:
- Uses `TAURI_DEV_HOST` environment variable
- Makes dev server accessible on local network
- Required because localhost doesn't work on physical devices

**Physical device limitations**:
- App hardcoded to `http://localhost:3000`
- For physical device testing, either:
  1. Modify API URL in `client.tsx`
  2. Use network tunneling (ngrok, etc.)
  3. Deploy server to accessible host

## Running All Three Together

```bash
# Terminal 1: Server
cargo run -p calendsync

# Terminal 2: Desktop (optional)
cargo tauri dev

# Terminal 3: iOS Simulator (optional)
cargo tauri ios dev
```

All clients connect to the same server and see synchronized updates via SSE.

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| "Failed to load calendar" | Server not running | Start with `cargo run -p calendsync` |
| Tauri build fails | Missing icons | Run `cargo tauri icon` |
| iOS can't connect | Wrong host | Use simulator, or modify API URL |
| CORS errors | Browser origin blocked | Server allows localhost by default |
| SSE not working | Connection blocked | Check CSP in tauri.conf.json |

## File Reference

| File | Purpose |
|------|---------|
| `crates/calendsync/src/main.rs` | Server entry point |
| `crates/frontend/src/calendsync/server.tsx` | Web SSR entry |
| `crates/frontend/src/calendsync/client.tsx` | Web hydration |
| `crates/frontend/src/tauri/client.tsx` | Desktop/mobile entry |
| `crates/frontend/src/tauri/App.tsx` | Desktop/mobile App wrapper |
| `crates/src-tauri/tauri.conf.json` | Tauri configuration |
| `crates/src-tauri/capabilities/default.json` | HTTP permissions |
