// crates/frontend/src/core/calendar/__tests__/aria.test.ts
import { describe, expect, test } from "bun:test"
import { buildAriaIds } from "../aria"

describe("buildAriaIds", () => {
  test("builds trigger and content IDs from base", () => {
    const ids = buildAriaIds("notification-center")
    expect(ids.triggerId).toBe("notification-center-trigger")
    expect(ids.contentId).toBe("notification-center-content")
  })

  test("handles hyphenated base IDs", () => {
    const ids = buildAriaIds("my-component-123")
    expect(ids.triggerId).toBe("my-component-123-trigger")
    expect(ids.contentId).toBe("my-component-123-content")
  })
})
