# Compound Pattern

The Compound Pattern creates components that work together through shared state via React Context. Components are "compound" because they're composed of multiple sub-components that only make sense together.

## When to Use

- Components with multiple related sub-components (dropdowns, menus, accordions, tabs)
- UI elements that share state internally but expose a simple API externally
- Building reusable component libraries

## Basic Pattern Structure

```typescript
// 1. Create context for shared state
const FlyOutContext = createContext<FlyOutContextValue | null>(null)

// 2. Parent component provides state
function FlyOut({ children }: { children: React.ReactNode }) {
    const [open, setOpen] = useState(false)

    return (
        <FlyOutContext.Provider value={{ open, toggle: () => setOpen(!open) }}>
            {children}
        </FlyOutContext.Provider>
    )
}

// 3. Child components consume state
function Toggle() {
    const ctx = useContext(FlyOutContext)
    if (!ctx) throw new Error("Toggle must be inside FlyOut")

    return <button onClick={ctx.toggle}>{ctx.open ? "Close" : "Open"}</button>
}

function List({ children }: { children: React.ReactNode }) {
    const ctx = useContext(FlyOutContext)
    if (!ctx) throw new Error("List must be inside FlyOut")

    return ctx.open ? <ul>{children}</ul> : null
}

function Item({ children }: { children: React.ReactNode }) {
    return <li>{children}</li>
}

// 4. Attach sub-components as properties
FlyOut.Toggle = Toggle
FlyOut.List = List
FlyOut.Item = Item
```

## Usage

```tsx
import { FlyOut } from "./FlyOut"

function Menu() {
    return (
        <FlyOut>
            <FlyOut.Toggle />
            <FlyOut.List>
                <FlyOut.Item>Edit</FlyOut.Item>
                <FlyOut.Item>Delete</FlyOut.Item>
            </FlyOut.List>
        </FlyOut>
    )
}
```

## Key Benefits

| Benefit | Description |
|---------|-------------|
| **Encapsulated state** | Parent manages state; children access via context |
| **Clean API** | Single import, discoverable sub-components |
| **Flexible composition** | Arrange sub-components freely within parent |
| **No prop drilling** | State flows through context, not props |

## Functional Core - Imperative Shell

Apply the FC-IS pattern to compound components by separating pure logic from side effects.

### Functional Core (Pure Functions)

Extract testable logic into pure functions:

```typescript
// core/flyout.ts - Pure functions, no React, no DOM

/** Determine effective open state from controlled/uncontrolled props */
export function resolveOpenState(
    controlledOpen: boolean | undefined,
    internalOpen: boolean
): { open: boolean; isControlled: boolean } {
    const isControlled = controlledOpen !== undefined
    return {
        open: isControlled ? controlledOpen : internalOpen,
        isControlled,
    }
}

/** Calculate next focused index for keyboard navigation */
export function getNextFocusIndex(
    currentIndex: number,
    itemCount: number,
    direction: 'up' | 'down' | 'home' | 'end'
): number {
    switch (direction) {
        case 'down':
            return (currentIndex + 1) % itemCount
        case 'up':
            return (currentIndex - 1 + itemCount) % itemCount
        case 'home':
            return 0
        case 'end':
            return itemCount - 1
    }
}

/** Build ARIA IDs from base ID */
export function buildAriaIds(baseId: string): { triggerId: string; contentId: string } {
    return {
        triggerId: `${baseId}-trigger`,
        contentId: `${baseId}-content`,
    }
}

/** Determine animation state transitions */
export function getNextAnimationState(
    currentState: AnimationState,
    action: 'open' | 'close' | 'animationEnd'
): AnimationState {
    switch (action) {
        case 'open':
            return 'opening'
        case 'close':
            return currentState === 'open' ? 'closing' : currentState
        case 'animationEnd':
            return currentState === 'opening' ? 'open' : 'closed'
    }
}
```

### Imperative Shell (React Components)

Components handle side effects and use pure functions:

