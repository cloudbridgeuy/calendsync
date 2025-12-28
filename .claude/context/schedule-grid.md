# ScheduleGrid Component

The ScheduleGrid is a compound component implementing a CSS Grid-based layout for schedule mode, inspired by Fantastical's calendar design.

## Architecture

### CSS Grid Layout

The component uses a 2-column, 3-row grid structure:

```
+------------------+---------------------------+
| Corner cell      | Day headers (sticky top)  |  Row 1
+------------------+---------------------------+
| "all-day" label  | All-day events            |  Row 2 (sticky below row 1)
+------------------+---------------------------+
| Hour column      | Timed content grid        |  Row 3 (scrollable)
| (sticky left)    |                           |
+------------------+---------------------------+
```

### Grid Definition

```css
.schedule-grid {
  display: grid;
  grid-template-columns: 60px 1fr;
  grid-template-rows: auto auto 1fr;
}
```

### Sticky Behavior

Multiple elements have sticky positioning for a polished scroll experience:

| Element | Position | Z-Index | Description |
|---------|----------|---------|-------------|
| Corner | top + left | 10 | Stays fixed at top-left corner |
| Day Headers | top | 9 | Horizontal scroll with content |
| All-Day Label | top (70px) + left | 8 | Below headers, fixed left |
| All-Day Events | top (70px) | 7 | Below headers, scrolls horizontally |
| Hour Column | left | 5 | Scrolls vertically with content |

## Component Structure

### Compound Pattern

Uses the compound component pattern for flexible composition:

```tsx
<ScheduleGrid>
  <ScheduleGrid.Corner />
  <ScheduleGrid.DayHeaders />
  <ScheduleGrid.AllDayLabel />
  <ScheduleGrid.AllDayEvents />
  <ScheduleGrid.HourColumn />
  <ScheduleGrid.TimedGrid />
</ScheduleGrid>
```

### Sub-Components

| Component | Grid Position | Purpose |
|-----------|---------------|---------|
| `Corner` | row 1, col 1 | Displays timezone abbreviation |
| `DayHeaders` | row 1, col 2 | Clickable day headers with date info |
| `AllDayLabel` | row 2, col 1 | "all-day" text label |
| `AllDayEvents` | row 2, col 2 | All-day, multi-day, and task entries |
| `HourColumn` | row 3, col 1 | 24-hour time labels |
| `TimedGrid` | row 3, col 2 | Timed event entries |

### Context

ScheduleGrid uses its own context to share values with sub-components:

```typescript
interface ScheduleGridContextValue {
  renderedDates: Date[]
  getEntriesForDate: (date: Date) => ServerEntry[]
  dayWidth: number
  highlightedDate: Date
  scrollToDate: (date: Date) => void
}
```

## All-Day Section

### Entry Categories

The all-day section displays three types of entries:

1. **Multi-day events**: Events spanning multiple days
2. **All-day events**: Events without specific times
3. **Tasks**: Checkbox-based todo items

### Collapsible Overflow

When more than 3 all-day/multi-day events exist:

- Shows first 3 events when collapsed
- "(+N more)" toggle expands to show all
- "Show less" toggle collapses back

### Task Toggle

Tasks are hidden by default:

- "(N tasks)" toggle shows task count
- Clicking reveals task checkboxes
- Tasks shown below events when expanded

### Pure Layout Functions

All layout logic lives in `core/calendar/allDayLayout.ts`:

```typescript
// Categorize entries into events and tasks
categorizeAllDayEntries(entries: ServerEntry[]): AllDayCategorized

// Compute visible vs hidden entries
computeAllDaySummary(entries: ServerEntry[], showOverflow: boolean): AllDaySummary

// Format toggle button text
formatOverflowToggle(hiddenCount: number, isExpanded: boolean): string | null
formatTasksToggle(taskCount: number, isExpanded: boolean): string | null
```

## Files

| File | Purpose |
|------|---------|
| `components/ScheduleGrid.tsx` | Compound component implementation |
| `components/AllDayEntryTile.tsx` | Individual all-day/task entry tile |
| `components/AllDayToggle.tsx` | Toggle button for overflow/tasks |
| `core/calendar/allDayLayout.ts` | Pure layout computation functions |
| `core/calendar/__tests__/allDayLayout.test.ts` | Unit tests for layout functions |
| `styles.css` | CSS Grid and sticky positioning styles |

## State Management

Toggle states are lifted to CalendarContext:

```typescript
// In CalendarContext
showAllDayOverflow: boolean      // Expand overflow entries
setShowAllDayOverflow: (show: boolean) => void
showAllDayTasks: boolean         // Show task entries
setShowAllDayTasks: (show: boolean) => void
```

This allows the states to persist when scrolling and be shared across all day columns.

## Usage

Schedule mode in Calendar.tsx uses ScheduleGrid:

```tsx
{isScheduleMode ? (
  <ScheduleGrid>
    <ScheduleGrid.Corner />
    <ScheduleGrid.DayHeaders />
    <ScheduleGrid.AllDayLabel />
    <ScheduleGrid.AllDayEvents />
    <ScheduleGrid.HourColumn />
    <ScheduleGrid.TimedGrid />
  </ScheduleGrid>
) : (
  <div className="days-scroll">
    <VirtualDaysContent />
  </div>
)}
```

The legacy `AllDaySection` component remains for backwards compatibility but is no longer used in the main Calendar view.
