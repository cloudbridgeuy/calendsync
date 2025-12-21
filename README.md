# CalendSync

A calendar sync application built with Rust, React, and Tauri.

## Quick Start

### Prerequisites

- Rust (latest stable)
- Bun (for frontend builds)
- Tauri CLI (`cargo install tauri-cli`) - for desktop/mobile

### Running the Application

CalendSync supports three access methods. All require the server running first.

#### 1. Start the Server

```bash
cargo run -p calendsync
```

Server runs on `http://localhost:3000`

#### 2. Access via Web Browser

Create a calendar first, then access it:
```bash
# Create a calendar and get its ID
CALENDAR_ID=$(curl -s -X POST http://localhost:3000/api/calendars \
  -d "name=Personal" -d "color=#3B82F6" | jq -r '.id')

# Open in browser
open "http://localhost:3000/calendar/$CALENDAR_ID"
```

#### 3. Run Desktop App (macOS)

```bash
# In a new terminal (keep server running)
cargo tauri dev
```

#### 4. Run Mobile App (iOS)

```bash
# First-time setup
cargo tauri ios init

# Run on simulator
cargo tauri ios dev

# Run on physical device
cargo tauri ios dev --host
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Axum Server (port 3000)                    │
│  - REST API (/api/*)                                            │
│  - React SSR pages (/calendar/{id})                             │
│  - SSE events (/api/events)                                     │
└─────────────────────────────────────────────────────────────────┘
         ▲                    ▲                    ▲
         │ HTTP/SSE           │ HTTP/SSE           │ HTTP/SSE
    ┌────┴────┐          ┌────┴────┐          ┌────┴────┐
    │   Web   │          │ Desktop │          │  Mobile │
    │ Browser │          │  Tauri  │          │  Tauri  │
    └─────────┘          └─────────┘          └─────────┘
```

## Tech Stack

- **Backend**: Rust, Axum, Tokio
- **Frontend**: React 19, TypeScript, Bun
- **SSR**: deno_core (JavaScript runtime in Rust)
- **Desktop/Mobile**: Tauri v2
- **Real-time**: Server-Sent Events (SSE)

## Development

```bash
# Run code quality checks
cargo xtask lint

# Auto-fix formatting
cargo xtask lint --fix

# Build frontend
cd crates/frontend && bun run build
```

## Project Structure

```
crates/
├── calendsync/     # Main web server
├── core/           # Pure business logic
├── client/         # CLI client
├── frontend/       # TypeScript/React code
├── ssr/            # React SSR worker pool
├── ssr_core/       # Pure SSR functions
└── src-tauri/      # Tauri desktop/mobile app
```

## Documentation

For detailed documentation, see:

- [Web Server](crates/calendsync/README.md)
- [CLI Client](crates/client/README.md)
- [CLAUDE.md](CLAUDE.md) - Development guidelines