```typescript
// components/FlyOut.tsx - React component with side effects

import { resolveOpenState, buildAriaIds, getNextFocusIndex } from '../core/flyout'

function FlyOut({ children, open: controlledOpen, defaultOpen = false }: FlyOutProps) {
    const [internalOpen, setInternalOpen] = useState(defaultOpen)

    // Use pure function to resolve state
    const { open, isControlled } = resolveOpenState(controlledOpen, internalOpen)

    // Use pure function to build IDs
    const id = useId()
    const { triggerId, contentId } = buildAriaIds(id)

    // Imperative: DOM side effects
    const lastFocusedRef = useRef<HTMLElement | null>(null)

    useEffect(() => {
        if (open) {
            // Side effect: save focus
            lastFocusedRef.current = document.activeElement as HTMLElement
        } else if (lastFocusedRef.current) {
            // Side effect: restore focus
            lastFocusedRef.current.focus()
        }
    }, [open])

    // ...
}

function List({ children }: { children: React.ReactNode }) {
    const { open, setOpen } = useFlyOut()
    const listRef = useRef<HTMLUListElement>(null)

    const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
        const items = listRef.current?.querySelectorAll<HTMLElement>('[role="menuitem"]')
        if (!items?.length) return

        const currentIndex = Array.from(items).findIndex(
            item => item === document.activeElement
        )

        // Use pure function for navigation logic
        let nextIndex: number | null = null
        switch (e.key) {
            case 'ArrowDown':
                nextIndex = getNextFocusIndex(currentIndex, items.length, 'down')
                break
            case 'ArrowUp':
                nextIndex = getNextFocusIndex(currentIndex, items.length, 'up')
                break
            case 'Home':
                nextIndex = getNextFocusIndex(currentIndex, items.length, 'home')
                break
            case 'End':
                nextIndex = getNextFocusIndex(currentIndex, items.length, 'end')
                break
            case 'Escape':
                setOpen(false)
                return
        }

        if (nextIndex !== null) {
            e.preventDefault()
            // Side effect: DOM focus
            items[nextIndex].focus()
        }
    }, [setOpen])

    // ...
}
```

### What Goes Where

| Functional Core | Imperative Shell |
|-----------------|------------------|
| State resolution (controlled vs uncontrolled) | `useState`, `useEffect` |
| ARIA ID generation | `useId()`, DOM attributes |
| Focus index calculation | `element.focus()`, `document.activeElement` |
| Animation state machine | `setTimeout`, `requestAnimationFrame` |
| Keyboard action mapping | Event listeners, `e.preventDefault()` |
| Validation logic | Error throwing, context checks |

### Benefits for Compound Components

1. **Testable**: Core logic tested without React/DOM
2. **Reusable**: Same logic works across different UI frameworks
3. **Debuggable**: Pure functions easier to reason about
4. **Type-safe**: TypeScript catches errors in pure functions

### File Structure

```
src/
├── core/
│   └── flyout/
│       ├── state.ts        # resolveOpenState, animation state
│       ├── navigation.ts   # getNextFocusIndex
│       ├── aria.ts         # buildAriaIds
│       └── index.ts        # re-exports
└── components/
    └── FlyOut/
        ├── FlyOut.tsx      # Main component (shell)
        ├── Toggle.tsx      # Sub-component (shell)
        ├── List.tsx        # Sub-component (shell)
        ├── Item.tsx        # Sub-component (shell)
        └── index.ts        # Composed export
```

### Testing Pure Functions

Tests for the Functional Core require no React, no DOM, no mocking:

```typescript
// __tests__/state.test.ts
import { describe, expect, test } from "bun:test"
import { resolveOpenState, getNextAnimationState } from "../state"

describe("resolveOpenState", () => {
    test("returns controlled state when provided", () => {
        const result = resolveOpenState(true, false)
        expect(result.open).toBe(true)
        expect(result.isControlled).toBe(true)
    })

    test("returns internal state when controlled is undefined", () => {
        const result = resolveOpenState(undefined, true)
        expect(result.open).toBe(true)
        expect(result.isControlled).toBe(false)
    })

    test("controlled false overrides internal true", () => {
        const result = resolveOpenState(false, true)
        expect(result.open).toBe(false)
        expect(result.isControlled).toBe(true)
    })
})

describe("getNextAnimationState", () => {
    test("open action transitions to opening", () => {
        expect(getNextAnimationState("closed", "open")).toBe("opening")
    })

    test("close action from open transitions to closing", () => {
        expect(getNextAnimationState("open", "close")).toBe("closing")
    })

    test("close action from closed stays closed", () => {
        expect(getNextAnimationState("closed", "close")).toBe("closed")
    })

    test("animationEnd from opening transitions to open", () => {
        expect(getNextAnimationState("opening", "animationEnd")).toBe("open")
    })

    test("animationEnd from closing transitions to closed", () => {
        expect(getNextAnimationState("closing", "animationEnd")).toBe("closed")
    })
})
```

