import { describe, expect, test } from "bun:test"
import {
  type Annotation,
  buildCreateAnnotationBody,
  extractComputedStyles,
  formatAnnotationMarkdown,
  formatAnnotationsMarkdown,
  generateSelector,
  groupByStatus,
  intentIcon,
  isVisibleAnnotation,
  severityLabel,
  statusColor,
  truncateTextContent,
} from "../annotations"

const emptyStyles = {
  color: "",
  background_color: "",
  font_size: "",
  font_family: "",
  padding: "",
  margin: "",
  width: "",
  height: "",
  display: "",
  position: "",
}

function makeAnnotation(overrides: Partial<Annotation> = {}): Annotation {
  return {
    id: "test-1",
    timestamp: "2024-01-15T10:00:00Z",
    tag_name: "h1",
    text_content: "January 2024",
    bounding_box: { top: 0, left: 0, width: 100, height: 50 },
    computed_styles: {
      color: "black",
      background_color: "white",
      font_size: "16px",
      font_family: "Inter",
      padding: "0",
      margin: "0",
      width: "100px",
      height: "50px",
      display: "block",
      position: "relative",
    },
    screenshot: null,
    component_name: "CalendarHeader",
    session_id: "session-1",
    url: "http://localhost:3000/calendar/1",
    element_path: "div.calendar > h1",
    comment: "Font size too small",
    intent: "fix",
    severity: "suggestion",
    status: "pending",
    selected_text: null,
    nearby_text: null,
    css_classes: [],
    accessibility: null,
    full_path: null,
    is_fixed: false,
    resolved_at: null,
    resolved_by: null,
    thread: [],
    ...overrides,
  }
}

describe("generateSelector", () => {
  test("simple tag", () => {
    expect(generateSelector("div", [], null, [])).toBe("div")
  })

  test("tag with id", () => {
    expect(generateSelector("div", ["foo"], "main", [])).toBe("div#main")
  })

  test("tag with classes", () => {
    expect(generateSelector("div", ["foo", "bar"], null, [])).toBe("div.foo.bar")
  })

  test("with parent selectors", () => {
    expect(generateSelector("h1", ["title"], null, ["body", "div.app"])).toBe(
      "body > div.app > h1.title",
    )
  })

  test("id takes precedence over classes", () => {
    expect(generateSelector("div", ["foo"], "main", ["body"])).toBe("body > div#main")
  })
})

describe("extractComputedStyles", () => {
  test("extracts known properties", () => {
    const styles: Record<string, string> = {
      color: "rgb(0, 0, 0)",
      backgroundColor: "rgb(255, 255, 255)",
      fontSize: "16px",
      fontFamily: "Inter",
      padding: "8px",
      margin: "0px",
      width: "100px",
      height: "50px",
      display: "block",
      position: "relative",
    }
    const result = extractComputedStyles(styles)
    expect(result.color).toBe("rgb(0, 0, 0)")
    expect(result.font_size).toBe("16px")
    expect(result.display).toBe("block")
  })

  test("returns empty strings for missing properties", () => {
    const result = extractComputedStyles({})
    expect(result.color).toBe("")
    expect(result.font_size).toBe("")
  })
})

describe("truncateTextContent", () => {
  test("returns short text as-is", () => {
    expect(truncateTextContent("Hello", 100)).toBe("Hello")
  })

  test("truncates long text", () => {
    const text = "a".repeat(50)
    expect(truncateTextContent(text, 10)).toBe("aaaaaaaaaa\u2026")
  })

  test("trims whitespace", () => {
    expect(truncateTextContent("  hello  world  ", 100)).toBe("hello world")
  })

  test("collapses internal whitespace", () => {
    expect(truncateTextContent("hello   \n  world", 100)).toBe("hello world")
  })
})

describe("statusColor", () => {
  test("pending returns blue", () => {
    expect(statusColor("pending")).toBe("#3b82f6")
  })

  test("acknowledged returns yellow", () => {
    expect(statusColor("acknowledged")).toBe("#eab308")
  })

  test("resolved returns green", () => {
    expect(statusColor("resolved")).toBe("#22c55e")
  })

  test("dismissed returns gray", () => {
    expect(statusColor("dismissed")).toBe("#9ca3af")
  })
})

describe("intentIcon", () => {
  test("fix returns wrench", () => {
    expect(intentIcon("fix")).toBe("\u{1F527}")
  })

  test("change returns pencil", () => {
    expect(intentIcon("change")).toBe("\u{270F}\u{FE0F}")
  })

  test("question returns question mark", () => {
    expect(intentIcon("question")).toBe("\u{2753}")
  })

  test("approve returns check mark", () => {
    expect(intentIcon("approve")).toBe("\u{2705}")
  })
})

describe("severityLabel", () => {
  test("blocking", () => {
    expect(severityLabel("blocking")).toBe("Blocking")
  })

  test("important", () => {
    expect(severityLabel("important")).toBe("Important")
  })

  test("suggestion", () => {
    expect(severityLabel("suggestion")).toBe("Suggestion")
  })
})

