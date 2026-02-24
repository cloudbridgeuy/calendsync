/**
 * Pure functions for UI annotation overlay.
 * No DOM access, no side effects — all functions take data in and return data out.
 */

export interface ComputedStyles {
  color: string
  background_color: string
  font_size: string
  font_family: string
  padding: string
  margin: string
  width: string
  height: string
  display: string
  position: string
}

export interface BoundingBox {
  top: number
  left: number
  width: number
  height: number
}

export interface Annotation {
  id: string
  timestamp: string
  selector: string
  component_name: string | null
  tag_name: string
  text_content: string
  note: string
  bounding_box: BoundingBox
  computed_styles: ComputedStyles
  screenshot: string | null
  resolved: boolean
  resolution_summary: string | null
}

export interface AnnotationSummary {
  total: number
  resolved: number
  unresolved: number
}

export interface AnnotationsListResponse {
  annotations: Annotation[]
  summary: AnnotationSummary
}

/**
 * Generate a unique CSS selector for an element by walking up the DOM tree.
 * Returns a path like "body > div.app > main > h1.title".
 */
export function generateSelector(
  tagName: string,
  classList: string[],
  id: string | null,
  parentSelectors: string[],
): string {
  let current = tagName.toLowerCase()
  if (id) {
    current = `${current}#${id}`
  } else if (classList.length > 0) {
    current = `${current}.${classList.join(".")}`
  }

  if (parentSelectors.length === 0) {
    return current
  }
  return `${parentSelectors.join(" > ")} > ${current}`
}

/**
 * Extract computed styles from a flat object of style properties.
 * Picks only the 10 properties we care about.
 */
export function extractComputedStyles(styles: Record<string, string>): ComputedStyles {
  return {
    color: styles.color ?? "",
    background_color: styles.backgroundColor ?? "",
    font_size: styles.fontSize ?? "",
    font_family: styles.fontFamily ?? "",
    padding: styles.padding ?? "",
    margin: styles.margin ?? "",
    width: styles.width ?? "",
    height: styles.height ?? "",
    display: styles.display ?? "",
    position: styles.position ?? "",
  }
}

/**
 * Truncate text content to a maximum length, adding ellipsis if needed.
 */
export function truncateTextContent(text: string, maxLength: number): string {
  const trimmed = text.trim().replace(/\s+/g, " ")
  if (trimmed.length <= maxLength) {
    return trimmed
  }
  return `${trimmed.slice(0, maxLength)}\u2026`
}

/**
 * Format a single annotation as markdown.
 */
export function formatAnnotationMarkdown(annotation: Annotation): string {
  const component = annotation.component_name ? ` (${annotation.component_name})` : ""
  const status = annotation.resolved ? " [RESOLVED]" : ""
  const lines = [
    `### \`${annotation.selector}\`${component}${status}`,
    "",
    `**Note:** ${annotation.note}`,
    `**Element:** \`<${annotation.tag_name}>\``,
  ]

  if (annotation.text_content) {
    lines.push(`**Text:** "${annotation.text_content}"`)
  }

  lines.push(
    `**Font:** ${annotation.computed_styles.font_family} at ${annotation.computed_styles.font_size}`,
  )
  lines.push(
    `**Color:** ${annotation.computed_styles.color} on ${annotation.computed_styles.background_color}`,
  )

  if (annotation.resolution_summary) {
    lines.push(`**Resolution:** ${annotation.resolution_summary}`)
  }

  return lines.join("\n")
}

/**
 * Format all annotations as a markdown document.
 */
export function formatAnnotationsMarkdown(annotations: Annotation[]): string {
  if (annotations.length === 0) {
    return "No annotations."
  }

  const header = `# UI Annotations (${annotations.length})\n\n`
  const body = annotations.map(formatAnnotationMarkdown).join("\n\n---\n\n")
  return header + body
}

/**
 * Build the request body for creating a new annotation from element data.
 */
export function buildCreateAnnotationBody(
  selector: string,
  componentName: string | null,
  tagName: string,
  textContent: string,
  note: string,
  boundingBox: BoundingBox,
  computedStyles: ComputedStyles,
  screenshot: string | null,
): Omit<Annotation, "id" | "timestamp" | "resolved" | "resolution_summary"> {
  return {
    selector,
    component_name: componentName,
    tag_name: tagName,
    text_content: truncateTextContent(textContent, 200),
    note,
    bounding_box: boundingBox,
    computed_styles: computedStyles,
    screenshot,
  }
}
