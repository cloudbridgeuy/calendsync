# Tauri Desktop Setup

## Running the Desktop App

```bash
cargo xtask dev desktop
```

This starts the Tauri desktop application with:
- Frontend dev server at `http://localhost:5173`
- WebSocket HMR at `ws://localhost:5174`
- Hot-reload for both frontend and Rust changes

## Architecture: Transport Abstraction Layer

The Tauri webview cannot make direct HTTP requests due to CSP/CORS restrictions. We use a transport abstraction layer to allow web and desktop to share the same hooks and components while using different HTTP implementations.

```
┌─────────────────────────────────────────────────────────┐
│                    Shared Hooks                         │
│  useCalendarState(), useOfflineCalendar(), etc.        │
│                        │                                │
│                        ▼                                │
│               TransportContext                          │
│           (provides transport adapter)                  │
│                        │                                │
│            ┌──────────┴──────────┐                     │
│            ▼                     ▼                      │
│    WebTransport           TauriTransport               │
│  (fetch + cookies)      (invoke -> Rust HTTP)          │
└─────────────────────────────────────────────────────────┘
```

**Web**: Direct `fetch()` with cookies - browser handles session cookies automatically.

**Desktop (Tauri)**: `invoke()` → Rust backend → `reqwest` - all HTTP requests are proxied through Rust commands.

### Key Files

| File | Purpose |
|------|---------|
| `crates/frontend/src/core/transport/types.ts` | Transport interface definition |
| `crates/frontend/src/core/transport/tauri.ts` | TauriTransport using `invoke()` |
| `crates/frontend/src/core/transport/context.tsx` | React context provider |
| `crates/src-tauri/src/http.rs` | Rust HTTP client functions |
| `crates/src-tauri/src/commands.rs` | Tauri command handlers |

## SSE (Server-Sent Events) in Tauri

SSE is **disabled** in Tauri because the browser's native `EventSource` API makes direct HTTP requests that bypass the transport layer. This causes authentication failures since the webview doesn't have access to the session cookie.

**Solution**: The `InitialData` type has an `sseEnabled` flag. Tauri sets `sseEnabled: false` when creating initial data:

```typescript
// crates/frontend/src/tauri/App.tsx
setInitialData({
  calendarId,
  highlightedDay,
  days,
  clientBundleUrl: "",
  controlPlaneUrl: "",
  sseEnabled: false, // Disabled in Tauri
})
```

The `useSseWithOffline` hook respects this flag and doesn't create an EventSource connection when disabled.

**Visual feedback still works**: The SSE event handlers (`onSseEntryAdded`, `onSseEntryUpdated`, `onSseEntryDeleted`) are exposed as actions from `useCalendarState`. These trigger flash animations, toasts, and notifications. When SSE is disabled, the visual feedback just won't be triggered by real-time server events.

## Deep-Link Plugin Configuration

The `tauri-plugin-deep-link` v2 expects an **object** format in `tauri.conf.json`:

```json
{
    "plugins": {
        "deep-link": {
            "desktop": {
                "schemes": ["calendsync"]
            }
        }
    }
}
```

The `desktop.schemes` field is an array of URL schemes to associate with this app.

## CSP Configuration

The CSP in `tauri.conf.json` must include `ipc:` and `tauri:` protocols for IPC to work:

```json
"csp": "default-src 'self'; ... connect-src 'self' ipc: tauri: http://localhost:3000 ..."
```

## Common Errors

### PluginInitialization "deep-link" error

```
error while running tauri application: PluginInitialization("deep-link",
"Error deserializing 'plugins.deep-link' within your Tauri configuration:
invalid type: map, expected a sequence")
```

**Cause:** Stale build cache or configuration mismatch.

**Fix:** Clean rebuild - `cargo clean -p calendsync_tauri && cargo xtask dev desktop`

### EventSource MIME type error

```
EventSource's response has a MIME type ("text/html") that is not "text/event-stream".
```

**Cause:** Duplicate SSE connections or `sseEnabled` flag not set correctly.

**Fix:** Ensure `sseEnabled: false` is set in Tauri's `App.tsx` and only `useSseWithOffline` (not `useSse`) is used for SSE.

## Debugging with Authenticated Session

When developing the Tauri desktop app, you may need to test features that require authentication. The `CALENDSYNC_DEV_SESSION` environment variable allows you to transfer an authenticated session from the web app to the desktop app.

### How It Works

1. Log in to the web app at `http://localhost:3000`
2. Open the Dev Menu (visible in dev mode)
3. Click "Copy Desktop Command" to copy the command with your session ID
4. Run the copied command in your terminal

### Usage

```bash
# Copy this command from the web app's Dev Menu
CALENDSYNC_DEV_SESSION=<session_id> cargo xtask dev desktop
```

The session ID is a UUID that identifies your authenticated session. When set, the Tauri app will use this session to authenticate HTTP requests instead of requiring a separate login.

### Security Notes

- Session IDs are only exposed in dev mode (not in production builds)
- The session transfer is one-time and for development only
- Sessions expire according to normal session timeout rules

## Build Commands

| Command | Description |
|---------|-------------|
| `cargo xtask dev desktop` | Run desktop app in dev mode |
| `CALENDSYNC_DEV_SESSION=<id> cargo xtask dev desktop` | Run with transferred auth session |
| `cargo tauri build` | Build desktop app for distribution |
| `cargo tauri dev` | Alternative to xtask (direct Tauri CLI) |

## File Structure

```
crates/src-tauri/
├── tauri.conf.json     # Main Tauri configuration
├── src/
│   ├── lib.rs          # App entry point and plugin setup
│   ├── commands.rs     # Tauri command handlers
│   ├── http.rs         # HTTP client functions (reqwest)
│   ├── auth.rs         # Session management
│   └── main.rs         # Desktop entry point
├── capabilities/
│   └── default.json    # Permission capabilities
└── icons/              # App icons for bundling
```