describe("groupByStatus", () => {
  test("empty list returns empty groups", () => {
    const result = groupByStatus([])
    expect(result.pending).toEqual([])
    expect(result.acknowledged).toEqual([])
    expect(result.resolved).toEqual([])
    expect(result.dismissed).toEqual([])
  })

  test("groups annotations by status", () => {
    const pending = makeAnnotation({ id: "1", status: "pending" })
    const resolved = makeAnnotation({ id: "2", status: "resolved" })
    const dismissed = makeAnnotation({ id: "3", status: "dismissed" })
    const acknowledged = makeAnnotation({ id: "4", status: "acknowledged" })
    const result = groupByStatus([pending, resolved, dismissed, acknowledged])
    expect(result.pending).toEqual([pending])
    expect(result.resolved).toEqual([resolved])
    expect(result.dismissed).toEqual([dismissed])
    expect(result.acknowledged).toEqual([acknowledged])
  })
})

describe("formatAnnotationMarkdown", () => {
  test("includes element_path and component", () => {
    const annotation = makeAnnotation()
    const result = formatAnnotationMarkdown(annotation)
    expect(result).toContain("`div.calendar > h1` (CalendarHeader)")
  })

  test("includes comment", () => {
    const annotation = makeAnnotation()
    const result = formatAnnotationMarkdown(annotation)
    expect(result).toContain("**Comment:** Font size too small")
  })

  test("includes intent and severity", () => {
    const annotation = makeAnnotation()
    const result = formatAnnotationMarkdown(annotation)
    expect(result).toContain("**Intent:**")
    expect(result).toContain("**Severity:** Suggestion")
  })

  test("no status tag for pending", () => {
    const annotation = makeAnnotation({ status: "pending" })
    const result = formatAnnotationMarkdown(annotation)
    expect(result).not.toContain("[PENDING]")
  })

  test("includes status tag for resolved", () => {
    const annotation = makeAnnotation({ status: "resolved", resolved_by: "agent" })
    const result = formatAnnotationMarkdown(annotation)
    expect(result).toContain("[RESOLVED]")
    expect(result).toContain("**Resolved by:** agent")
  })

  test("includes status tag for acknowledged", () => {
    const annotation = makeAnnotation({ status: "acknowledged" })
    const result = formatAnnotationMarkdown(annotation)
    expect(result).toContain("[ACKNOWLEDGED]")
  })
})

describe("formatAnnotationsMarkdown", () => {
  test("empty list", () => {
    expect(formatAnnotationsMarkdown([])).toBe("No annotations.")
  })

  test("formats header with count", () => {
    const annotations = [makeAnnotation()]
    const result = formatAnnotationsMarkdown(annotations)
    expect(result).toContain("# UI Annotations (1)")
  })
})

describe("buildCreateAnnotationBody", () => {
  test("truncates text content", () => {
    const body = buildCreateAnnotationBody(
      "div",
      null,
      "div",
      "a".repeat(300),
      "note",
      { top: 0, left: 0, width: 0, height: 0 },
      emptyStyles,
      null,
    )
    expect((body.text_content as string).length).toBeLessThanOrEqual(201) // 200 + ellipsis
  })

  test("includes AFS fields with defaults", () => {
    const body = buildCreateAnnotationBody(
      "div.app > button",
      "MyComponent",
      "button",
      "Click me",
      "Button too small",
      { top: 10, left: 20, width: 100, height: 40 },
      emptyStyles,
      null,
    )
    expect(body.element_path).toBe("div.app > button")
    expect(body.comment).toBe("Button too small")
    expect(body.intent).toBe("fix")
    expect(body.severity).toBe("suggestion")
    expect(body.css_classes).toEqual([])
  })

  test("accepts options overrides", () => {
    const body = buildCreateAnnotationBody(
      "div",
      null,
      "div",
      "",
      "test",
      { top: 0, left: 0, width: 0, height: 0 },
      emptyStyles,
      null,
      { intent: "question", severity: "blocking", cssClasses: ["btn", "primary"] },
    )
    expect(body.intent).toBe("question")
    expect(body.severity).toBe("blocking")
    expect(body.css_classes).toEqual(["btn", "primary"])
  })
})

describe("isVisibleAnnotation", () => {
  test("pending annotation is visible", () => {
    expect(isVisibleAnnotation(makeAnnotation({ status: "pending" }))).toBe(true)
  })

  test("acknowledged annotation is not visible", () => {
    expect(isVisibleAnnotation(makeAnnotation({ status: "acknowledged" }))).toBe(false)
  })

  test("resolved annotation is not visible", () => {
    expect(isVisibleAnnotation(makeAnnotation({ status: "resolved" }))).toBe(false)
  })

  test("dismissed annotation is not visible", () => {
    expect(isVisibleAnnotation(makeAnnotation({ status: "dismissed" }))).toBe(false)
  })
})
