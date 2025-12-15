# Responsive Day Column Layout

The calendar uses a responsive layout system that determines how many day columns are visible based on viewport width, and how those columns are positioned within the viewport.

## Viewport Breakpoints

| Viewport Width  | Visible Days | Day Width          | Notes                                      |
| --------------- | ------------ | ------------------ | ------------------------------------------ |
| < 500px         | 1            | 100% of viewport   | Full-width single day for mobile           |
| 500px - 749px   | 1            | 75% of viewport    | Centered day with partial columns visible  |
| 750px - 1249px  | 3            | 33.3% of viewport  | Three days fill viewport exactly           |
| 1250px - 1749px | 5            | 20% of viewport    | Five days fill viewport exactly            |
| >= 1750px       | 7            | 14.3% of viewport  | Seven days fill viewport exactly           |

## Core Functions

### `calculateVisibleDays(containerWidth: number): number`

Returns the number of days that should be visible based on container width.

```typescript
calculateVisibleDays(400)  // Returns 1 (single day, full width)
calculateVisibleDays(600)  // Returns 1 (single day, 75% width)
calculateVisibleDays(900)  // Returns 3 (three days)
calculateVisibleDays(1500) // Returns 5 (five days)
calculateVisibleDays(1920) // Returns 7 (seven days)
```

### `calculateDayWidth(containerWidth: number, visibleDays: number): number`

Returns the width of each day column in pixels.

```typescript
calculateDayWidth(375, 1)  // Returns 375 (full width < 500px)
calculateDayWidth(600, 1)  // Returns 450 (75% of 600 for 500-749px range)
calculateDayWidth(900, 3)  // Returns 300 (900 / 3)
```

## Special Case: 75% Width (500-749px)

For viewports between 500px and 749px, the day column is 75% of the viewport width instead of 100%. This is a visual design choice that:

1. Provides a better visual appearance than cramped 50% columns
2. Shows partial buffer columns on both sides (12.5% each)
3. Gives the user a visual hint that they can scroll

### Scroll Centering for 75% Width

When the day width doesn't fill the viewport, the scroll position centers the day(s) in the viewport:

```typescript
// For 600px viewport with 75% width (450px day):
// centerOffset = (600 - 450) / 2 = 75px
// scrollLeft = dayIndex * dayWidth - centerOffset

// Result: Day is centered with 75px of partial columns on each side
```

## Scroll Centering Algorithm

The `calculateScrollPosition` function positions the scroll container to center the highlighted day within the visible day group:

```typescript
export function calculateScrollPosition(
  targetDate: Date,
  windowStartDate: Date,
  dayWidth: number,
  containerWidth: number,
  visibleDays: number,
): number {
  const dayIndex = /* calculate from dates */

  // How many days appear before the centered day
  const daysBeforeCenter = Math.floor(visibleDays / 2)

  // First visible day index
  const firstVisibleDayIndex = dayIndex - daysBeforeCenter

  // Expected total width of visible days
  const expectedVisibleWidth = dayWidth * visibleDays

  // If visible days don't fill viewport (75% case), center them
  if (expectedVisibleWidth < containerWidth) {
    const centerOffset = (containerWidth - expectedVisibleWidth) / 2
    return Math.max(0, firstVisibleDayIndex * dayWidth - centerOffset)
  }

  // Normal case: align first visible day with left edge
  return Math.max(0, firstVisibleDayIndex * dayWidth)
}
```

## Visual Behavior

| Viewport | Visible Days | Partial Columns | Day Position |
| -------- | ------------ | --------------- | ------------ |
| < 500px  | 1            | None            | Fills viewport |
| 500-749px| 1            | Yes (12.5% each)| Centered |
| >= 750px | 3, 5, or 7   | None            | Fill viewport exactly |

## Files

- **Pure functions**: `src/core/calendar/virtualScroll.ts`
- **Navigation**: `src/core/calendar/navigation.ts`
- **Hook**: `src/calendsync/hooks/useVirtualScroll.ts`
- **Tests**: `src/core/calendar/__tests__/virtualScroll.test.ts`