```typescript
// __tests__/navigation.test.ts
import { describe, expect, test } from "bun:test"
import { getNextFocusIndex } from "../navigation"

describe("getNextFocusIndex", () => {
    test("down wraps from last to first", () => {
        expect(getNextFocusIndex(4, 5, "down")).toBe(0)
    })

    test("down increments normally", () => {
        expect(getNextFocusIndex(2, 5, "down")).toBe(3)
    })

    test("up wraps from first to last", () => {
        expect(getNextFocusIndex(0, 5, "up")).toBe(4)
    })

    test("up decrements normally", () => {
        expect(getNextFocusIndex(2, 5, "up")).toBe(1)
    })

    test("home returns 0", () => {
        expect(getNextFocusIndex(3, 5, "home")).toBe(0)
    })

    test("end returns last index", () => {
        expect(getNextFocusIndex(1, 5, "end")).toBe(4)
    })
})
```

```typescript
// __tests__/aria.test.ts
import { describe, expect, test } from "bun:test"
import { buildAriaIds } from "../aria"

describe("buildAriaIds", () => {
    test("builds trigger and content IDs from base", () => {
        const ids = buildAriaIds("menu-1")
        expect(ids.triggerId).toBe("menu-1-trigger")
        expect(ids.contentId).toBe("menu-1-content")
    })

    test("handles empty base ID", () => {
        const ids = buildAriaIds("")
        expect(ids.triggerId).toBe("-trigger")
        expect(ids.contentId).toBe("-content")
    })
})
```

**Key testing principles:**

| Principle | Example |
|-----------|---------|
| No DOM needed | `getNextFocusIndex` tests index math, not `element.focus()` |
| No React needed | Pure functions don't use hooks |
| No mocking needed | No external dependencies to mock |
| Fast execution | Runs in milliseconds, no setup/teardown |
| Easy to understand | Input → output, no side effects |

## Controlled vs Uncontrolled Mode

The basic pattern traps state inside the component. For parent components to read or control state, support both modes:

```typescript
interface FlyOutProps {
    children: React.ReactNode
    // Controlled mode
    open?: boolean
    onOpenChange?: (open: boolean) => void
    // Uncontrolled mode
    defaultOpen?: boolean
}

function FlyOut({
    children,
    open: controlledOpen,
    onOpenChange,
    defaultOpen = false
}: FlyOutProps) {
    const [internalOpen, setInternalOpen] = useState(defaultOpen)

    // Use controlled value if provided, otherwise internal
    const isControlled = controlledOpen !== undefined
    const open = isControlled ? controlledOpen : internalOpen

    const setOpen = useCallback((nextOpen: boolean) => {
        if (isControlled) {
            onOpenChange?.(nextOpen)
        } else {
            setInternalOpen(nextOpen)
        }
    }, [isControlled, onOpenChange])

    const toggle = useCallback(() => setOpen(!open), [open, setOpen])

    const value = useMemo(() => ({ open, toggle, setOpen }), [open, toggle, setOpen])

    return (
        <FlyOutContext.Provider value={value}>
            {children}
        </FlyOutContext.Provider>
    )
}
```

### Usage Modes

```tsx
// Uncontrolled (internal state)
<FlyOut defaultOpen={false}>
    <FlyOut.Toggle />
    <FlyOut.List>...</FlyOut.List>
</FlyOut>

// Controlled (parent owns state)
const [isOpen, setIsOpen] = useState(false)

<FlyOut open={isOpen} onOpenChange={setIsOpen}>
    <FlyOut.Toggle />
    <FlyOut.List>...</FlyOut.List>
</FlyOut>

// Can now read/control from parent
<button onClick={() => setIsOpen(true)}>Open Menu</button>
{isOpen && <p>Menu is open!</p>}
```

## Event Callbacks

Even in uncontrolled mode, parents often need to react to state changes:

