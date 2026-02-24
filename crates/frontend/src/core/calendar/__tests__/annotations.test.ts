import { describe, expect, test } from "bun:test"
import {
  type Annotation,
  buildCreateAnnotationBody,
  extractComputedStyles,
  formatAnnotationMarkdown,
  formatAnnotationsMarkdown,
  generateSelector,
  truncateTextContent,
} from "../annotations"

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

describe("formatAnnotationMarkdown", () => {
  const annotation: Annotation = {
    id: "test-1",
    timestamp: "2024-01-15T10:00:00Z",
    selector: "div.calendar > h1",
    component_name: "CalendarHeader",
    tag_name: "h1",
    text_content: "January 2024",
    note: "Font size too small",
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
    resolved: false,
    resolution_summary: null,
  }

  test("includes selector and component", () => {
    const result = formatAnnotationMarkdown(annotation)
    expect(result).toContain("`div.calendar > h1` (CalendarHeader)")
  })

  test("includes note", () => {
    const result = formatAnnotationMarkdown(annotation)
    expect(result).toContain("**Note:** Font size too small")
  })

  test("excludes resolution when not resolved", () => {
    const result = formatAnnotationMarkdown(annotation)
    expect(result).not.toContain("**Resolution:**")
  })

  test("includes resolution when resolved", () => {
    const resolved = { ...annotation, resolved: true, resolution_summary: "Fixed it" }
    const result = formatAnnotationMarkdown(resolved)
    expect(result).toContain("[RESOLVED]")
    expect(result).toContain("**Resolution:** Fixed it")
  })
})

describe("formatAnnotationsMarkdown", () => {
  test("empty list", () => {
    expect(formatAnnotationsMarkdown([])).toBe("No annotations.")
  })

  test("formats header with count", () => {
    const annotations: Annotation[] = [
      {
        id: "1",
        timestamp: "",
        selector: "div",
        component_name: null,
        tag_name: "div",
        text_content: "",
        note: "test",
        bounding_box: { top: 0, left: 0, width: 0, height: 0 },
        computed_styles: {
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
        },
        screenshot: null,
        resolved: false,
        resolution_summary: null,
      },
    ]
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
      {
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
      },
      null,
    )
    expect(body.text_content.length).toBeLessThanOrEqual(201) // 200 + ellipsis
  })
})
