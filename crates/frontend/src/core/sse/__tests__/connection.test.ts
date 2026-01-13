import { describe, expect, test } from "bun:test"
import {
  buildSseUrl,
  calculateReconnectDelay,
  deriveEntryTypeFromFlags,
  MAX_RECONNECT_ATTEMPTS,
  parseEventData,
  RECONNECT_DELAY_MS,
  shouldReconnect,
} from "../connection"

describe("calculateReconnectDelay", () => {
  test("returns base delay for first attempt (attempts = 0)", () => {
    expect(calculateReconnectDelay(0)).toBe(RECONNECT_DELAY_MS)
  })

  test("doubles delay for each attempt", () => {
    expect(calculateReconnectDelay(1)).toBe(RECONNECT_DELAY_MS * 2)
    expect(calculateReconnectDelay(2)).toBe(RECONNECT_DELAY_MS * 4)
    expect(calculateReconnectDelay(3)).toBe(RECONNECT_DELAY_MS * 8)
  })

  test("caps exponent at maxExponent", () => {
    // Default maxExponent is 4, so max multiplier is 2^4 = 16
    expect(calculateReconnectDelay(4)).toBe(RECONNECT_DELAY_MS * 16)
    expect(calculateReconnectDelay(5)).toBe(RECONNECT_DELAY_MS * 16)
    expect(calculateReconnectDelay(100)).toBe(RECONNECT_DELAY_MS * 16)
  })

  test("uses custom base delay", () => {
    expect(calculateReconnectDelay(0, 1000)).toBe(1000)
    expect(calculateReconnectDelay(1, 1000)).toBe(2000)
  })

  test("uses custom max exponent", () => {
    expect(calculateReconnectDelay(3, RECONNECT_DELAY_MS, 2)).toBe(RECONNECT_DELAY_MS * 4)
  })
})

describe("shouldReconnect", () => {
  test("returns true when attempts < maxAttempts", () => {
    expect(shouldReconnect(0)).toBe(true)
    expect(shouldReconnect(1)).toBe(true)
    expect(shouldReconnect(MAX_RECONNECT_ATTEMPTS - 1)).toBe(true)
  })

  test("returns false when attempts >= maxAttempts", () => {
    expect(shouldReconnect(MAX_RECONNECT_ATTEMPTS)).toBe(false)
    expect(shouldReconnect(MAX_RECONNECT_ATTEMPTS + 1)).toBe(false)
  })

  test("uses custom maxAttempts", () => {
    expect(shouldReconnect(2, 3)).toBe(true)
    expect(shouldReconnect(3, 3)).toBe(false)
  })
})

describe("parseEventData", () => {
  test("parses valid JSON", () => {
    const result = parseEventData<{ foo: string }>('{"foo": "bar"}')
    expect(result).toEqual({ foo: "bar" })
  })

  test("returns null for invalid JSON", () => {
    expect(parseEventData("not json")).toBe(null)
    expect(parseEventData("{invalid}")).toBe(null)
    expect(parseEventData("")).toBe(null)
  })

  test("parses nested objects", () => {
    const input = '{"entry": {"id": "123", "title": "Test"}}'
    const result = parseEventData<{ entry: { id: string; title: string } }>(input)
    expect(result).toEqual({ entry: { id: "123", title: "Test" } })
  })

  test("parses arrays", () => {
    const result = parseEventData<number[]>("[1, 2, 3]")
    expect(result).toEqual([1, 2, 3])
  })
})

describe("buildSseUrl", () => {
  test("builds URL with calendar ID", () => {
    const url = buildSseUrl("http://localhost:3000", "cal-123")
    expect(url).toBe("http://localhost:3000/api/events?calendar_id=cal-123")
  })

  test("includes last event ID when provided", () => {
    const url = buildSseUrl("http://localhost:3000", "cal-123", "evt-456")
    expect(url).toBe("http://localhost:3000/api/events?calendar_id=cal-123&last_event_id=evt-456")
  })

  test("handles null last event ID", () => {
    const url = buildSseUrl("http://localhost:3000", "cal-123", null)
    expect(url).toBe("http://localhost:3000/api/events?calendar_id=cal-123")
  })

  test("encodes special characters in calendar ID", () => {
    const url = buildSseUrl("http://localhost:3000", "cal&id=123")
    expect(url).toBe("http://localhost:3000/api/events?calendar_id=cal%26id%3D123")
  })

  test("encodes special characters in last event ID", () => {
    const url = buildSseUrl("http://localhost:3000", "cal-123", "evt&id=456")
    expect(url).toBe(
      "http://localhost:3000/api/events?calendar_id=cal-123&last_event_id=evt%26id%3D456",
    )
  })
})

describe("deriveEntryTypeFromFlags", () => {
  test("returns 'timed' when isTimed is true", () => {
    expect(deriveEntryTypeFromFlags({ isTimed: true })).toBe("timed")
  })

  test("returns 'task' when isTask is true", () => {
    expect(deriveEntryTypeFromFlags({ isTask: true })).toBe("task")
  })

  test("returns 'multi_day' when isMultiDay is true", () => {
    expect(deriveEntryTypeFromFlags({ isMultiDay: true })).toBe("multi_day")
  })

  test("returns 'all_day' when no flags are true", () => {
    expect(deriveEntryTypeFromFlags({})).toBe("all_day")
    expect(deriveEntryTypeFromFlags({ isTimed: false, isTask: false, isMultiDay: false })).toBe(
      "all_day",
    )
  })

  test("prioritizes isTimed over other flags", () => {
    expect(deriveEntryTypeFromFlags({ isTimed: true, isTask: true })).toBe("timed")
    expect(deriveEntryTypeFromFlags({ isTimed: true, isMultiDay: true })).toBe("timed")
  })

  test("prioritizes isTask over isMultiDay", () => {
    expect(deriveEntryTypeFromFlags({ isTask: true, isMultiDay: true })).toBe("task")
  })
})
