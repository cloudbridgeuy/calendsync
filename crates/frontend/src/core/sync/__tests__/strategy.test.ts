import { describe, expect, test } from "bun:test"
import { decideSyncStrategy } from "../strategy"

describe("decideSyncStrategy", () => {
  test("returns 'use_local' when hasLocalData && hasSyncState", () => {
    const result = decideSyncStrategy(true, true, false)
    expect(result).toEqual({ type: "use_local" })
  })

  test("returns 'use_local' when hasLocalData && hasSyncState && hasSsrDays", () => {
    // use_local takes priority over hydrate_ssr
    const result = decideSyncStrategy(true, true, true)
    expect(result).toEqual({ type: "use_local" })
  })

  test("returns 'hydrate_ssr' when hasSsrDays and not use_local", () => {
    const result = decideSyncStrategy(false, false, true)
    expect(result).toEqual({ type: "hydrate_ssr" })
  })

  test("returns 'hydrate_ssr' when hasLocalData but no hasSyncState and hasSsrDays", () => {
    // Local data without sync state means we should use SSR if available
    const result = decideSyncStrategy(true, false, true)
    expect(result).toEqual({ type: "hydrate_ssr" })
  })

  test("returns 'hydrate_ssr' when hasSyncState but no hasLocalData and hasSsrDays", () => {
    // Sync state without local data means we should use SSR if available
    const result = decideSyncStrategy(false, true, true)
    expect(result).toEqual({ type: "hydrate_ssr" })
  })

  test("returns 'full_sync' when nothing else matches", () => {
    const result = decideSyncStrategy(false, false, false)
    expect(result).toEqual({ type: "full_sync" })
  })

  test("returns 'full_sync' when only hasLocalData (no sync state, no SSR)", () => {
    const result = decideSyncStrategy(true, false, false)
    expect(result).toEqual({ type: "full_sync" })
  })

  test("returns 'full_sync' when only hasSyncState (no local data, no SSR)", () => {
    const result = decideSyncStrategy(false, true, false)
    expect(result).toEqual({ type: "full_sync" })
  })

  // Exhaustive test of all boolean combinations
  describe("all boolean combinations", () => {
    const testCases: Array<{
      hasLocalData: boolean
      hasSyncState: boolean
      hasSsrDays: boolean
      expectedType: "use_local" | "hydrate_ssr" | "full_sync"
    }> = [
      // use_local: hasLocalData && hasSyncState (takes priority)
      { hasLocalData: true, hasSyncState: true, hasSsrDays: false, expectedType: "use_local" },
      { hasLocalData: true, hasSyncState: true, hasSsrDays: true, expectedType: "use_local" },
      // hydrate_ssr: hasSsrDays (when not use_local)
      { hasLocalData: false, hasSyncState: false, hasSsrDays: true, expectedType: "hydrate_ssr" },
      { hasLocalData: true, hasSyncState: false, hasSsrDays: true, expectedType: "hydrate_ssr" },
      { hasLocalData: false, hasSyncState: true, hasSsrDays: true, expectedType: "hydrate_ssr" },
      // full_sync: everything else
      { hasLocalData: false, hasSyncState: false, hasSsrDays: false, expectedType: "full_sync" },
      { hasLocalData: true, hasSyncState: false, hasSsrDays: false, expectedType: "full_sync" },
      { hasLocalData: false, hasSyncState: true, hasSsrDays: false, expectedType: "full_sync" },
    ]

    for (const { hasLocalData, hasSyncState, hasSsrDays, expectedType } of testCases) {
      test(`(${hasLocalData}, ${hasSyncState}, ${hasSsrDays}) => ${expectedType}`, () => {
        const result = decideSyncStrategy(hasLocalData, hasSyncState, hasSsrDays)
        expect(result.type).toBe(expectedType)
      })
    }
  })
})
