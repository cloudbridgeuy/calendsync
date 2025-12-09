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
│   ├── useCalendarState.ts   # Calendar state management
│   ├── useVirtualScroll.ts   # Virtual scroll hook
│   ├── useNotificationCenter.ts # Notification center state
│   └── useSse.ts        # SSE connection management
└── components/
    ├── Calendar.tsx     # Main calendar compound component
    ├── CalendarHeader.tsx # Month/year display header
    ├── DayContainer.tsx # Day container compound component (header + content)
    ├── DayColumn.tsx    # Entry tiles for a single day
    ├── EntryTile.tsx    # Calendar entry card
    ├── NotificationCenter.tsx # Bell + dropdown panel
    └── EntryModal.tsx   # Entry create/edit modal
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

## Virtual Scroll Architecture

The calendar uses native browser scrolling with a virtual scroll window for infinite day navigation.

### Architecture Overview

```
┌─────────────────────────────────────────────────┐
│  MONTH YEAR (fixed, from highlightedDate)       │
├─────────────────────────────────────────────────┤
│  ┌─────────┐ ┌─────────┐ ┌─────────┐           │
│  │ 8 MON   │ │ 9 TUE   │ │ 10 WED  │  ← Fixed  │
│  │         │ │ TODAY   │ │         │    headers│
│  ├─────────┤ ├─────────┤ ├─────────┤           │
│  │         │ │         │ │         │           │
│  │ Entries │ │ Entries │ │ Entries │  ← Scroll │
│  │ (vert)  │ │ (vert)  │ │ (vert)  │    vert.  │
│  └─────────┘ └─────────┘ └─────────┘           │
│  ←────── Horizontal scroll (native) ──────→    │
└─────────────────────────────────────────────────┘
```

### Key Components

- **CalendarHeader** - Shows only month/year based on `highlightedDate`
- **DayContainer** - Compound component with sticky header + scrollable content
- **DayColumn** - Entry tiles for a single day

### Virtual Scroll Strategy

The virtual scroll renders a fixed window of 21 days (10 buffer + center + 10 buffer).
When scrolling near edges, the window shifts and scroll position adjusts instantly for infinite scrolling.

### Scroll Container Hierarchy

The calendar uses a carefully structured DOM hierarchy for scrolling:

```html
<main class="entry-container scroll-container">  <!-- Scrolling element -->
  <div class="days-scroll">                       <!-- Flex container -->
    <!-- DayContainer components -->
  </div>
</main>
```

**Critical CSS Requirements**:
- `.scroll-container` is the scrolling element with `overflow-x: scroll`
- `.days-scroll` is a flex container that must NOT have `overflow-x` or `width: 100%` on desktop
- If `.days-scroll` constrains width, no overflow occurs and scroll events won't fire on the parent
- The `isScrollable` guard in `useLayoutEffect` ensures initial scroll position is only set when content actually overflows

### Responsive Breakpoints

| Container Width | Visible Days | Layout |
|-----------------|--------------|--------|
| < 480px | 1 | Mobile portrait |
| 480-767px | 3 | Mobile landscape / small tablet |
| 768-1023px | 5 | Tablet |
| 1024-1439px | 5 | Desktop |
| ≥ 1440px | 7 | Large desktop |

**Note**: Mobile/desktop behavior is controlled entirely via CSS media queries at `767px` breakpoint. There is no `isMobile` JavaScript state - only `visibleDays` derived from viewport width.

### Pure Calculation Functions

Located in `crates/frontend/src/core/calendar/virtualScroll.ts`:

- `calculateVisibleDays(containerWidth)` - Returns 1, 3, 5, or 7 based on breakpoints
- `calculateDayWidth(containerWidth, visibleDays)` - Width per day column
- `calculateVirtualWindow(centerDate, config)` - Array of dates in virtual window
- `calculateScrollPosition(targetDate, windowStart, dayWidth, containerWidth)` - ScrollLeft to center a date
- `calculateHighlightedDay(scrollLeft, containerWidth, dayWidth, windowStart)` - Date at viewport center
- `shouldRecenter(scrollLeft, totalWidth, containerWidth, dayWidth, threshold)` - Detect edge proximity
- `calculateRecenterOffset(direction, windowStart, dayWidth, shiftDays)` - New window position + scroll adjustment

Located in `crates/frontend/src/core/calendar/navigation.ts`:

- `isScrollable(scrollWidth, clientWidth)` - Check if element has scrollable content
- `calculateCenteredScrollPosition(targetDayIndex, dayWidth, containerWidth, totalContentWidth)` - Calculate scroll position to center a date
- `calculateCenterDayIndex(scrollLeft, containerWidth, dayWidth)` - Calculate which day is at viewport center
- `detectEdgeProximity(scrollLeft, maxScroll, thresholdPx)` - Detect if near scroll edges

### Virtual Scroll Hook

`useVirtualScroll` in `crates/frontend/src/calendsync/hooks/useVirtualScroll.ts` provides:

- `scrollContainerRef` - Ref for scroll container
- `highlightedDate` - Date closest to viewport center
- `renderedDates` - Array of dates to render
- `dayWidth` - Width of each day column
- `visibleDays` - Number of visible days
- `scrollToDate(date, animated?)` - Programmatic navigation
- `scrollToToday()` - Jump to today

### Single-Day Snap Scrolling

When `visibleDays === 1` (mobile portrait), the calendar snaps to show a complete day after scrolling ends:

**Pure Functions** (`virtualScroll.ts`):
- `shouldSnapToDay(visibleDays)` - Returns true only when `visibleDays === 1`
- `calculateSnapScrollPosition(scrollLeft, dayWidth, containerWidth, windowStartDate)` - Calculates snap target

**Behavior**:
- Snap only triggers after user releases touch/mouse (not during active drag)
- 50ms debounce after scroll stops
- Instant snap (no animation) for responsive feel
- Tracks `isDraggingRef` via touch/mouse events to prevent snapping during drag

### Click-to-Navigate Headers

Day headers are clickable and navigate to center that day:

```typescript
<DayContainer
  onHeaderClick={() => scrollToDate(date)}
>
  <DayContainer.Header />  {/* Clickable */}
</DayContainer>
```

**Accessibility**:
- `role="button"` and `tabIndex={0}` on header
- Keyboard support (Enter/Space)
- `aria-label` with day name and number

### Day Container Compound Component

`DayContainer` in `crates/frontend/src/calendsync/components/DayContainer.tsx`:

```typescript
<DayContainer date={date} dayWidth={dayWidth} isHighlighted={isHighlighted}>
  <DayContainer.Header />
  <DayContainer.Content>
    <DayColumn dateKey={dateKey} entries={entries} />
  </DayContainer.Content>
</DayContainer>
```

### Navigation Feedback

- **Haptic**: Vibrates on day change (if supported)
- **Audio**: Short tick sound on day change (if supported)

### Keyboard Navigation

- **Arrow Left/Right**: Navigate days
- **T key**: Jump to today

### Floating Buttons

Two pill-shaped floating buttons provide quick actions:

| Button | Position | Visibility | Action |
|--------|----------|------------|--------|
| **New** | Bottom-right | Always visible | Opens create entry modal for highlighted day |
| **Today** | Bottom-left | Hidden when viewing today | Scrolls to today's date |

Both buttons share the same visual style:
- Pill shape (`border-radius: 999px`)
- Orange accent background (`--accent`)
- Responsive sizing (larger on desktop)

**Components**:
- `Fab` - New button (in `Calendar.tsx`)
- `TodayButton` - Today button (in `TodayButton.tsx`)

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
