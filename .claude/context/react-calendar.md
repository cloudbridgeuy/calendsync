# React Calendar (calendar-react)

The React calendar is a server-side rendered (SSR) calendar view with real-time updates via Server-Sent Events (SSE).

## Route

- **URL**: `/calendar/{calendar_id}` - React SSR calendar page
- **Handler**: `crates/calendsync/src/handlers/calendar_react.rs`

## Architecture

### Server-Side Rendering

Uses `deno_core` to render React 19 with the prerender API:

1. Rust handler receives request at `/calendar/{calendar_id}`
2. Loads entries for the calendar from AppState
3. Runs JavaScript bundle (`calendar-react-server.js`) in deno_core
4. React prerenders the calendar HTML with initial data
5. Returns HTML with hydration script

### Client Hydration

The client bundle (`calendar-react-client-[hash].js`) hydrates the server-rendered HTML:

1. Attaches event handlers to server-rendered DOM
2. Connects to SSE endpoint for real-time updates
3. Manages local state (notifications, highlighted day)

### Real-Time Updates (SSE)

**Endpoint**: `GET /api/events?calendar_id={uuid}`

**Handler**: `crates/calendsync/src/handlers/events.rs`

Streams calendar events:
- `entry_added` - New entry created
- `entry_updated` - Entry modified
- `entry_deleted` - Entry removed

Features:
- Event history for reconnection catch-up (`last_event_id` parameter)
- Graceful shutdown signaling via broadcast channel
- Keep-alive pings every 30 seconds
- Demo mode generates random events every 3-8 seconds

### Notification Center

The notification center displays real-time SSE events:

**Component**: `crates/frontend/src/calendar-react/components/NotificationCenter.tsx`

**Features**:
- Bell icon in top-right header with unread badge
- Dropdown panel with notification list
- SVG icons for each notification type (added/updated/deleted)
- Mark as read, mark all read, clear all actions
- Card-styled items with colored accents

**Styling** (`styles.css`):
- Panel uses `--bg-secondary` background
- Items are cards with `--bg-tertiary` background and 8px border-radius
- Unread items have orange left border (`--accent`)
- Dark mode icon colors with semi-transparent backgrounds

## File Structure

```
crates/frontend/src/calendar-react/
├── server.tsx           # SSR entry point (prerender)
├── client.tsx           # Client hydration entry point
├── styles.css           # All component styles
├── types.ts             # TypeScript type definitions
├── hooks/               # React hooks
│   ├── useCalendar.ts   # Calendar state management
│   ├── useNotifications.ts # Notification center state
│   └── useSSE.ts        # SSE connection management
└── components/
    ├── Calendar.tsx     # Main calendar component
    ├── DayColumn.tsx    # Single day view
    ├── EntryTile.tsx    # Calendar entry card
    ├── NotificationCenter.tsx # Bell + dropdown panel
    └── Header.tsx       # Calendar header with navigation
```

## Build Integration

Frontend build scripts in `package.json`:

```bash
# Server bundle (no hash, for deno_core)
bun run build:calendar-react:server

# Client bundle (hashed, for browser caching)
bun run build:calendar-react:client

# CSS (copied directly)
bun run build:calendar-react:css
```

Output files:
- `dist/calendar-react-server.js` - Server bundle (loaded by Rust)
- `dist/calendar-react-client-[hash].js` - Client bundle (loaded in browser)
- `dist/calendar-react.css` - Styles (loaded in browser)

## State Management

### AppState Extensions

`crates/calendsync/src/state.rs` includes SSE support:

```rust
pub struct AppState {
    // ... existing fields ...

    /// Event counter for generating unique event IDs
    pub event_counter: Arc<AtomicU64>,
    /// Event history for SSE reconnection catch-up
    pub event_history: Arc<RwLock<VecDeque<StoredEvent>>>,
    /// Shutdown signal sender for SSE connections
    pub shutdown_tx: broadcast::Sender<()>,
}
```

### CalendarEvent Types

```rust
pub enum CalendarEvent {
    EntryAdded { entry: CalendarEntry, date: String },
    EntryUpdated { entry: CalendarEntry, date: String },
    EntryDeleted { entry_id: Uuid, date: String },
}
```

## API Endpoints

### Calendar Entries API

**Endpoint**: `GET /api/calendar-entries`

Returns entries in `ServerDay[]` format for React calendar:

```typescript
interface ServerDay {
    date: string;          // ISO date (YYYY-MM-DD)
    entries: ServerEntry[];
}

interface ServerEntry {
    id: string;
    title: string;
    kind: "event" | "task";
    status: "pending" | "completed";
    startTime?: string;    // HH:MM
    endTime?: string;      // HH:MM
    // ... other fields
}
```

**Query Parameters**:
- `calendar_id` - UUID of calendar
- `highlighted_day` - Center date (ISO format)
- `before` - Days before highlighted_day (default: 3)
- `after` - Days after highlighted_day (default: 3)

## Graceful Shutdown

The server signals SSE handlers to close connections on shutdown:

```rust
// In main.rs
async fn shutdown_signal(state: AppState) {
    // Wait for Ctrl+C or SIGTERM...

    // Signal SSE handlers to close
    state.signal_shutdown();
}
```

SSE handlers subscribe to shutdown and close cleanly:

```rust
let mut shutdown_rx = state.subscribe_shutdown();

// In stream...
tokio::select! {
    _ = shutdown_rx.recv() => break,
    // ... handle events ...
}
```

## Development

```bash
# Run server with React calendar
cargo run -p calendsync -- --port 3000

# Access calendar
open http://localhost:3000/calendar/{calendar_id}

# Get calendar ID from API
curl http://localhost:3000/api/calendars
```

## CSS Variables

The calendar uses CSS custom properties for theming:

```css
--bg-primary    /* Main background */
--bg-secondary  /* Panel backgrounds */
--bg-tertiary   /* Card backgrounds */
--text-primary  /* Main text */
--text-secondary /* Muted text */
--border        /* Border color */
--accent        /* Orange accent (#F97316) */
```
