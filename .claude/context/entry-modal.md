# Entry Modal

URL-based modal for creating and editing calendar entries.

## URL Scheme

| URL | Description |
|-----|-------------|
| `/calendar/{id}` | Calendar view (modal closed) |
| `/calendar/{id}/entry` | Create mode (highlighted day pre-filled) |
| `/calendar/{id}/entry?entry_id={uuid}` | Edit mode (entry data pre-loaded) |

## Navigation Behavior

| Action | Method | Effect |
|--------|--------|--------|
| Open modal (FAB/entry click) | `history.pushState` | Adds to history |
| Cancel/Escape/Click outside | `history.back()` | Natural back button |
| Save entry | `history.replaceState` | Prevents re-open on back |

## Architecture

Follows Functional Core - Imperative Shell pattern:

### Functional Core (`core/calendar/modal.ts`)

Pure functions with no side effects:

```typescript
// Parse URL to determine modal state
parseModalUrl(pathname, search) → { mode, entryId } | null

// Build modal URLs
buildModalUrl(calendarId, mode, entryId?) → string
buildCalendarUrl(calendarId) → string

// Form data conversion
entryToFormData(entry: ServerEntry) → EntryFormData
formDataToApiPayload(data, calendarId) → URLSearchParams

// Validation
validateFormData(data) → { valid: boolean, errors: string[] }
```

### Imperative Shell (React hooks)

**`useModalUrl`** - URL/history state management:
- Parses current URL on mount
- Listens to `popstate` events
- Provides `openCreateModal`, `openEditModal`, `closeModal`, `closeAfterSave`

**`useEntryApi`** - Entry CRUD operations:
- `createEntry(data)` → POST `/api/entries`
- `updateEntry(entryId, data)` → PUT `/api/entries/{id}`
- `deleteEntry(entryId)` → DELETE `/api/entries/{id}`
- `fetchEntry(entryId)` → GET `/api/entries/{id}`

## SSR Support

The modal state is included in `InitialData` for server-side rendering:

```typescript
interface InitialData {
    // ... existing fields ...
    modal?: {
        mode: "create" | "edit"
        entryId?: string        // Edit mode
        entry?: ServerEntry     // Pre-fetched entry data
        defaultDate?: string    // Create mode
    }
}
```

Backend handlers:
- `calendar_react_ssr` - `/calendar/{id}` (no modal)
- `calendar_react_ssr_entry` - `/calendar/{id}/entry` (modal open)

## File Structure

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
└── calendar_react.rs               # SSR handlers
```

## Form Fields

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| Title | text | Yes | Entry name |
| Date | date | Yes | ISO format (YYYY-MM-DD) |
| All Day | checkbox | - | Toggles time fields |
| Start Time | time | No | HH:MM format |
| End Time | time | No | Must be after start |
| Description | textarea | No | Additional details |
| Location | text | No | Where the event occurs |

## Types

```typescript
interface ModalState {
    mode: "create" | "edit"
    entryId?: string
    entry?: ServerEntry
    defaultDate?: string
}

interface EntryFormData {
    title: string
    date: string
    startTime?: string
    endTime?: string
    isAllDay: boolean
    description?: string
    location?: string
    entryType: "all_day" | "timed" | "task" | "multi_day"
    endDate?: string
}
```

## Accessibility

- Modal has `role="dialog"` and `aria-modal="true"`
- Title linked via `aria-labelledby`
- Escape key closes modal
- Form inputs have associated labels
- Entry tiles are keyboard accessible (`tabIndex`, Enter/Space)
