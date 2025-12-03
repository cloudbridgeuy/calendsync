# calendsync_client

Command-line client for the calendsync API.

## Installation

```bash
cargo install --path crates/client
```

## Usage

```bash
calendsync-client [OPTIONS] <COMMAND>
```

### Global Options

| Option | Description | Default |
|--------|-------------|---------|
| `--base-url <URL>` | API base URL | `http://localhost:3000` |
| `--format <FORMAT>` | Output format: `json`, `pretty` | `pretty` |
| `--quiet` | Suppress non-essential output | false |

Environment: `CALENDSYNC_URL` overrides `--base-url`

## Commands

### Users

```bash
# List all users
calendsync-client users list

# Create a user
calendsync-client users create --name "John Doe" --email "john@example.com"

# Get a user
calendsync-client users get <ID>

# Delete a user
calendsync-client users delete <ID>
```

### Calendars

```bash
# List all calendars
calendsync-client calendars list

# Create a calendar
calendsync-client calendars create --name "Work" --color "#3B82F6"
calendsync-client calendars create --name "Personal" --color "#10B981" --description "My personal calendar"

# Get a calendar
calendsync-client calendars get <ID>

# Update a calendar
calendsync-client calendars update <ID> --name "New Name"
calendsync-client calendars update <ID> --color "#F59E0B"

# Delete a calendar
calendsync-client calendars delete <ID>
```

### Entries

```bash
# List entries for a calendar
calendsync-client entries list --calendar-id <UUID>

# List with date range
calendsync-client entries list --calendar-id <UUID> --start 2024-01-01 --end 2024-01-31

# List around a highlighted day
calendsync-client entries list --calendar-id <UUID> --highlighted-day 2024-01-15

# Create an entry
calendsync-client entries create --calendar-id <UUID> --title "Meeting" --date 2024-01-15 --type timed --start-time 09:00 --end-time 10:00

# Entry types: all-day, timed, task, multi-day
calendsync-client entries create --calendar-id <UUID> --title "Conference" --date 2024-01-20 --type multi-day --end-date 2024-01-22
calendsync-client entries create --calendar-id <UUID> --title "Birthday" --date 2024-01-25 --type all-day
calendsync-client entries create --calendar-id <UUID> --title "Review PR" --date 2024-01-15 --type task

# Get an entry
calendsync-client entries get <ID>

# Update an entry
calendsync-client entries update <ID> --title "Updated Title"
calendsync-client entries update <ID> --completed true

# Toggle task completion
calendsync-client entries toggle <ID>

# Delete an entry
calendsync-client entries delete <ID>
```

### Events (SSE)

```bash
# Watch real-time events for a calendar
calendsync-client events watch <CALENDAR_ID>

# Resume from a specific event ID
calendsync-client events watch <CALENDAR_ID> --last-event-id 123
```

### Health

```bash
# Check SSR health
calendsync-client health ssr

# Get SSR pool stats
calendsync-client health ssr-stats
```

## Output Formats

### Pretty (default)

```
CALENDARS (2)
----------------------------------------
Work (#3B82F6)
  ID: 550e8400-e29b-41d4-a716-446655440001

Personal (#10B981)
  ID: 550e8400-e29b-41d4-a716-446655440002
  Description: My personal calendar
```

### JSON

```bash
calendsync-client --format json calendars list
```

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "name": "Work",
    "color": "#3B82F6",
    "description": null
  }
]
```

## Architecture

```
crates/client/
├── Cargo.toml
└── src/
    ├── main.rs          # CLI entry point
    ├── lib.rs           # Public API
    ├── error.rs         # ClientError enum
    ├── cli/             # Clap command definitions
    │   ├── mod.rs       # Cli struct, Commands enum
    │   ├── users.rs
    │   ├── calendars.rs
    │   ├── entries.rs
    │   ├── events.rs
    │   └── health.rs
    ├── client/          # HTTP client (Imperative Shell)
    │   ├── mod.rs       # CalendsyncClient
    │   ├── users.rs
    │   ├── calendars.rs
    │   ├── entries.rs
    │   ├── events.rs
    │   └── health.rs
    └── output/          # Formatting (Functional Core)
        ├── mod.rs
        ├── json.rs
        └── pretty.rs
```

The crate follows Functional Core - Imperative Shell:

- **CLI** (`cli/`): Pure command definitions with clap
- **Client** (`client/`): HTTP requests (Imperative Shell)
- **Output** (`output/`): Pure formatting functions (Functional Core)