```typescript
interface FlyOutProps {
    children: React.ReactNode
    defaultOpen?: boolean
    onOpen?: () => void
    onClose?: () => void
}

function FlyOut({ children, defaultOpen = false, onOpen, onClose }: FlyOutProps) {
    const [open, setOpen] = useState(defaultOpen)

    const handleSetOpen = useCallback((nextOpen: boolean) => {
        setOpen(nextOpen)
        if (nextOpen) {
            onOpen?.()
        } else {
            onClose?.()
        }
    }, [onOpen, onClose])

    // ...
}
```

## Imperative Handle (Ref-Based Control)

For triggering actions from parent without full controlled mode:

```typescript
interface FlyOutHandle {
    open: () => void
    close: () => void
    toggle: () => void
    isOpen: () => boolean
}

const FlyOut = forwardRef<FlyOutHandle, FlyOutProps>((props, ref) => {
    const [open, setOpen] = useState(props.defaultOpen ?? false)

    useImperativeHandle(ref, () => ({
        open: () => setOpen(true),
        close: () => setOpen(false),
        toggle: () => setOpen(prev => !prev),
        isOpen: () => open,
    }), [open])

    // ...
})

// Usage
function Parent() {
    const flyoutRef = useRef<FlyOutHandle>(null)

    return (
        <>
            <button onClick={() => flyoutRef.current?.open()}>
                Open from parent
            </button>
            <FlyOut ref={flyoutRef}>
                <FlyOut.Toggle />
                <FlyOut.List>...</FlyOut.List>
            </FlyOut>
        </>
    )
}
```

## Accessibility

Compound components must coordinate ARIA attributes across sub-components:

```typescript
interface FlyOutContextValue {
    open: boolean
    toggle: () => void
    // Shared IDs for ARIA relationships
    triggerId: string
    contentId: string
}

function FlyOut({ children }: FlyOutProps) {
    const [open, setOpen] = useState(false)
    const id = useId()
    const triggerId = `${id}-trigger`
    const contentId = `${id}-content`

    const value = useMemo(() => ({
        open,
        toggle: () => setOpen(!open),
        triggerId,
        contentId,
    }), [open, triggerId, contentId])

    return (
        <FlyOutContext.Provider value={value}>
            {children}
        </FlyOutContext.Provider>
    )
}

function Toggle({ children }: { children: React.ReactNode }) {
    const { open, toggle, triggerId, contentId } = useFlyOut()

    return (
        <button
            id={triggerId}
            onClick={toggle}
            aria-expanded={open}
            aria-controls={contentId}
            aria-haspopup="menu"
        >
            {children}
        </button>
    )
}

function List({ children }: { children: React.ReactNode }) {
    const { open, contentId, triggerId } = useFlyOut()

    if (!open) return null

    return (
        <ul
            id={contentId}
            role="menu"
            aria-labelledby={triggerId}
        >
            {children}
        </ul>
    )
}

function Item({ children, onSelect }: { children: React.ReactNode; onSelect?: () => void }) {
    return (
        <li role="menuitem" onClick={onSelect}>
            {children}
        </li>
    )
}
```

## Focus Management

Trap focus inside when open, restore on close:

```typescript
function FlyOut({ children, ...props }: FlyOutProps) {
    const [open, setOpen] = useState(false)
    const triggerRef = useRef<HTMLElement>(null)
    const contentRef = useRef<HTMLElement>(null)

    // Store trigger element for focus restoration
    const lastFocusedRef = useRef<HTMLElement | null>(null)

    useEffect(() => {
        if (open) {
            // Save current focus
            lastFocusedRef.current = document.activeElement as HTMLElement
            // Focus first focusable element in content
            contentRef.current?.querySelector<HTMLElement>(
                'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
            )?.focus()
        } else if (lastFocusedRef.current) {
            // Restore focus on close
            lastFocusedRef.current.focus()
            lastFocusedRef.current = null
        }
    }, [open])

    // Context includes refs for sub-components
    const value = useMemo(() => ({
        open,
        toggle: () => setOpen(!open),
        triggerRef,
        contentRef,
    }), [open])

    return (
        <FlyOutContext.Provider value={value}>
            {children}
        </FlyOutContext.Provider>
    )
}
```

## Keyboard Navigation

Support standard keyboard patterns:

