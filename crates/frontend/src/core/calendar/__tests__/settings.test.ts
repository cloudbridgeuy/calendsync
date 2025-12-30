import { describe, expect, test } from "bun:test"
import {
  DEFAULT_SETTINGS,
  getSettingsStorageKey,
  parseSettingsJson,
  serializeSettings,
  toggleShowTasks,
  updateEntryStyle,
  updateShowTasks,
  updateViewMode,
} from "../settings"

describe("getSettingsStorageKey", () => {
  test("creates key with calendar ID", () => {
    expect(getSettingsStorageKey("abc123")).toBe("calendsync_settings_abc123")
  })

  test("handles empty calendar ID", () => {
    expect(getSettingsStorageKey("")).toBe("calendsync_settings_")
  })
})

describe("parseSettingsJson", () => {
  test("returns defaults for null input", () => {
    const result = parseSettingsJson(null)
    expect(result).toEqual(DEFAULT_SETTINGS)
  })

  test("returns defaults for empty string", () => {
    const result = parseSettingsJson("")
    expect(result).toEqual(DEFAULT_SETTINGS)
  })

  test("returns defaults for invalid JSON", () => {
    const result = parseSettingsJson("not valid json")
    expect(result).toEqual(DEFAULT_SETTINGS)
  })

  test("parses valid settings", () => {
    const json = JSON.stringify({
      viewMode: "schedule",
      showTasks: false,
      entryStyle: "filled",
    })
    const result = parseSettingsJson(json)
    expect(result.viewMode).toBe("schedule")
    expect(result.showTasks).toBe(false)
    expect(result.entryStyle).toBe("filled")
  })

  test("uses defaults for missing fields", () => {
    const json = JSON.stringify({ viewMode: "schedule" })
    const result = parseSettingsJson(json)
    expect(result.viewMode).toBe("schedule")
    expect(result.showTasks).toBe(DEFAULT_SETTINGS.showTasks)
    expect(result.entryStyle).toBe(DEFAULT_SETTINGS.entryStyle)
  })

  test("uses defaults for invalid viewMode", () => {
    const json = JSON.stringify({ viewMode: "invalid", showTasks: false })
    const result = parseSettingsJson(json)
    expect(result.viewMode).toBe(DEFAULT_SETTINGS.viewMode)
    expect(result.showTasks).toBe(false)
  })

  test("uses defaults for invalid entryStyle", () => {
    const json = JSON.stringify({ entryStyle: "invalid" })
    const result = parseSettingsJson(json)
    expect(result.entryStyle).toBe(DEFAULT_SETTINGS.entryStyle)
  })

  test("uses defaults for non-boolean showTasks", () => {
    const json = JSON.stringify({ showTasks: "true" })
    const result = parseSettingsJson(json)
    expect(result.showTasks).toBe(DEFAULT_SETTINGS.showTasks)
  })
})

describe("serializeSettings", () => {
  test("serializes settings to JSON", () => {
    const settings = {
      viewMode: "schedule" as const,
      showTasks: false,
      entryStyle: "filled" as const,
    }
    const result = serializeSettings(settings)
    expect(JSON.parse(result)).toEqual(settings)
  })
})

describe("updateViewMode", () => {
  test("updates view mode to schedule", () => {
    const settings = { ...DEFAULT_SETTINGS }
    const result = updateViewMode(settings, "schedule")
    expect(result.viewMode).toBe("schedule")
    expect(result.showTasks).toBe(settings.showTasks)
    expect(result.entryStyle).toBe(settings.entryStyle)
  })

  test("updates view mode to compact", () => {
    const settings = { ...DEFAULT_SETTINGS, viewMode: "schedule" as const }
    const result = updateViewMode(settings, "compact")
    expect(result.viewMode).toBe("compact")
  })

  test("returns new object (immutable)", () => {
    const settings = { ...DEFAULT_SETTINGS }
    const result = updateViewMode(settings, "schedule")
    expect(result).not.toBe(settings)
  })
})

describe("updateShowTasks", () => {
  test("updates showTasks to false", () => {
    const settings = { ...DEFAULT_SETTINGS }
    const result = updateShowTasks(settings, false)
    expect(result.showTasks).toBe(false)
    expect(result.viewMode).toBe(settings.viewMode)
  })

  test("updates showTasks to true", () => {
    const settings = { ...DEFAULT_SETTINGS, showTasks: false }
    const result = updateShowTasks(settings, true)
    expect(result.showTasks).toBe(true)
  })

  test("returns new object (immutable)", () => {
    const settings = { ...DEFAULT_SETTINGS }
    const result = updateShowTasks(settings, false)
    expect(result).not.toBe(settings)
  })
})

describe("toggleShowTasks", () => {
  test("toggles from true to false", () => {
    const settings = { ...DEFAULT_SETTINGS, showTasks: true }
    const result = toggleShowTasks(settings)
    expect(result.showTasks).toBe(false)
  })

  test("toggles from false to true", () => {
    const settings = { ...DEFAULT_SETTINGS, showTasks: false }
    const result = toggleShowTasks(settings)
    expect(result.showTasks).toBe(true)
  })

  test("returns new object (immutable)", () => {
    const settings = { ...DEFAULT_SETTINGS }
    const result = toggleShowTasks(settings)
    expect(result).not.toBe(settings)
  })
})

describe("updateEntryStyle", () => {
  test("updates entry style to filled", () => {
    const settings = { ...DEFAULT_SETTINGS }
    const result = updateEntryStyle(settings, "filled")
    expect(result.entryStyle).toBe("filled")
    expect(result.viewMode).toBe(settings.viewMode)
    expect(result.showTasks).toBe(settings.showTasks)
  })

  test("updates entry style to compact", () => {
    const settings = { ...DEFAULT_SETTINGS, entryStyle: "filled" as const }
    const result = updateEntryStyle(settings, "compact")
    expect(result.entryStyle).toBe("compact")
  })

  test("returns new object (immutable)", () => {
    const settings = { ...DEFAULT_SETTINGS }
    const result = updateEntryStyle(settings, "filled")
    expect(result).not.toBe(settings)
  })

  test("preserves other settings", () => {
    const settings = {
      viewMode: "schedule" as const,
      showTasks: false,
      entryStyle: "compact" as const,
    }
    const result = updateEntryStyle(settings, "filled")
    expect(result.viewMode).toBe("schedule")
    expect(result.showTasks).toBe(false)
    expect(result.entryStyle).toBe("filled")
  })
})
