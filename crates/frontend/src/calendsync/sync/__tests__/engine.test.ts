/**
 * Unit tests for SyncEngine transport initialization.
 *
 * Tests the pure logic of transport identity checking using the
 * transportInitialized flag instead of object comparison.
 */

import { describe, expect, it, mock } from "bun:test"

import type { ServerEntry } from "@core/calendar/types"
import type { Transport } from "@core/transport/types"

import type { CalendSyncDatabase } from "../../db"
import { SyncEngine } from "../engine"

/** Mock Dexie database for testing */
function createMockDb(): CalendSyncDatabase {
  return {
    entries: {
      update: mock(),
      delete: mock(),
      get: mock(),
      put: mock(),
    },
    pending_operations: {
      add: mock(),
      delete: mock(),
      put: mock(),
      count: mock(() => Promise.resolve(0)),
      toArray: mock(() => Promise.resolve([])),
    },
  } as unknown as CalendSyncDatabase
}

/** Mock Transport for testing */
function createMockTransport(): Transport {
  return {
    createEntry: mock(() => Promise.resolve({} as ServerEntry)),
    updateEntry: mock(() => Promise.resolve({} as ServerEntry)),
    deleteEntry: mock(() => Promise.resolve(undefined)),
    // Other methods not used by SyncEngine - provide stubs
    exchangeAuthCode: mock(() => Promise.resolve("")),
    validateSession: mock(() => Promise.resolve(false)),
    logout: mock(() => Promise.resolve()),
    fetchMyCalendars: mock(() => Promise.resolve([])),
    fetchEntries: mock(() => Promise.resolve([])),
    toggleEntry: mock(() => Promise.resolve({} as ServerEntry)),
    fetchEntry: mock(() => Promise.resolve({} as ServerEntry)),
    getSession: mock(() => Promise.resolve(null)),
    setSession: mock(() => Promise.resolve()),
    clearSession: mock(() => Promise.resolve()),
    getLastCalendar: mock(() => Promise.resolve(null)),
    setLastCalendar: mock(() => Promise.resolve()),
    clearLastCalendar: mock(() => Promise.resolve()),
  }
}

describe("SyncEngine transport initialization", () => {
  it("initializes transport on first call", () => {
    const db = createMockDb()
    const engine = new SyncEngine(db)
    const transport = createMockTransport()

    expect(engine.hasTransport()).toBe(false)

    engine.initTransport(transport)

    expect(engine.hasTransport()).toBe(true)
  })

  it("skips initialization on subsequent calls with same transport", () => {
    const db = createMockDb()
    const engine = new SyncEngine(db)
    const transport = createMockTransport()

    engine.initTransport(transport)
    expect(engine.hasTransport()).toBe(true)

    // Second call with same transport should be a no-op
    engine.initTransport(transport)
    expect(engine.hasTransport()).toBe(true)
  })

  it("skips initialization on subsequent calls with different transport", () => {
    const db = createMockDb()
    const engine = new SyncEngine(db)
    const transport1 = createMockTransport()
    const transport2 = createMockTransport()

    engine.initTransport(transport1)
    expect(engine.hasTransport()).toBe(true)

    // Second call with different transport should still be a no-op
    // because transportInitialized flag is true
    engine.initTransport(transport2)
    expect(engine.hasTransport()).toBe(true)
  })

  it("allows re-initialization after reset (testing only)", () => {
    const db = createMockDb()
    const engine = new SyncEngine(db)
    const transport1 = createMockTransport()
    const transport2 = createMockTransport()

    // Initialize with first transport
    engine.initTransport(transport1)
    expect(engine.hasTransport()).toBe(true)

    // Reset for testing
    engine.resetTransport()
    expect(engine.hasTransport()).toBe(false)

    // Re-initialize with second transport
    engine.initTransport(transport2)
    expect(engine.hasTransport()).toBe(true)
  })

  it("resets to default API client after reset", () => {
    const db = createMockDb()
    const engine = new SyncEngine(db)
    const transport = createMockTransport()

    // Initialize with transport
    engine.initTransport(transport)
    expect(engine.hasTransport()).toBe(true)

    // Reset should restore default API client
    engine.resetTransport()
    expect(engine.hasTransport()).toBe(false)

    // Engine should still function with default fetch-based API
    // (this is tested indirectly - the api field is private)
  })

  it("hasTransport returns false before initialization", () => {
    const db = createMockDb()
    const engine = new SyncEngine(db)

    expect(engine.hasTransport()).toBe(false)
  })

  it("hasTransport returns true after initialization", () => {
    const db = createMockDb()
    const engine = new SyncEngine(db)
    const transport = createMockTransport()

    engine.initTransport(transport)

    expect(engine.hasTransport()).toBe(true)
  })

  it("hasTransport uses flag instead of null check", () => {
    const db = createMockDb()
    const engine = new SyncEngine(db)
    const transport = createMockTransport()

    // Before init: flag is false, transport is null
    expect(engine.hasTransport()).toBe(false)

    // After init: flag is true, transport is set
    engine.initTransport(transport)
    expect(engine.hasTransport()).toBe(true)

    // After reset: flag is false, transport is null again
    engine.resetTransport()
    expect(engine.hasTransport()).toBe(false)
  })
})
