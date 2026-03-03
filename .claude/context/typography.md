# Typography System

## Overview

The app uses a centralized typography system based on CSS custom properties defined in `:root`. The body font is a system sans-serif stack — no web fonts are loaded.

## Design Tokens

### Font Families

```css
--font-sans: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
--font-mono: ui-monospace, monospace;
```

- `body` uses `var(--font-sans)` — all elements inherit unless overridden
- Monospace is used only in the dev annotation overlay (`.annotation-info`, `.annotation-detail-path`)
- Dev overlay elements use `var(--font-sans)` / `var(--font-mono)` — not hardcoded stacks

### Font Size Scale

```css
--text-2xs: 0.5rem;    /*  8px — today badges, tiny labels */
--text-xs:  0.625rem;  /* 10px — badges, hour labels, entry time/location */
--text-sm:  0.75rem;   /* 12px — day names, labels, entry titles in schedule */
--text-base: 0.875rem; /* 14px — descriptions, buttons, form text */
--text-md:  1rem;      /* 16px — entry titles, form inputs */
--text-lg:  1.25rem;   /* 20px — modal title, close button */
--text-xl:  1.5rem;    /* 24px — day numbers, scroll indicator */
--text-2xl: 3rem;      /* 48px — empty/error state icons */
--text-3xl: 5rem;      /* 80px — hero day number */
```

### Font Weight

Use numeric values (`400`, `500`, `600`, `700`) — never the `bold` keyword.

## Line Height

**No global `line-height` on `body`.** The schedule grid has JS-calculated absolute-positioned entries that cannot absorb taller line boxes. The 70px fixed-height day headers are also sensitive.

Instead, `line-height: 1.5` is applied surgically to text-flow selectors where readability matters:
- `.entry-title`, `.entry-description`
- `.empty-day-text`, `.loading-day-text`, `.error-text`
- `.toast`, `.notification-empty p`, `.notification-item-date`
- `.form-group label`, `.form-error`
- `.settings-profile-name`, `.settings-profile-email`

Selectors that explicitly set a different `line-height` (and must not be changed):
- `.day-number` — `line-height: 1` (5rem decorative numeral)
- `.flash-message-text` — `line-height: 1.4` (constrained banner)
- `.flash-message-close` — `line-height: 1` (icon centering)
- `.now-time-label` — `line-height: 1` (absolute positioned, `translateY(-50%)`)

## Dev Overlay

Dev overlay and annotation sections use hardcoded `px` font sizes. These are intentionally **not** on the `--text-*` scale — they are a separate concern and should remain independent from app typography.

## Adding New Styles

When adding font styles to new selectors:
1. Use `var(--text-*)` for `font-size` — pick the closest step from the scale
2. Do **not** add `font-family` — it inherits from `body`
3. Add `line-height: 1.5` only for text-flow/prose content, not compact UI elements
4. Use numeric `font-weight` values
