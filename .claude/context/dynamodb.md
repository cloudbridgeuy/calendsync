# DynamoDB Infrastructure

This document covers the DynamoDB infrastructure setup and xtask commands for calendsync.

## Overview

calendsync uses a single-table DynamoDB design with composite keys. For the full schema documentation, see `docs/dynamodb.md`.

## xtask Commands

### Deploy Table

```bash
# Deploy (creates table if not exists, adds missing GSIs)
cargo xtask dynamodb deploy

# Deploy without confirmation prompt
cargo xtask dynamodb deploy --force

# Destroy the table
cargo xtask dynamodb deploy --destroy

# Use custom table name
cargo xtask dynamodb deploy --table-name my-table
```

**Behavior**:
- Idempotent: safe to run multiple times
- Shows plan before applying changes
- Waits for table to become ACTIVE

### Seed Data

```bash
# Seed mock entries for a calendar
cargo xtask dynamodb seed --calendar-id <UUID>

# Seed around a specific date
cargo xtask dynamodb seed --calendar-id <UUID> --highlighted-day 2024-06-15

# Seed a specific count
cargo xtask dynamodb seed --calendar-id <UUID> --count 50
```

**Entry Distribution**:
- ~15% multi-day events
- ~20% all-day events
- ~45% timed activities
- ~20% tasks

## Local Development

### Prerequisites

- Docker and Docker Compose

### Start Local DynamoDB

```bash
docker compose up -d
```

### Environment Variables

```bash
export AWS_ENDPOINT_URL=http://localhost:8000
export AWS_REGION=us-east-1
export AWS_ACCESS_KEY_ID=local
export AWS_SECRET_ACCESS_KEY=local
```

### Stop Local DynamoDB

```bash
# Stop (preserves data)
docker compose stop

# Stop and remove container
docker compose down

# Stop and remove everything including data
docker compose down -v
```

## Code Architecture

The `xtask/src/dynamodb/` module follows Functional Core - Imperative Shell:

```
xtask/src/dynamodb/
├── mod.rs        # CLI commands, entry point
├── error.rs      # DynamodbError enum
├── config.rs     # TableConfig types (Functional Core)
├── planning.rs   # Pure plan calculation (Functional Core)
├── client.rs     # AWS SDK client (Imperative Shell)
├── deploy.rs     # Table operations (Imperative Shell)
└── seed.rs       # Mock data generation
```

### Functional Core (Pure)

- `config.rs`: `TableConfig`, `GsiConfig`, `KeyAttribute` - pure data types
- `planning.rs`: `calculate_deploy_plan()`, `calculate_destroy_plan()` - pure functions

### Imperative Shell (I/O)

- `client.rs`: AWS SDK client creation, `get_table_state()`
- `deploy.rs`: `execute_deploy_plan()`, `execute_destroy_plan()`
- `seed.rs`: `seed_entries()` - batch writes to DynamoDB

## AWS SDK Configuration

The client auto-detects configuration from environment:

| Variable | Purpose | Default |
|----------|---------|---------|
| `AWS_ENDPOINT_URL` | Custom endpoint (for local) | None (uses AWS) |
| `AWS_REGION` | AWS region | `us-east-1` |
| `AWS_PROFILE` | AWS profile | Default profile |
| `AWS_ACCESS_KEY_ID` | Access key | From profile |
| `AWS_SECRET_ACCESS_KEY` | Secret key | From profile |

## Entity Types

The schema supports these entities:

| Entity | Description |
|--------|-------------|
| `User` | User accounts with name and email |
| `Calendar` | Named calendar with color |
| `CalendarMembership` | User-calendar link with role |
| `CalendarEntry` | Events, tasks in a calendar |

### Calendar Roles

| Role | Read | Write | Administer |
|------|------|-------|------------|
| `owner` | Yes | Yes | Yes |
| `writer` | Yes | Yes | No |
| `reader` | Yes | No | No |