```typescript
function List({ children }: { children: React.ReactNode }) {
    const { open, contentId, setOpen } = useFlyOut()
    const listRef = useRef<HTMLUListElement>(null)

    const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
        const items = listRef.current?.querySelectorAll<HTMLElement>('[role="menuitem"]')
        if (!items?.length) return

        const currentIndex = Array.from(items).findIndex(
            item => item === document.activeElement
        )

        switch (e.key) {
            case 'ArrowDown':
                e.preventDefault()
                items[(currentIndex + 1) % items.length].focus()
                break
            case 'ArrowUp':
                e.preventDefault()
                items[(currentIndex - 1 + items.length) % items.length].focus()
                break
            case 'Home':
                e.preventDefault()
                items[0].focus()
                break
            case 'End':
                e.preventDefault()
                items[items.length - 1].focus()
                break
            case 'Escape':
                e.preventDefault()
                setOpen(false)
                break
        }
    }, [setOpen])

    if (!open) return null

    return (
        <ul
            ref={listRef}
            id={contentId}
            role="menu"
            onKeyDown={handleKeyDown}
        >
            {children}
        </ul>
    )
}
```

## Portal Rendering

Dropdowns often need to escape overflow containers:

```typescript
function List({ children }: { children: React.ReactNode }) {
    const { open, contentId } = useFlyOut()

    if (!open) return null

    return createPortal(
        <ul id={contentId} role="menu" className="flyout-list">
            {children}
        </ul>,
        document.body
    )
}
```

**Note:** Portal rendering requires additional positioning logic (e.g., Floating UI) to position the menu relative to the trigger.

## Animation States

For enter/exit animations, track transition state:

```typescript
type AnimationState = 'closed' | 'opening' | 'open' | 'closing'

function FlyOut({ children }: FlyOutProps) {
    const [open, setOpen] = useState(false)
    const [animationState, setAnimationState] = useState<AnimationState>('closed')

    const handleOpen = useCallback(() => {
        setOpen(true)
        setAnimationState('opening')
        // Transition to 'open' after animation
        requestAnimationFrame(() => {
            requestAnimationFrame(() => setAnimationState('open'))
        })
    }, [])

    const handleClose = useCallback(() => {
        setAnimationState('closing')
        // Wait for animation to complete before unmounting
        setTimeout(() => {
            setOpen(false)
            setAnimationState('closed')
        }, 200) // Match CSS transition duration
    }, [])

    const value = useMemo(() => ({
        open,
        animationState,
        toggle: () => open ? handleClose() : handleOpen(),
    }), [open, animationState, handleOpen, handleClose])

    return (
        <FlyOutContext.Provider value={value}>
            {children}
        </FlyOutContext.Provider>
    )
}

function List({ children }: { children: React.ReactNode }) {
    const { open, animationState, contentId } = useFlyOut()

    // Render during opening/closing for animation
    if (!open && animationState === 'closed') return null

    return (
        <ul
            id={contentId}
            role="menu"
            className={`flyout-list flyout-list--${animationState}`}
        >
            {children}
        </ul>
    )
}
```

```css
.flyout-list {
    opacity: 0;
    transform: translateY(-8px);
    transition: opacity 200ms, transform 200ms;
}

.flyout-list--open {
    opacity: 1;
    transform: translateY(0);
}

.flyout-list--closing {
    opacity: 0;
    transform: translateY(-8px);
}
```

## Implementation Approaches

### Context API (Recommended)

Uses `createContext` and `useContext`. Allows arbitrary nesting depth.

```typescript
const TabsContext = createContext<TabsContextValue | null>(null)

function useTabs() {
    const ctx = useContext(TabsContext)
    if (!ctx) throw new Error("useTabs must be used within Tabs")
    return ctx
}
```

### React.Children.map (Limited)

Clones children with additional props. Only works with direct children.

```typescript
function FlyOut({ children }: { children: React.ReactNode }) {
    const [open, setOpen] = useState(false)

    return (
        <div>
            {React.Children.map(children, child =>
                React.cloneElement(child as React.ReactElement, {
                    open,
                    toggle: () => setOpen(!open)
                })
            )}
        </div>
    )
}
```

**Limitation:** Wrapping children breaks prop injection:

```tsx
// This breaks with React.Children.map
<FlyOut>
    <div>
        <FlyOut.Toggle /> {/* Won't receive props */}
    </div>
</FlyOut>
```

