# Lint --verbose Agent Guidance — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Update CLAUDE.md so agents never use `--verbose` and know how to navigate the log file instead.

**Architecture:** Documentation-only change — single edit to the Lint Checks section of CLAUDE.md.

**Tech Stack:** Markdown

**Applicable patterns:** None (no code changes).

---

## Group 1 (single task)

### Task 1: Update CLAUDE.md Lint Checks section

**File:** `CLAUDE.md` (lines 239–273)

**What to change:**

1. **Add agent-specific guidance** after the opening bold paragraph (line 241). Insert a new paragraph:

```markdown
**For agents (Claude):** Never use `--verbose` — it exists for human terminal use. If you need more context about a specific check's output (e.g., to understand a passing check's warnings), read the log file directly:

- `Read target/xtask-lint.log` — full output of all checks, sectioned by `--- check-name [STATUS] ---` headers
- `Grep "--- cargo clippy" target/xtask-lint.log` — jump to a specific check's output
- `Grep "\\[FAIL\\]" target/xtask-lint.log` — find all failures
```

2. **Remove `--verbose` from the quick-reference code block** (line 245). The code block should become:

```bash
cargo xtask lint              # Run all checks (quiet — only errors print)
cargo xtask lint --fix        # Auto-fix formatting and clippy issues
cargo xtask lint --no-biome   # Skip biome checks
cargo xtask lint --no-typecheck  # Skip TypeScript type checking
cargo xtask lint --no-bun-test   # Skip bun test checks
```

3. **Keep `--verbose` documented** but move it below the check listing (after line 271), in a human-oriented section:

```markdown
### Human Terminal Options

```bash
cargo xtask lint --verbose    # Show all output including passes (human use only)
```
```

**Verification:**

1. Read the updated CLAUDE.md Lint Checks section and confirm:
   - The agent guidance paragraph appears before the code block
   - `--verbose` is NOT in the main code block
   - `--verbose` is documented under a separate "Human Terminal Options" heading
   - The log file navigation examples use the correct `--- name [STATUS] ---` format matching `format_log_entry()` in `xtask/src/lint/mod.rs`
