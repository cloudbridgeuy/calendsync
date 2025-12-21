# systemfd Integration

This document describes how calendsync uses `systemfd` and `listenfd` for socket activation during development.

## Overview

[systemfd](https://github.com/mitsuhiko/systemfd) is a utility that opens sockets and passes them to child processes via file descriptors. This enables:

- **Zero-downtime restarts**: Socket stays open while the server binary restarts
- **Seamless cargo-watch integration**: Recompile and restart without dropping connections
- **Socket preservation**: Connected clients aren't disconnected during reloads

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      systemfd                                    │
│  - Opens socket on port 3000                                     │
│  - Passes file descriptor to child                               │
│  - Respawns child on exit                                        │
├─────────────────────────────────────────────────────────────────┤
│                      cargo watch                                 │
│  - Watches Rust source files                                     │
│  - Recompiles on change                                          │
│  - Restarts binary (inherits socket FD)                          │
├─────────────────────────────────────────────────────────────────┤
│                      calendsync binary                           │
│  - Uses listenfd to receive socket                               │
│  - Falls back to new socket if none provided                     │
└─────────────────────────────────────────────────────────────────┘
```

## Usage

### For Rust-Only Changes (No TypeScript)

```bash
# Uses systemfd for socket preservation during Rust rebuilds
systemfd --no-pid -s http::3000 -- cargo watch -x 'run -p calendsync'
```

This is useful when:
- Making changes to Rust code only
- No TypeScript/CSS changes expected
- Need fastest possible Rust iteration

### For Full-Stack Development (Recommended)

```bash
# Uses custom xtask with TypeScript hot-reload
cargo xtask dev server
```

This is the recommended approach because it:
- Watches both Rust and TypeScript
- Hot-reloads TypeScript without server restart
- Auto-refreshes browser on changes
- Shows build errors in browser overlay

## How listenfd Works in calendsync

In `crates/calendsync/src/main.rs`:

```rust
use listenfd::ListenFd;
use tokio::net::TcpListener;

// Check for inherited file descriptor
let mut listenfd = ListenFd::from_env();
let listener = match listenfd.take_tcp_listener(0)? {
    // systemfd passed us a socket
    Some(listener) => {
        listener.set_nonblocking(true)?;
        TcpListener::from_std(listener)?
    }
    // No inherited socket, create our own
    None => {
        let addr = format!("{}:{}", cli.host, cli.port);
        TcpListener::bind(&addr).await?
    }
};
```

Key points:
- `ListenFd::from_env()` checks for `LISTEN_FDS` environment variable
- `take_tcp_listener(0)` gets the first file descriptor (index 0)
- If no FD is passed, falls back to regular socket binding
- Works identically whether started via systemfd or directly

## Comparison of Approaches

| Feature | systemfd + cargo-watch | cargo xtask dev server |
|---------|----------------------|---------------------|
| Rust changes | Recompiles + restarts | Recompiles + restarts |
| TypeScript changes | Not watched | Hot-reloads (no restart) |
| CSS changes | Not watched | Hot-swaps (no reload) |
| Browser refresh | Manual | Automatic |
| Build errors | Terminal only | Browser overlay |
| Socket preservation | Yes | No |
| Setup | External tools | Built-in |

## When to Use Each Approach

**Use `systemfd --no-pid -s http::3000 -- cargo watch`:**
- Pure Rust development
- Testing server behavior during restarts
- Debugging socket preservation

**Use `cargo xtask dev server`:**
- Full-stack development (Rust + TypeScript)
- UI development
- Most common development scenarios

## Environment Variables

| Variable | Effect |
|----------|--------|
| `LISTEN_FDS` | Set by systemfd, number of file descriptors passed |
| `LISTEN_PID` | Set by systemfd, PID of current process |
| `DEV_MODE` | Enables hot-reload endpoints (set by xtask) |
| `PORT` | Server port (default: 3000) |
| `HOST` | Server host (default: 0.0.0.0) |

## Installation

```bash
# Install systemfd
cargo install systemfd

# Install cargo-watch (for cargo watch command)
cargo install cargo-watch
```

## Troubleshooting

### "Address already in use"

When using systemfd, the socket is held open. If you restart systemfd itself, it may fail:

```bash
# Kill any existing systemfd processes
pkill systemfd

# Try again
systemfd --no-pid -s http::3000 -- cargo watch -x 'run -p calendsync'
```

### Server not receiving socket

Check that listenfd is properly imported and the dependency is in Cargo.toml:

```toml
[dependencies]
listenfd = { workspace = true }
```

### Port conflict with xtask dev server

The xtask dev command doesn't use systemfd - it binds directly. If you previously used systemfd, make sure it's stopped:

```bash
pkill systemfd
cargo xtask dev server
```
