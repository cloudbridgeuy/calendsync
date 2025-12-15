/**
 * Update manifest.json with the latest JS and CSS assets.
 *
 * Scans the dist directory for:
 * - calendsync-server.js (unhashed)
 * - calendsync-client-[hash].js (hashed)
 * - calendsync-[hash].css (hashed)
 *
 * For hashed files, keeps only the newest version and removes older ones.
 */

import { readdir, readFile, rm, stat, writeFile } from "node:fs/promises"

const DIST_DIR = "./dist"
const MANIFEST_PATH = "./manifest.json"

interface FileInfo {
  filename: string
  mtime: Date
}

async function main() {
  const files = await readdir(DIST_DIR).catch(() => [])

  // Find server JS (unhashed)
  const serverJs = files.find((f) => f === "calendsync-server.js")

  // Find client JS files (hashed: calendsync-client-[hash].js)
  const clientJsFiles: FileInfo[] = []
  for (const f of files) {
    if (f.startsWith("calendsync-client-") && f.endsWith(".js") && !f.endsWith(".js.map")) {
      const stats = await stat(`${DIST_DIR}/${f}`)
      clientJsFiles.push({ filename: f, mtime: stats.mtime })
    }
  }

  // Sort by modification time (newest first) and keep only the latest
  clientJsFiles.sort((a, b) => b.mtime.getTime() - a.mtime.getTime())
  const latestClientJs = clientJsFiles[0]?.filename

  // Remove old client JS files (keep only the latest)
  for (let i = 1; i < clientJsFiles.length; i++) {
    const oldFile = clientJsFiles[i].filename
    await rm(`${DIST_DIR}/${oldFile}`).catch(() => {})
    await rm(`${DIST_DIR}/${oldFile}.map`).catch(() => {})
    console.log(`Removed old client JS: ${oldFile}`)
  }

  // Find CSS files (hashed: calendsync-[hash].css)
  const cssFiles: FileInfo[] = []
  for (const f of files) {
    if (f.startsWith("calendsync-") && f.endsWith(".css") && !f.includes("-client-")) {
      const stats = await stat(`${DIST_DIR}/${f}`)
      cssFiles.push({ filename: f, mtime: stats.mtime })
    }
  }

  cssFiles.sort((a, b) => b.mtime.getTime() - a.mtime.getTime())
  const latestCss = cssFiles[0]?.filename

  // Remove old CSS files (keep only the latest)
  for (let i = 1; i < cssFiles.length; i++) {
    const oldFile = cssFiles[i].filename
    await rm(`${DIST_DIR}/${oldFile}`).catch(() => {})
    console.log(`Removed old CSS: ${oldFile}`)
  }

  // Read existing manifest or create empty one
  let manifest: Record<string, string> = {}
  try {
    const content = await readFile(MANIFEST_PATH, "utf-8")
    manifest = JSON.parse(content)
  } catch {
    // Manifest doesn't exist, start fresh
  }

  // Update manifest entries
  if (serverJs) {
    manifest["calendsync.js"] = serverJs
  }
  if (latestClientJs) {
    manifest["calendsync-client.js"] = latestClientJs
  }
  if (latestCss) {
    manifest["calendsync.css"] = latestCss
  }

  // Write updated manifest
  await writeFile(MANIFEST_PATH, `${JSON.stringify(manifest, null, 2)}\n`)

  console.log("Manifest updated:", manifest)
}

main().catch((err) => {
  console.error("Manifest update failed:", err)
  process.exit(1)
})
