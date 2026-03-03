/**
 * Pure functions for UI annotation overlay.
 * No DOM access, no side effects -- all functions take data in and return data out.
 */

export type AnnotationIntent = "fix" | "change" | "question" | "approve"
export type AnnotationSeverity = "blocking" | "important" | "suggestion"
export type AnnotationStatus = "pending" | "acknowledged" | "resolved" | "dismissed"
export type ThreadAuthor = "human" | "agent"

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

export interface ThreadMessage {
  id: string
  annotation_id: string
  message: string
  author: ThreadAuthor
  timestamp: string
}

export interface AccessibilityInfo {
  role?: string
  label?: string
  description?: string
}

export interface Annotation {
  id: string
  timestamp: string
  tag_name: string
  text_content: string
  bounding_box: BoundingBox
  computed_styles: ComputedStyles
  screenshot: string | null
  component_name: string | null
  // AFS fields
  session_id: string
  url: string
  element_path: string
  comment: string
  intent: AnnotationIntent
  severity: AnnotationSeverity
  status: AnnotationStatus
  selected_text: string | null
  nearby_text: string | null
  css_classes: string[]
  accessibility: AccessibilityInfo | null
  full_path: string | null
  is_fixed: boolean
  resolved_at: string | null
  resolved_by: string | null
  thread: ThreadMessage[]
}

export interface AnnotationSummary {
  total: number
  pending: number
  acknowledged: number
  resolved: number
  dismissed: number
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
 * Return a color hex code for the given annotation status.
 */
export function statusColor(status: AnnotationStatus): string {
  switch (status) {
    case "pending":
      return "#3b82f6" // blue
    case "acknowledged":
      return "#eab308" // yellow
    case "resolved":
      return "#22c55e" // green
    case "dismissed":
      return "#9ca3af" // gray
  }
}

/**
 * Return an icon character for the given annotation intent.
 */
export function intentIcon(intent: AnnotationIntent): string {
  switch (intent) {
    case "fix":
      return "\u{1F527}"
    case "change":
      return "\u{270F}\u{FE0F}"
    case "question":
      return "\u{2753}"
    case "approve":
      return "\u{2705}"
  }
}

/**
 * Return a human-readable label for the given severity level.
 */
export function severityLabel(severity: AnnotationSeverity): string {
  switch (severity) {
    case "blocking":
      return "Blocking"
    case "important":
      return "Important"
    case "suggestion":
      return "Suggestion"
  }
}

/**
 * Group annotations by their status into a record keyed by AnnotationStatus.
 */
export function groupByStatus(annotations: Annotation[]): Record<AnnotationStatus, Annotation[]> {
  const groups: Record<AnnotationStatus, Annotation[]> = {
    pending: [],
    acknowledged: [],
    resolved: [],
    dismissed: [],
  }
  for (const ann of annotations) {
    groups[ann.status].push(ann)
  }
  return groups
}

/**
 * Format a single annotation as markdown.
 */
export function formatAnnotationMarkdown(annotation: Annotation): string {
  const component = annotation.component_name ? ` (${annotation.component_name})` : ""
  const statusTag = annotation.status !== "pending" ? ` [${annotation.status.toUpperCase()}]` : ""
  const lines = [
    `### \`${annotation.element_path}\`${component}${statusTag}`,
    "",
    `**Comment:** ${annotation.comment}`,
    `**Intent:** ${intentIcon(annotation.intent)} ${annotation.intent}`,
    `**Severity:** ${severityLabel(annotation.severity)}`,
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

  if (annotation.resolved_by) {
    lines.push(`**Resolved by:** ${annotation.resolved_by}`)
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
  elementPath: string,
  componentName: string | null,
  tagName: string,
  textContent: string,
  comment: string,
  boundingBox: BoundingBox,
  computedStyles: ComputedStyles,
  screenshot: string | null,
  options?: {
    intent?: AnnotationIntent
    severity?: AnnotationSeverity
    selectedText?: string | null
    nearbyText?: string | null
    cssClasses?: string[]
    accessibility?: AccessibilityInfo | null
    fullPath?: string | null
  },
): Record<string, unknown> {
  return {
    url: typeof window !== "undefined" ? window.location.href : "",
    element_path: elementPath,
    component_name: componentName,
    tag_name: tagName,
    text_content: truncateTextContent(textContent, 200),
    comment,
    bounding_box: boundingBox,
    computed_styles: computedStyles,
    screenshot,
    intent: options?.intent ?? "fix",
    severity: options?.severity ?? "suggestion",
    selected_text: options?.selectedText ?? null,
    nearby_text: options?.nearbyText ?? null,
    css_classes: options?.cssClasses ?? [],
    accessibility: options?.accessibility ?? null,
    full_path: options?.fullPath ?? null,
  }
}

/**
 * Whether an annotation should be visible on the overlay.
 * Only pending annotations are shown — acknowledged, resolved,
 * and dismissed annotations disappear from the UI.
 */
export function isVisibleAnnotation(annotation: Annotation): boolean {
  return annotation.status === "pending"
}
