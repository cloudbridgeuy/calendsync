# Now Indicator

A real-time current-time indicator in the schedule view. Shows a red horizontal line and a time label in the hour column. Updates every 30 seconds and auto-scrolls to the current time on mount.

## Visual Parts

| Element | Location | Description |
|---------|----------|-------------|
| Time label | Hour column (sticky left) | Red "6:38 PM" text with background mask, z-index 6 |
| Horizontal line | Timed grid (full width) | 2px red line spanning all day columns, z-index 3 |

All positioned at `top: ((hours * 60 + minutes) / 1440) * 100%`.

## Pure Functions

Located in `core/calendar/nowIndicator.ts`:

| Function | Purpose |
|----------|---------|
| `calculateNowPositionPercent(hours, minutes)` | Vertical position as percentage (0–100) |
| `formatNowLabel(hours, minutes)` | 12-hour time string (e.g. "6:38 PM") |
| `calculateScrollToCurrentTime(hours, minutes, viewportHeight, totalHeight)` | Clamped scroll offset for upper-third centering |

## Hook

`useCurrentTime(intervalMs = 30_000)` — Returns a `Date` that refreshes on the given interval. Used in `ScheduleGridRoot` to drive position updates.

## CSS

- `--now-indicator: #ef4444` custom property on `:root`
- `.now-indicator-line` — absolute positioned, full-width red line
- `.now-time-label` — absolute positioned in the hour column with `var(--bg-secondary)` background to mask hour labels

All elements use `pointer-events: none`.

## Integration Points

- **ScheduleGrid context** — `now: Date` field added to `ScheduleGridContextValue`
- **HourColumn** — Renders `<span className="now-time-label">` at the computed percentage
- **TimedGrid** — Renders `<div className="now-indicator-line">`
- **Calendar.tsx Days** — Uses `calculateScrollToCurrentTime` instead of fixed 8 AM scroll when entering schedule mode

## Files

| File | Purpose |
|------|---------|
| `core/calendar/nowIndicator.ts` | Pure position/formatting functions |
| `core/calendar/__tests__/nowIndicator.test.ts` | Unit tests |
| `hooks/useCurrentTime.ts` | Interval-based Date hook |
| `components/ScheduleGrid.tsx` | Renders indicator elements |
| `components/Calendar.tsx` | Auto-scroll on mount |
| `styles.css` | Indicator styling |
