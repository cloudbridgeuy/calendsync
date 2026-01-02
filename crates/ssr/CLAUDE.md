# CLAUDE.md

## Pommel - Semantic Code Search

This sub-project (ssr) uses Pommel (v0.5.0) for semantic code search with hybrid vector + keyword matching.

**Supported languages:** C#, Dart, Elixir, Go, Java, JavaScript, Kotlin, PHP, Python, Rust, Solidity, Swift, TypeScript

### Code Search Priority

**IMPORTANT: Use `pm search` BEFORE using Grep/Glob for code exploration.**

Pommel saves ~95% of tokens compared to traditional file exploration. When looking for:
- How something is implemented → `pm search "authentication flow"`
- Where a pattern is used → `pm search "error handling"`
- Related code/concepts → `pm search "database connection"`
- Code that does X → `pm search "validate user input"`

Only fall back to Grep/Glob when:
- Searching for an exact string literal (e.g., a specific error message)
- Looking for a specific identifier name you already know
- Pommel daemon is not running

### Quick Search Examples
```bash
# Search within this sub-project (default when running from here)
pm search "authentication logic"

# Search with JSON output
pm search "error handling" --json

# Search across entire monorepo
pm search "shared utilities" --all

# Search specific chunk levels
pm search "class definitions" --level class

# Show detailed match reasons
pm search "rate limiting" --verbose
```

### Available Commands
- `pm search <query>` - Hybrid search this sub-project (or use --all for everything)
- `pm status` - Check daemon status and index statistics
- `pm subprojects` - List all sub-projects
- `pm start` / `pm stop` - Control the background daemon

### Tips
- Searches default to this sub-project when you're in this directory
- Use `--all` to search across the entire monorepo
- Use `--verbose` to see why results matched
- Chunk levels: file (entire files), class (structs/interfaces/classes), method (functions/methods)
