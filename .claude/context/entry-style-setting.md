# Entry Style Setting

The calendar supports two visual styles for entry rendering: **Compact** and **Filled**.

## Overview

- **Compact**: Border-only accent color with neutral background
- **Filled**: Full background color with white text

The setting applies globally across both Compact and Schedule views, persists via localStorage per calendar, and defaults to "Compact".

## Architecture

### Settings Flow

```
localStorage → parseSettingsJson() → useCalendarSettings → CalendarContext → Components
                                                    ↓
                                          SettingsMenuContext → StyleToggle UI
```

### Type Definitions

```typescript
// crates/frontend/src/core/calendar/settings.ts
type EntryStyle = "compact" | "filled"

interface CalendarSettings {
  viewMode: ViewMode
  showTasks: boolean
  entryStyle: EntryStyle  // NEW
}
```

### Pure Functions

```typescript
// Update entry style (immutable)
updateEntryStyle(settings, entryStyle) → CalendarSettings
```

## Components

### StyleToggle

Located in `SettingsMenu.tsx` as a compound component sub-component:

```tsx
<SettingsMenu.StyleToggle />
```

Renders radio buttons for "Compact" and "Filled" options.

### Entry Components

All three entry components read `entryStyle` from `CalendarContext` and apply styles conditionally:

| Component | File | Color Property |
|-----------|------|----------------|
| EntryTile | `EntryTile.tsx` | `borderLeftColor` or `backgroundColor` |
| AllDayEntryTile | `AllDayEntryTile.tsx` | `borderLeftColor` or `backgroundColor` |
| ScheduleTimedEntry | `ScheduleTimedEntry.tsx` | `borderLeftColor` or `backgroundColor` |

### CSS Classes

Each component applies an `entry-style-{compact|filled}` class:

```css
/* Compact style */
.entry-tile.entry-style-compact { ... }
.all-day-entry.entry-style-compact { ... }
.schedule-timed-entry.entry-style-compact { ... }

/* Filled style */
.entry-tile.entry-style-filled { ... }
.all-day-entry.entry-style-filled { ... }
.schedule-timed-entry.entry-style-filled { ... }
```

## localStorage

Key format: `calendsync_settings_{calendarId}`

Example stored value:
```json
{
  "viewMode": "schedule",
  "showTasks": true,
  "entryStyle": "filled"
}
```

Backward compatible: Missing `entryStyle` falls back to `"compact"`.

## Files

| File | Purpose |
|------|---------|
| `settings.ts` | Pure functions, types, constants |
| `settings.test.ts` | Unit tests for settings functions |
| `useCalendarSettings.ts` | Hook with localStorage persistence |
| `CalendarContext.tsx` | Exposes `setEntryStyle` to components |
| `SettingsMenuContext.tsx` | Settings panel state management |
| `SettingsMenu.tsx` | `StyleToggle` UI component |
| `EntryTile.tsx` | Compact view entry rendering |
| `AllDayEntryTile.tsx` | All-day section entry rendering |
| `ScheduleTimedEntry.tsx` | Schedule view timed entry rendering |
| `styles.css` | CSS variants for both styles |
