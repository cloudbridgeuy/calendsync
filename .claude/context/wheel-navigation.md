# Wheel Navigation

The React calendar supports wheel/trackpad navigation with direction lock.

## Behavior

| Gesture | Action |
|---------|--------|
| Horizontal scroll (deltaX) | Navigate days (no modifier needed) |
| Vertical scroll (deltaY) | Scroll entries within day column |
| Vertical scroll + Cmd/Ctrl | Navigate days |

## Direction Lock

Once a gesture direction is detected (first 1px of movement), all events in the other direction are ignored until the gesture ends (150ms timeout).

This prevents accidental diagonal gestures from triggering both scrolling and day navigation simultaneously.

## Implementation

Located in `crates/frontend/src/calendar-react/components/Calendar.tsx`:

```typescript
useEffect(() => {
    let accumulatedDeltaX = 0
    let accumulatedDeltaY = 0
    let lastWheelTime = 0
    let lockedDirection: "x" | "y" | null = null

    const TRACKPAD_THRESHOLD = 10      // Pixels per day for trackpad
    const MOUSE_WHEEL_THRESHOLD = 50   // Large deltas = mouse wheel
    const GESTURE_TIMEOUT = 50         // Reset direction lock after this gap
    const DIRECTION_LOCK_THRESHOLD = 1 // Pixels to determine initial direction

    // ... handler logic
}, [actions])
```

## Device Detection

- **Mouse wheel**: `|delta| >= 50` - discrete jumps, 1 event = 1 day navigation
- **Trackpad**: `|delta| < 50` - continuous values, accumulated with threshold

## Touch Gestures (Mobile)

Mobile uses separate touch event handlers with swipe detection:
- Horizontal swipe: Navigate days with snap animation
- Vertical scroll: Scroll entries (native behavior)

The touch handlers also implement direction lock using `isHorizontalSwipeRef`.
