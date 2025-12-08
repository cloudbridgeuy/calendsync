/**
 * Bun development server for Tauri with hot reload support.
 *
 * Features:
 * - Static file serving from dist/
 * - WebSocket-based hot reload (HMR)
 * - TAURI_DEV_HOST support for iOS physical devices
 * - File watching for automatic rebuilds
 *
 * Usage:
 *   bun run ./dev-server.ts
 *
 * Environment:
 *   TAURI_DEV_HOST - Set by Tauri CLI for iOS physical device development
 */

import { existsSync, mkdirSync, watch } from "node:fs"
import { join } from "node:path"
import { spawn } from "bun"

// Configuration
const PORT = 5173
const WS_PORT = 5174
const HOST = process.env.TAURI_DEV_HOST || "localhost"
const DIST_DIR = join(import.meta.dir, "dist-dev")
const SRC_DIR = join(import.meta.dir, "src")

// Track connected WebSocket clients for HMR
const clients = new Set<WebSocket>()

// Content type mapping
const CONTENT_TYPES: Record<string, string> = {
    ".html": "text/html",
    ".js": "application/javascript",
    ".css": "text/css",
    ".json": "application/json",
    ".png": "image/png",
    ".jpg": "image/jpeg",
    ".svg": "image/svg+xml",
    ".ico": "image/x-icon",
}

function getContentType(path: string): string {
    const ext = path.substring(path.lastIndexOf("."))
    return CONTENT_TYPES[ext] || "application/octet-stream"
}

// HMR client script injected into HTML
function getHmrScript(): string {
    return `
<script>
(function() {
    const ws = new WebSocket('ws://${HOST}:${WS_PORT}');
    ws.onopen = () => console.log('[HMR] Connected');
    ws.onmessage = (e) => {
        if (e.data === 'reload') {
            console.log('[HMR] Reloading...');
            location.reload();
        }
    };
    ws.onclose = () => {
        console.log('[HMR] Connection lost, retrying in 1s...');
        setTimeout(() => location.reload(), 1000);
    };
    ws.onerror = (e) => console.error('[HMR] Error:', e);
    // Keepalive ping
    setInterval(() => {
        if (ws.readyState === WebSocket.OPEN) {
            ws.send('ping');
        }
    }, 30000);
})();
</script>
`
}

// WebSocket server for hot reload notifications
const _wsServer = Bun.serve({
    port: WS_PORT,
    hostname: HOST,
    fetch(req, server) {
        // Upgrade HTTP to WebSocket
        if (server.upgrade(req)) {
            return undefined
        }
        return new Response("WebSocket server", { status: 200 })
    },
    websocket: {
        open(ws) {
            clients.add(ws as unknown as WebSocket)
            console.log(`[HMR] Client connected (${clients.size} total)`)
        },
        close(ws) {
            clients.delete(ws as unknown as WebSocket)
            console.log(`[HMR] Client disconnected (${clients.size} total)`)
        },
        message(ws, message) {
            // Handle keepalive ping
            if (message === "ping") {
                ws.send("pong")
            }
        },
    },
})

// Notify all connected clients to reload
function notifyReload() {
    console.log(`[HMR] Notifying ${clients.size} client(s) to reload`)
    for (const client of clients) {
        try {
            ;(client as unknown as { send: (msg: string) => void }).send("reload")
        } catch {
            // Client may have disconnected
        }
    }
}

// HTTP server for static file serving
const _httpServer = Bun.serve({
    port: PORT,
    hostname: HOST,
    async fetch(req) {
        const url = new URL(req.url)
        let path = url.pathname

        // Default to index.html for root
        if (path === "/" || path === "") {
            path = "/index.html"
        }

        const filePath = join(DIST_DIR, path)
        const file = Bun.file(filePath)

        // Check if file exists
        if (!(await file.exists())) {
            // For SPA routing, fallback to index.html
            const indexPath = join(DIST_DIR, "index.html")
            const indexFile = Bun.file(indexPath)

            if (await indexFile.exists()) {
                let html = await indexFile.text()
                // Inject HMR script
                html = html.replace("</body>", `${getHmrScript()}</body>`)
                return new Response(html, {
                    headers: { "Content-Type": "text/html" },
                })
            }

            return new Response("Not Found", { status: 404 })
        }

        // Inject HMR script into HTML files
        if (path.endsWith(".html")) {
            let html = await file.text()
            html = html.replace("</body>", `${getHmrScript()}</body>`)
            return new Response(html, {
                headers: { "Content-Type": "text/html" },
            })
        }

        // Serve static file with correct content type
        return new Response(file, {
            headers: { "Content-Type": getContentType(path) },
        })
    },
})

// Ensure dist-dev directory exists
if (!existsSync(DIST_DIR)) {
    mkdirSync(DIST_DIR, { recursive: true })
}

console.log(`[Dev Server] HTTP server: http://${HOST}:${PORT}`)
console.log(`[Dev Server] WebSocket HMR: ws://${HOST}:${WS_PORT}`)
console.log(`[Dev Server] Serving from: ${DIST_DIR}`)

// File watcher for auto-rebuild
let rebuildTimeout: ReturnType<typeof setTimeout> | null = null
let isRebuilding = false

async function rebuild() {
    if (isRebuilding) return
    isRebuilding = true

    console.log("[Watch] Rebuilding...")

    const proc = spawn(["bun", "run", "build:tauri:dev:live"], {
        cwd: import.meta.dir,
        stdout: "inherit",
        stderr: "inherit",
    })

    const exitCode = await proc.exited

    if (exitCode === 0) {
        console.log("[Watch] Build complete")
        notifyReload()
    } else {
        console.error(`[Watch] Build failed with exit code ${exitCode}`)
    }

    isRebuilding = false
}

// Watch src directory for changes
watch(SRC_DIR, { recursive: true }, (_event, filename) => {
    if (!filename) return

    // Ignore non-source files
    if (filename.endsWith(".d.ts") || filename.includes("node_modules")) {
        return
    }

    // Debounce rebuilds (wait 100ms for multiple rapid changes)
    if (rebuildTimeout) {
        clearTimeout(rebuildTimeout)
    }

    rebuildTimeout = setTimeout(() => {
        console.log(`[Watch] Change detected: ${filename}`)
        rebuild()
    }, 100)
})

console.log(`[Watch] Watching ${SRC_DIR} for changes...`)
console.log("[Dev Server] Ready! Press Ctrl+C to stop.")
