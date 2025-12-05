# React Calendar

The React calendar is a server-side rendered (SSR) calendar view with real-time updates via Server-Sent Events (SSE).

## Routes

| URL | Description |
|-----|-------------|
| `/calendar/{calendar_id}` | React SSR calendar page (modal closed) |
| `/calendar/{calendar_id}/entry` | Calendar with create modal open |
| `/calendar/{calendar_id}/entry?entry_id={id}` | Calendar with edit modal open |

**Handler**: `crates/calendsync/src/handlers/calendar_react.rs`

## Architecture

### Server-Side Rendering

Uses `deno_core` to render React 19 with the prerender API:

1. Rust handler receives request at `/calendar/{calendar_id}`
2. Loads entries for the calendar from AppState
3. Runs JavaScript bundle (`calendsync-server.js`) in deno_core
4. React prerenders the calendar HTML with initial data
5. Returns HTML with hydration script

### Client Hydration

The client bundle (`calendsync-client-[hash].js`) hydrates the server-rendered HTML:

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

**Component**: `crates/frontend/src/calendsync/components/NotificationCenter.tsx`

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
crates/frontend/src/calendsync/
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
bun run build:calendsync:server

# Client bundle (hashed, for browser caching)
bun run build:calendsync:client

# CSS (copied directly)
bun run build:calendsync:css
```

Output files:
- `dist/calendsync-server.js` - Server bundle (loaded by Rust)
- `dist/calendsync-client-[hash].js` - Client bundle (loaded in browser)
- `dist/calendsync.css` - Styles (loaded in browser)

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

## Responsive Layout

The calendar adapts to viewport width, showing different numbers of day columns:

### Breakpoints

| Viewport Width | Visible Days | Layout |
|----------------|--------------|--------|
| < 768px | 1 | Mobile (swipe navigation) |
| 750-1249px | 3 | Narrow desktop |
| 1250-1749px | 5 | Medium desktop |
| ≥ 1750px | 7 | Wide desktop |

### Layout Constants

Defined in `crates/frontend/src/core/calendar/types.ts`:

```typescript
export const DEFAULT_LAYOUT_CONSTANTS: LayoutConstants = {
    minDayWidth: 250,        // Minimum 250px per day column
    mobileBreakpoint: 768,   // Below this = mobile (1 day)
    swipeThreshold: 50,      // Swipe distance to navigate
    velocityThreshold: 0.3,  // Fast swipe detection
    animationDuration: 200,  // Transition timing
    mobileBuffer: 30,        // Days rendered for mobile infinite scroll
}
```

### Pure Calculation Functions

Located in `crates/frontend/src/core/calendar/layout.ts`:

- `calculateVisibleDays(containerWidth)` - Returns odd number (1, 3, 5, or 7)
- `isMobileViewport(containerWidth)` - Boolean check against breakpoint
- `calculateDayWidth(containerWidth, visibleDays)` - Per-column width

### Hydration and Layout Timing

**Critical**: Layout calculation uses `useLayoutEffect` (not `useEffect`) to ensure correct column count before browser paint:

```typescript
// In useCalendarState.ts
useLayoutEffect(() => {
    if (typeof window !== "undefined") {
        updateLayout(window.innerWidth)
        window.addEventListener("resize", handleResize)
        return () => window.removeEventListener("resize", handleResize)
    }
}, [updateLayout])
```

**Why `useLayoutEffect`?**
- SSR defaults to desktop layout (7 days)
- `useEffect` runs AFTER paint → flash of 7 cramped columns on mobile
- `useLayoutEffect` runs BEFORE paint → correct layout from first render

### Mobile Rendering

When `isMobile = true`:
- Renders `mobileBuffer * 2 + 1 = 61` day columns for smooth swiping
- Each column is 100% width
- Uses CSS transform (`translateX`) for swipe animation
- Only center day visible, others off-screen

### Desktop Rendering

When `isMobile = false`:
- Renders exactly `visibleDays` columns (3, 5, or 7)
- Each column width = `100 / visibleDays` percent
- No swipe animation, uses arrow/wheel navigation

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

## Entry Modal

The calendar includes a URL-based modal for creating and editing entries.

### URL-Based Modal State

The modal state is controlled by the URL, enabling:
- Deep linking to create/edit forms
- Browser back/forward navigation
- SSR with modal open state

**URL Scheme**:
- `/calendar/{id}` - Modal closed
- `/calendar/{id}/entry` - Create mode (highlighted day pre-filled)
- `/calendar/{id}/entry?entry_id={uuid}` - Edit mode (entry data pre-loaded)

### Navigation Behavior

| Action | Navigation Method | Effect |
|--------|------------------|--------|
| Open modal (click FAB or entry) | `history.pushState` | Adds to history |
| Cancel/Escape/Click outside | `history.back()` | Natural back button behavior |
| Save entry | `history.replaceState` | Prevents re-opening on back |

### Architecture (Functional Core - Imperative Shell)

**Functional Core** (`crates/frontend/src/core/calendar/modal.ts`):
- `parseModalUrl(pathname, search)` - Parse URL to modal state
- `buildModalUrl(calendarId, mode, entryId?)` - Build modal URL
- `entryToFormData(entry)` - Convert ServerEntry to form data
- `formDataToApiPayload(data, calendarId)` - Convert form to API params
- `validateFormData(data)` - Validate form fields

**Imperative Shell** (hooks):
- `useModalUrl` - URL/history management
- `useEntryApi` - Entry CRUD operations

### File Structure

```
crates/frontend/src/
├── core/calendar/
│   └── modal.ts                    # Pure modal utilities
├── calendsync/
│   ├── components/
│   │   └── EntryModal.tsx          # Modal component
│   ├── hooks/
│   │   ├── useModalUrl.ts          # URL state hook
│   │   └── useEntryApi.ts          # Entry API hook
│   └── types.ts                    # ModalState, EntryFormData
└── ...
crates/calendsync/src/handlers/
└── calendar_react.rs               # SSR handlers (calendar_react_ssr_entry)
```

### InitialData Extension

The `InitialData` interface includes optional modal state for SSR:

```typescript
interface InitialData {
    // ... existing fields ...
    modal?: {
        mode: "create" | "edit"
        entryId?: string        // Edit mode
        entry?: ServerEntry     // Pre-fetched entry (edit mode SSR)
        defaultDate?: string    // Create mode
    }
}
```
