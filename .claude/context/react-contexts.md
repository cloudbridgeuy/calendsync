# React Contexts

React Contexts eliminate props drilling and enable compound component patterns in the frontend.

## CalendarContext

**Location:** `crates/frontend/src/calendsync/contexts/CalendarContext.tsx`

Provides shared state to calendar sub-components (DayColumn, EntryTile) without threading props through intermediate components.

### Context Value

```typescript
interface CalendarContextValue {
    flashStates: Map<string, FlashState>  // Entry animation states
    onEntryClick: (entry: ServerEntry) => void  // Opens edit modal
    onEntryToggle: (entry: ServerEntry) => void  // Toggles task checkbox
    isMobile: boolean  // Viewport detection
}
```

### Usage

**Provider** (in Calendar.tsx):
```tsx
<CalendarProvider
    flashStates={flashStates}
    onEntryClick={handleEntryClick}
    onEntryToggle={handleEntryToggle}
    isMobile={isMobile}
>
    {/* DayColumn and EntryTile children */}
</CalendarProvider>
```

**Consumer** (in EntryTile.tsx):
```tsx
const { flashStates, onEntryClick, onEntryToggle } = useCalendarContext()
```

### Benefits

- Eliminates 4 props from DayColumn interface
- EntryTile self-contained (reads from context)
- Easier to add new consumers without prop threading

## ARIA Utilities

**Location:** `crates/frontend/src/core/calendar/aria.ts`

Pure functions for ARIA accessibility support, following the Functional Core pattern.

### buildAriaIds

Generates coordinated ARIA IDs for trigger/content component pairs:

```typescript
const { triggerId, contentId } = buildAriaIds("notification-center")
// triggerId: "notification-center-trigger"
// contentId: "notification-center-content"
```

Used in compound components for proper ARIA relationships:
- `aria-controls={contentId}` on trigger
- `aria-labelledby={triggerId}` on content panel

## File Structure

```
crates/frontend/src/
├── calendsync/
│   └── contexts/
│       ├── index.ts              # Barrel export
│       ├── CalendarContext.tsx   # Calendar state context
│       └── NotificationContext.tsx  # (future) Notification compound component
└── core/calendar/
    └── aria.ts                   # Pure ARIA utilities
```

## Adding New Contexts

1. Create `[Name]Context.tsx` in `contexts/`
2. Define `[Name]ContextValue` interface
3. Create context with `createContext<T | null>(null)`
4. Create provider component with `useMemo` for value
5. Create `use[Name]Context` hook with null check
6. Export from `contexts/index.ts`