## TypeScript Types

```typescript
interface FlyOutContextValue {
    open: boolean
    toggle: () => void
    setOpen: (open: boolean) => void
    triggerId: string
    contentId: string
}

interface FlyOutProps {
    children: React.ReactNode
    // Controlled
    open?: boolean
    onOpenChange?: (open: boolean) => void
    // Uncontrolled
    defaultOpen?: boolean
    // Callbacks
    onOpen?: () => void
    onClose?: () => void
}

interface FlyOutComponent extends React.FC<FlyOutProps> {
    Toggle: React.FC<{ children?: React.ReactNode }>
    List: React.FC<{ children: React.ReactNode }>
    Item: React.FC<{ children: React.ReactNode; onSelect?: () => void }>
}

const FlyOut: FlyOutComponent = ({ children, ...props }) => {
    // implementation
}
```

## Complete Example

```typescript
import {
    createContext,
    useContext,
    useState,
    useCallback,
    useMemo,
    useId,
    useEffect,
    useRef,
    forwardRef,
    useImperativeHandle,
} from 'react'

// Types
interface FlyOutContextValue {
    open: boolean
    toggle: () => void
    setOpen: (open: boolean) => void
    triggerId: string
    contentId: string
}

interface FlyOutProps {
    children: React.ReactNode
    open?: boolean
    onOpenChange?: (open: boolean) => void
    defaultOpen?: boolean
    onOpen?: () => void
    onClose?: () => void
}

interface FlyOutHandle {
    open: () => void
    close: () => void
    toggle: () => void
    isOpen: () => boolean
}

// Context
const FlyOutContext = createContext<FlyOutContextValue | null>(null)

function useFlyOut() {
    const ctx = useContext(FlyOutContext)
    if (!ctx) throw new Error('Component must be used within FlyOut')
    return ctx
}

// Main component
const FlyOutRoot = forwardRef<FlyOutHandle, FlyOutProps>((props, ref) => {
    const {
        children,
        open: controlledOpen,
        onOpenChange,
        defaultOpen = false,
        onOpen,
        onClose,
    } = props

    const [internalOpen, setInternalOpen] = useState(defaultOpen)
    const isControlled = controlledOpen !== undefined
    const open = isControlled ? controlledOpen : internalOpen

    const id = useId()
    const triggerId = `flyout-${id}-trigger`
    const contentId = `flyout-${id}-content`

    const setOpen = useCallback((nextOpen: boolean) => {
        if (isControlled) {
            onOpenChange?.(nextOpen)
        } else {
            setInternalOpen(nextOpen)
        }
        if (nextOpen) onOpen?.()
        else onClose?.()
    }, [isControlled, onOpenChange, onOpen, onClose])

    const toggle = useCallback(() => setOpen(!open), [open, setOpen])

    useImperativeHandle(ref, () => ({
        open: () => setOpen(true),
        close: () => setOpen(false),
        toggle,
        isOpen: () => open,
    }), [open, setOpen, toggle])

    const value = useMemo(() => ({
        open,
        toggle,
        setOpen,
        triggerId,
        contentId,
    }), [open, toggle, setOpen, triggerId, contentId])

    return (
        <FlyOutContext.Provider value={value}>
            <div className="flyout">{children}</div>
        </FlyOutContext.Provider>
    )
})

// Sub-components
function Toggle({ children }: { children?: React.ReactNode }) {
    const { open, toggle, triggerId, contentId } = useFlyOut()

    return (
        <button
            id={triggerId}
            type="button"
            onClick={toggle}
            aria-expanded={open}
            aria-controls={contentId}
            aria-haspopup="menu"
            className="flyout-toggle"
        >
            {children ?? (open ? 'Close' : 'Open')}
        </button>
    )
}

function List({ children }: { children: React.ReactNode }) {
    const { open, contentId, triggerId, setOpen } = useFlyOut()
    const listRef = useRef<HTMLUListElement>(null)

    // Close on Escape
    useEffect(() => {
        if (!open) return
        const handleEscape = (e: KeyboardEvent) => {
            if (e.key === 'Escape') setOpen(false)
        }
        document.addEventListener('keydown', handleEscape)
        return () => document.removeEventListener('keydown', handleEscape)
    }, [open, setOpen])

    if (!open) return null

    return (
        <ul
            ref={listRef}
            id={contentId}
            role="menu"
            aria-labelledby={triggerId}
            className="flyout-list"
        >
            {children}
        </ul>
    )
}

function Item({
    children,
    onSelect
}: {
    children: React.ReactNode
    onSelect?: () => void
}) {
    const { setOpen } = useFlyOut()

    const handleClick = useCallback(() => {
        onSelect?.()
        setOpen(false)
    }, [onSelect, setOpen])

    return (
        <li
            role="menuitem"
            tabIndex={0}
            onClick={handleClick}
            onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault()
                    handleClick()
                }
            }}
            className="flyout-item"
        >
            {children}
        </li>
    )
}

// Compose final component
const FlyOut = Object.assign(FlyOutRoot, {
    Toggle,
    List,
    Item,
})

export { FlyOut, useFlyOut }
export type { FlyOutProps, FlyOutHandle }
```

