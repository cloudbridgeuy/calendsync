/**
 * Server entry point for React SSR.
 * Uses React 19's prerender API to render the app to HTML.
 */

import { prerender } from "react-dom/static"

import { App } from "./App"
import type { InitialData, SSRConfig } from "./types"

// Declare the custom ops provided by Rust
declare const Deno: {
  core: {
    ops: {
      op_set_html(html: string): void
    }
  }
}

// Declare the SSR config provided by Rust
declare const __SSR_CONFIG__: SSRConfig

/**
 * Collect a ReadableStream to a string.
 */
async function streamToString(stream: ReadableStream<Uint8Array>): Promise<string> {
  const reader = stream.getReader()
  const decoder = new TextDecoder()
  let html = ""

  while (true) {
    const { done, value } = await reader.read()
    if (done) break
    if (value) {
      html += decoder.decode(value, { stream: true })
    }
  }

  // Flush any remaining bytes
  html += decoder.decode()

  return html
}

/**
 * Main SSR function.
 */
async function main() {
  console.log("[SSR] Starting React SSR with prerender...")

  // Get initial data from Rust via global config
  const initialData: InitialData = __SSR_CONFIG__.initialData

  console.log(`[SSR] Calendar ID: ${initialData.calendarId}`)
  console.log(`[SSR] Highlighted day: ${initialData.highlightedDay}`)
  console.log(`[SSR] Initial entries: ${initialData.days.length} days`)

  try {
    // Use React 19's prerender API
    const { prelude } = await prerender(<App initialData={initialData} />)

    console.log("[SSR] Prerender complete, collecting stream...")

    // Collect the stream to a string
    const html = await streamToString(prelude)

    console.log(`[SSR] HTML generated: ${html.length} bytes`)

    // Send HTML back to Rust
    Deno.core.ops.op_set_html(html)

    console.log("[SSR] HTML sent to Rust")
  } catch (err) {
    console.error("[SSR] Error during render:", err)
    throw err
  }
}

// Run the main function
main()
