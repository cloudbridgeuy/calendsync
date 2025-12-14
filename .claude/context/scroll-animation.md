# Scroll Animation System

The calendar uses a custom scroll animation system for reliable, fast day header click navigation.

## Problem Solved

Native browser `scrollTo({ behavior: "smooth" })` has several issues:
- Too slow for responsive feel
- Gets interrupted by momentum scroll (trackpad/touch)
- Race conditions with virtual scroll re-centering
- Fails to trigger unpredictably

## Architecture

Follows Functional Core - Imperative Shell pattern:

```
core/calendar/
├── scrollAnimation.ts    # Pure animation math (easing, progress, position)
└── accessibility.ts      # Pure ARIA/focus selectors

calendsync/hooks/
├── useScrollAnimation.ts # RAF loop, cancellation, state management
└── useAriaAnnouncer.ts   # ARIA live region for announcements
```

## Animation Configuration

| Property | Value | Notes |
|----------|-------|-------|
| Duration | ~180ms | Fast, responsive feel |
| Easing | ease-out cubic | Fast start, smooth deceleration |
| Scaling | 1x-1.5x based on distance | Longer scrolls slightly slower |

## Key Behaviors

### Momentum Cancellation

Before starting animation, momentum scroll is cancelled:

```typescript
// Self-assignment trick cancels browser momentum
container.scrollLeft = container.scrollLeft
```

This works on both desktop (trackpad) and mobile (touch).

### Animation Interruption

New clicks cancel previous animation and start fresh from current position:

```typescript
const animateScrollTo = (targetPosition: number) => {
  cancelAnimation() // Cancel existing
  // ... start new animation
}
```

### Re-centering Disabled During Animation

The virtual scroll re-centering logic is disabled during animation to prevent conflicts:

```typescript
if (recenterDirection && !isAnimating()) {
  // Only re-center when not animating
}
```

### Accessibility

1. **ARIA Announcement**: Screen readers hear "Navigated to [day] [number]"
2. **Focus Management**: After scroll completes:
   - Focus moves to first entry in target day
   - If day is empty, focus moves to day header

## Integration Points

### useVirtualScroll

The `scrollToDate` function uses custom animation for visible days:

```typescript
if (animated) {
  cancelAnimation()
  pendingFocusDateRef.current = formatDateKey(targetDate)
  announce(generateNavigationAnnouncement(targetDate))
  animateScrollTo(targetScrollPosition)
} else {
  container.scrollTo({ left: targetScrollPosition, behavior: "instant" })
}
```

### DayContainer

Day headers have `data-date` attribute for focus targeting:

```tsx
<div
  className="day-container-header"
  data-date={formatDateKey(date)}
  tabIndex={0}
  // ...
>
```

## CSS Requirements

The `.sr-only` class is required for ARIA live region:

```css
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
```

## Testing

Unit tests cover pure functions in `scrollAnimation.test.ts`:
- Easing function behavior
- Progress calculation
- Position interpolation
- Duration scaling

Manual testing needed for:
- Mobile Safari (aggressive momentum)
- Rapid click interruption
- Screen reader announcements (VoiceOver, NVDA)

## Files

| File | Purpose |
|------|---------|
| `core/calendar/scrollAnimation.ts` | Pure animation functions |
| `core/calendar/accessibility.ts` | ARIA and focus utilities |
| `hooks/useScrollAnimation.ts` | RAF loop and state |
| `hooks/useAriaAnnouncer.ts` | ARIA live region |
| `hooks/useVirtualScroll.ts` | Integration point |
| `components/DayContainer.tsx` | data-date attribute |
| `styles.css` | .sr-only class |
