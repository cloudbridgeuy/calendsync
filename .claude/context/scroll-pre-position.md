# Scroll Pre-Position

Prevents the flash of wrong days on page refresh by pre-positioning scroll content with CSS transforms during SSR.

## Problem

On page refresh, the browser renders the virtual scroll window at `scrollLeft=0`, showing buffer days (the far-left edge of the window) before JavaScript hydrates and sets the correct scroll position. This creates a visible flash.

## Solution

A CSS class `.scroll-pre-position` applies `translateX()` to shift content to approximately the correct position before the browser paints. On hydration, `useLayoutEffect` removes the class and sets `scrollLeft` — both happen before the browser paints, making the transition invisible.

## CSS Variables

Three CSS variables drive the pre-position calculation:

| Variable | Purpose |
|----------|---------|
| `--scroll-offset-days` | Number of days to offset (`bufferDays - floor(visibleDays / 2)`) |
| `--day-width` | Full-viewport day width (used for scroll math) |
| `--schedule-day-width` | Day width minus hour column (used for visual column layout) |

`--scroll-offset-days` varies per breakpoint to match the JS `calculateScrollPosition()` formula:

| Breakpoint | Visible Days | `--scroll-offset-days` |
|------------|-------------|----------------------|
| `<500px` | 1 | 10 |
| `500–749px` | 1 (75% width) | 10 |
| `750–1249px` | 3 | 9 |
| `1250–1749px` | 5 | 8 |
| `≥1750px` | 7 | 7 |

## Compact Mode vs Schedule Mode

### Compact Mode

Transform on `.days-scroll` — no sticky elements, straightforward:

```css
.days-scroll.scroll-pre-position {
  transform: translateX(calc(-1 * var(--scroll-offset-days) * var(--day-width)));
}
```

### Schedule Mode

The schedule grid has sticky-left elements (hour column, corner, all-day label) that must NOT be shifted — `scrollLeft` leaves them pinned via `position: sticky`, so the transform must do the same. Only the three content children are transformed:

```css
.schedule-grid.scroll-pre-position .schedule-day-headers,
.schedule-grid.scroll-pre-position .schedule-all-day-events,
.schedule-grid.scroll-pre-position .schedule-timed-grid {
  transform: translateX(calc(-1 * var(--scroll-offset-days) * var(--day-width)));
}
```

**Critical:** Uses `--day-width`, not `--schedule-day-width`. The JS `calculateScrollPosition()` computes offsets using `dayWidth = containerWidth / visibleDays` (matching `--day-width`). Using `--schedule-day-width` (which subtracts 60px for the hour column) produces a different pixel offset, causing a visible jump when `scrollLeft` replaces the transform.

## React Integration

`useVirtualScroll` manages a `scrollPrePositioned` boolean state:
- Starts as `true` (SSR and initial client render)
- Set to `false` in the `useLayoutEffect` that initializes scroll position (before paint)
- Exposed through `CalendarContext` so `Calendar.Days` can apply the CSS class

## Files

| File | Role |
|------|------|
| `styles.css` | CSS variables and `.scroll-pre-position` rules |
| `hooks/useVirtualScroll.ts` | `scrollPrePositioned` state management |
| `contexts/CalendarContext.tsx` | Context interface with `scrollPrePositioned` |
| `components/Calendar.tsx` | Applies class to `.days-scroll` and `<ScheduleGrid>` |
| `components/ScheduleGrid.tsx` | Accepts `className` prop on root |