## Common Pitfalls

| Pitfall | Solution |
|---------|----------|
| Missing context check | Always throw if context is null |
| Re-renders on every render | Memoize context value with `useMemo` |
| Controlled/uncontrolled mixing | Pick one mode or implement both properly |
| Focus not restored on close | Track and restore `document.activeElement` |
| No keyboard support | Implement Arrow, Escape, Enter handlers |
| ARIA attributes missing | Coordinate IDs between trigger and content |

## Anti-Patterns to Avoid

### Boolean Checks at Consumer Level

**Anti-pattern:**
```tsx
{items.length === 0 ? (
    <Menu.EmptyState />
) : (
    items.map(item => <Menu.Item key={item.id} item={item} />)
)}
```

**Why it's bad:** Pollutes consumer with logic that belongs inside the component. Every consumer must duplicate this conditional logic.

**Better pattern:** Components handle their own visibility:
```tsx
// EmptyState checks internally and returns null if items exist
function EmptyState() {
    const { items } = useMenuContext()
    if (items.length > 0) return null
    return <div>No items</div>
}

// Consumer is clean and declarative
<Menu.EmptyState />
```

### Iteration at Consumer Level

**Anti-pattern:**
```tsx
<Menu.List>
    {items.map(item => (
        <Menu.Item key={item.id} item={item} />
    ))}
</Menu.List>
```

**Why it's bad:** Filtering, sorting, and transformation logic ends up scattered across consumers. Changes to iteration logic require updating every usage site.

**Better pattern:** Create an Items component that handles iteration internally:
```tsx
function Items() {
    const { items } = useMenuContext()
    if (items.length === 0) return <EmptyState />
    return (
        <>
            {items.map(item => (
                <Item key={item.id} item={item} />
            ))}
        </>
    )
}

// Consumer is clean and declarative
<Menu.Items />
```

**Principle:** Compound components should encapsulate their logic. Consumers should compose declaratively, not imperatively.

### Wrapper Elements at Consumer Level

**Anti-pattern:**
```tsx
<Menu>
    <div className="menu-header-wrapper">
        <Menu.Header />
    </div>
    <main className="menu-content">
        <Menu.Items />
    </main>
</Menu>
```

**Why it's bad:** Wrapper elements like `<div>` and `<main>` are implementation details. Placing them at the consumer level:
- Duplicates markup across every usage site
- Requires consumers to know internal class names
- Makes refactoring the component's DOM structure harder

**Better pattern:** Sub-components render their own wrappers:
```tsx
function Header() {
    const ctx = useMenuContext()
    return (
        <div className="menu-header-wrapper">
            {/* header content */}
        </div>
    )
}

function Items() {
    const { items } = useMenuContext()
    return (
        <main className="menu-content">
            {/* items rendering */}
        </main>
    )
}

// Consumer is clean and declarative
<Menu>
    <Menu.Header />
    <Menu.Items />
</Menu>
```

**Principle:** Sub-components own their complete DOM structure, including wrapper elements.

## References

- [React Patterns: Compound Components](https://www.patterns.dev/react/compound-pattern)
- [Kent C. Dodds: Advanced React Patterns](https://kentcdodds.com/blog/compound-components-with-react-hooks)
- [Radix UI Primitives](https://www.radix-ui.com/primitives) - Production compound components
- [Reach UI](https://reach.tech/) - Accessible compound components
