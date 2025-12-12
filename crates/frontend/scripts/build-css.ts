/**
 * Build CSS with content hashing.
 *
 * Copies src/calendsync/styles.css to dist/calendsync-[hash].css
 * where [hash] is an 8-character hash of the file content.
 */

import { createHash } from "node:crypto"
import { copyFile, readdir, rm } from "node:fs/promises"

const SRC_CSS = "./src/calendsync/styles.css"
const DIST_DIR = "./dist"

async function main() {
  // Read the source CSS
  const file = Bun.file(SRC_CSS)
  const content = await file.arrayBuffer()

  // Compute content hash (8 chars, like bun's default)
  const hash = createHash("sha256").update(Buffer.from(content)).digest("hex").slice(0, 8)

  const outputFilename = `calendsync-${hash}.css`
  const outputPath = `${DIST_DIR}/${outputFilename}`

  // Remove any old calendsync-*.css files
  const files = await readdir(DIST_DIR).catch(() => [])
  for (const f of files) {
    if (f.startsWith("calendsync-") && f.endsWith(".css")) {
      await rm(`${DIST_DIR}/${f}`)
    }
  }

  // Copy CSS with new hashed name
  await copyFile(SRC_CSS, outputPath)

  console.log(`CSS built: ${outputFilename}`)
}

main().catch((err) => {
  console.error("CSS build failed:", err)
  process.exit(1)
})
