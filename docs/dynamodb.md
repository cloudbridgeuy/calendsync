# DynamoDB Schema Documentation

This document describes the DynamoDB single-table design for calendsync persistence.

## Table Overview

**Table Name**: `calendsync`

The schema uses a single-table design with composite keys, following DynamoDB best practices for related entities.

### Table Structure

| Attribute | Type | Description |
|-----------|------|-------------|
| `PK` | String | Partition Key |
| `SK` | String | Sort Key |
| `GSI1PK` | String | GSI1 Partition Key |
| `GSI1SK` | String | GSI1 Sort Key |
| `GSI2PK` | String | GSI2 Partition Key (email lookups) |
| `GSI2SK` | String | GSI2 Sort Key |
| `entityType` | String | Entity discriminator: `USER`, `CALENDAR`, `MEMBERSHIP`, `ENTRY` |

### Global Secondary Index (GSI1)

| Property | Value |
|----------|-------|
| Index Name | `GSI1` |
| Partition Key | `GSI1PK` (String) |
| Sort Key | `GSI1SK` (String) |
| Projection | `ALL` |

Used for:
- Get all calendars for a user (via memberships)
- Get all entries for a calendar within a date range

### Global Secondary Index (GSI2)

| Property | Value |
|----------|-------|
| Index Name | `GSI2` |
| Partition Key | `GSI2PK` (String) |
| Sort Key | `GSI2SK` (String) |
| Projection | `ALL` |

Used for:
- Get user by email address

### Global Secondary Index (GSI3)

| Property | Value |
|----------|-------|
| Index Name | `GSI3` |
| Partition Key | `GSI3PK` (String) |
| Sort Key | `GSI3SK` (String) |
| Projection | `ALL` |

Used for:
- Get user by OAuth provider (e.g., Google, GitHub)

### Billing Mode

PAY_PER_REQUEST (on-demand capacity)

---

## Entity Key Patterns

### User

| Attribute | Pattern | Example |
|-----------|---------|---------|
| `PK` | `USER#<user_id>` | `USER#550e8400-e29b-41d4-a716-446655440001` |
| `SK` | `USER#<user_id>` | `USER#550e8400-e29b-41d4-a716-446655440001` |
| `GSI1PK` | (not used) | - |
| `GSI1SK` | (not used) | - |
| `GSI2PK` | `EMAIL#<email>` | `EMAIL#john@example.com` |
| `GSI2SK` | `USER#<user_id>` | `USER#550e8400-e29b-41d4-a716-446655440001` |
| `GSI3PK` | `PROV#<provider>#<subject>` | `PROV#google#123456789` (optional) |
| `GSI3SK` | `USER#<user_id>` | `USER#550e8400-e29b-41d4-a716-446655440001` (optional) |

**Note:** GSI3 keys are only present when the user has OAuth provider information (`provider` and `providerSubject` fields).

**Attributes (without OAuth)**:
```json
{
  "PK": "USER#550e8400-e29b-41d4-a716-446655440001",
  "SK": "USER#550e8400-e29b-41d4-a716-446655440001",
  "GSI2PK": "EMAIL#john@example.com",
  "GSI2SK": "USER#550e8400-e29b-41d4-a716-446655440001",
  "entityType": "USER",
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "name": "John Doe",
  "email": "john@example.com",
  "createdAt": "2024-01-15T10:30:00Z",
  "updatedAt": "2024-01-15T10:30:00Z"
}
```

**Attributes (with OAuth)**:
```json
{
  "PK": "USER#550e8400-e29b-41d4-a716-446655440001",
  "SK": "USER#550e8400-e29b-41d4-a716-446655440001",
  "GSI2PK": "EMAIL#john@example.com",
  "GSI2SK": "USER#550e8400-e29b-41d4-a716-446655440001",
  "GSI3PK": "PROV#google#123456789",
  "GSI3SK": "USER#550e8400-e29b-41d4-a716-446655440001",
  "entityType": "USER",
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "name": "John Doe",
  "email": "john@example.com",
  "provider": "google",
  "providerSubject": "123456789",
  "createdAt": "2024-01-15T10:30:00Z",
  "updatedAt": "2024-01-15T10:30:00Z"
}
```

### Calendar

| Attribute | Pattern | Example |
|-----------|---------|---------|
| `PK` | `CAL#<calendar_id>` | `CAL#550e8400-e29b-41d4-a716-446655440002` |
| `SK` | `CAL#<calendar_id>` | `CAL#550e8400-e29b-41d4-a716-446655440002` |
| `GSI1PK` | (not used) | - |
| `GSI1SK` | (not used) | - |

**Attributes**:
```json
{
  "PK": "CAL#550e8400-e29b-41d4-a716-446655440002",
  "SK": "CAL#550e8400-e29b-41d4-a716-446655440002",
  "entityType": "CALENDAR",
  "id": "550e8400-e29b-41d4-a716-446655440002",
  "name": "Personal",
  "color": "#3B82F6",
  "description": "My personal calendar",
  "createdAt": "2024-01-15T10:30:00Z",
  "updatedAt": "2024-01-15T10:30:00Z"
}
```

### CalendarMembership

Links users to calendars with roles (`owner`, `writer`, `reader`).

| Attribute | Pattern | Example |
|-----------|---------|---------|
| `PK` | `CAL#<calendar_id>` | `CAL#550e8400-e29b-41d4-a716-446655440002` |
| `SK` | `MEMBER#<user_id>` | `MEMBER#550e8400-e29b-41d4-a716-446655440001` |
| `GSI1PK` | `USER#<user_id>` | `USER#550e8400-e29b-41d4-a716-446655440001` |
| `GSI1SK` | `CAL#<calendar_id>` | `CAL#550e8400-e29b-41d4-a716-446655440002` |

**Attributes**:
```json
{
  "PK": "CAL#550e8400-e29b-41d4-a716-446655440002",
  "SK": "MEMBER#550e8400-e29b-41d4-a716-446655440001",
  "GSI1PK": "USER#550e8400-e29b-41d4-a716-446655440001",
  "GSI1SK": "CAL#550e8400-e29b-41d4-a716-446655440002",
  "entityType": "MEMBERSHIP",
  "calendarId": "550e8400-e29b-41d4-a716-446655440002",
  "userId": "550e8400-e29b-41d4-a716-446655440001",
  "role": "owner",
  "createdAt": "2024-01-15T10:30:00Z",
  "updatedAt": "2024-01-15T10:30:00Z"
}
```

**Roles**:
| Role | Read Entries | Write Entries | Administer Calendar |
|------|--------------|---------------|---------------------|
| `owner` | Yes | Yes | Yes (delete calendar, invite users) |
| `writer` | Yes | Yes | No |
| `reader` | Yes | No | No |

### CalendarEntry

| Attribute | Pattern | Example |
|-----------|---------|---------|
| `PK` | `ENTRY#<entry_id>` | `ENTRY#550e8400-e29b-41d4-a716-446655440003` |
| `SK` | `ENTRY#<entry_id>` | `ENTRY#550e8400-e29b-41d4-a716-446655440003` |
| `GSI1PK` | `CAL#<calendar_id>` | `CAL#550e8400-e29b-41d4-a716-446655440002` |
| `GSI1SK` | `ENTRY#<date>#<entry_id>` | `ENTRY#2024-01-15#550e8400-...` |

The `GSI1SK` format enables:
- Lexicographic sorting by date (ISO 8601 `YYYY-MM-DD`)
- Range queries with `BETWEEN` for date spans
- Uniqueness via trailing `entry_id`

**Attributes (Timed Event)**:
```json
{
  "PK": "ENTRY#550e8400-e29b-41d4-a716-446655440003",
  "SK": "ENTRY#550e8400-e29b-41d4-a716-446655440003",
  "GSI1PK": "CAL#550e8400-e29b-41d4-a716-446655440002",
  "GSI1SK": "ENTRY#2024-01-15#550e8400-e29b-41d4-a716-446655440003",
  "entityType": "ENTRY",
  "id": "550e8400-e29b-41d4-a716-446655440003",
  "calendarId": "550e8400-e29b-41d4-a716-446655440002",
  "title": "Standup Meeting",
  "description": null,
  "location": "Zoom",
  "date": "2024-01-15",
  "color": "#3B82F6",
  "kind": {
    "type": "Timed",
    "start": "09:00:00",
    "end": "09:30:00"
  },
  "createdAt": "2024-01-15T08:00:00Z",
  "updatedAt": "2024-01-15T08:00:00Z"
}
```

**Entry Kind Variants**:
```json
// MultiDay
{ "type": "MultiDay", "start": "2024-01-14", "end": "2024-01-16" }

// AllDay
{ "type": "AllDay" }

// Timed
{ "type": "Timed", "start": "09:00:00", "end": "09:30:00" }

// Task
{ "type": "Task", "completed": false }
```

---

## Access Patterns

### 1. Get User by ID

```
Query:
  TableName: calendsync
  KeyConditionExpression: PK = :pk AND SK = :sk
  ExpressionAttributeValues:
    :pk = "USER#<user_id>"
    :sk = "USER#<user_id>"
```

### 2. Get Calendar by ID

```
Query:
  TableName: calendsync
  KeyConditionExpression: PK = :pk AND SK = :sk
  ExpressionAttributeValues:
    :pk = "CAL#<calendar_id>"
    :sk = "CAL#<calendar_id>"
```

### 3. Get Entry by ID

```
Query:
  TableName: calendsync
  KeyConditionExpression: PK = :pk AND SK = :sk
  ExpressionAttributeValues:
    :pk = "ENTRY#<entry_id>"
    :sk = "ENTRY#<entry_id>"
```

### 4. Get All Users for a Calendar (with roles)

```
Query:
  TableName: calendsync
  KeyConditionExpression: PK = :pk AND begins_with(SK, :sk_prefix)
  ExpressionAttributeValues:
    :pk = "CAL#<calendar_id>"
    :sk_prefix = "MEMBER#"
```

Returns membership items with `userId` and `role`. To get full user details, perform a `BatchGetItem` on the user IDs.

### 5. Get All Calendars for a User (with roles)

```
Query:
  TableName: calendsync
  IndexName: GSI1
  KeyConditionExpression: GSI1PK = :pk AND begins_with(GSI1SK, :sk_prefix)
  ExpressionAttributeValues:
    :pk = "USER#<user_id>"
    :sk_prefix = "CAL#"
```

Returns membership items with `calendarId` and `role`. To get full calendar details, perform a `BatchGetItem`.

### 6. Get Entries for a Calendar Within a Date Range

```
Query:
  TableName: calendsync
  IndexName: GSI1
  KeyConditionExpression: GSI1PK = :pk AND GSI1SK BETWEEN :start AND :end
  ExpressionAttributeValues:
    :pk = "CAL#<calendar_id>"
    :start = "ENTRY#2024-01-01#"
    :end = "ENTRY#2024-01-31#~"
```

The `~` character (ASCII 126) ensures all entries on the end date are included.

### 7. Get User by Email

```
Query:
  TableName: calendsync
  IndexName: GSI2
  KeyConditionExpression: GSI2PK = :pk
  ExpressionAttributeValues:
    :pk = "EMAIL#john@example.com"
```

Returns the user item matching the email address. This is a unique lookup since emails are unique per user.

### 8. Get User by OAuth Provider

```
Query:
  TableName: calendsync
  IndexName: GSI3
  KeyConditionExpression: GSI3PK = :pk
  ExpressionAttributeValues:
    :pk = "PROV#google#123456789"
```

Returns the user item matching the OAuth provider and subject ID. Used during OAuth authentication to find existing users. The `provider` identifies the OAuth provider (e.g., "google", "github") and `provider_subject` is the unique identifier from that provider.

---

## Local Development Setup

### Prerequisites

- Docker and Docker Compose installed
- AWS CLI (optional, for manual testing)

### Starting Local DynamoDB

```bash
# Start the local DynamoDB container
docker compose up -d

# Verify it's running
docker compose ps
```

### Environment Variables

Set these environment variables to use local DynamoDB:

```bash
export AWS_ENDPOINT_URL=http://localhost:8000
export AWS_REGION=us-east-1
export AWS_ACCESS_KEY_ID=local
export AWS_SECRET_ACCESS_KEY=local
```

Or create a `.env.local` file:

```bash
AWS_ENDPOINT_URL=http://localhost:8000
AWS_REGION=us-east-1
AWS_ACCESS_KEY_ID=local
AWS_SECRET_ACCESS_KEY=local
```

### Deploying the Schema

```bash
# Deploy the table (creates if not exists)
cargo xtask dynamodb deploy

# Deploy with confirmation skip
cargo xtask dynamodb deploy --force

# Destroy the table
cargo xtask dynamodb deploy --destroy

# Use a custom table name
cargo xtask dynamodb deploy --table-name my-test-table
```

### Seeding Test Data

```bash
# Seed entries for a calendar (requires calendar_id)
cargo xtask dynamodb seed --calendar-id <UUID>

# Seed around a specific date
cargo xtask dynamodb seed --calendar-id <UUID> --highlighted-day 2024-06-15

# Seed a specific number of entries
cargo xtask dynamodb seed --calendar-id <UUID> --count 50
```

### Manual Verification with AWS CLI

```bash
# List tables
aws dynamodb list-tables --endpoint-url http://localhost:8000

# Describe the table
aws dynamodb describe-table --table-name calendsync --endpoint-url http://localhost:8000

# Scan all items (for debugging)
aws dynamodb scan --table-name calendsync --endpoint-url http://localhost:8000

# Query users
aws dynamodb query \
  --table-name calendsync \
  --key-condition-expression "PK = :pk" \
  --expression-attribute-values '{":pk": {"S": "USER#<user_id>"}}' \
  --endpoint-url http://localhost:8000
```

### Stopping Local DynamoDB

```bash
# Stop the container (preserves data)
docker compose stop

# Stop and remove container (preserves volume data)
docker compose down

# Stop and remove everything including data
docker compose down -v
```

---

## Data Migration Notes

### From In-Memory to DynamoDB

When migrating from the current in-memory storage:

1. **Users**: Map `User { id, name, email }` directly
2. **Calendars**: Map `Calendar { id, name, color, description }` directly
3. **Entries**: Map `CalendarEntry` with `kind` serialized as JSON
4. **Memberships**: New entity - create membership for each user-calendar relationship

### ID Format

All IDs are UUID v4 strings (e.g., `550e8400-e29b-41d4-a716-446655440001`).

### Timestamps

All timestamps use ISO 8601 format in UTC (e.g., `2024-01-15T10:30:00Z`).

---

## Design Decisions

### Why Single-Table Design?

1. **Reduced round-trips**: Related data can be fetched in fewer queries
2. **Simpler infrastructure**: One table to manage, monitor, and back up
3. **Cost efficiency**: Pay-per-request billing scales with actual usage

### Why `ALL` Projection on GSI1?

The most frequent access patterns (calendar views, user dashboards) need full entity data. Using `ALL` projection eliminates follow-up `GetItem` calls, reducing latency.

### Why Composite Sort Keys for Entries?

The `ENTRY#<date>#<id>` format enables:
- Efficient date-range queries using `BETWEEN`
- Natural chronological ordering
- Uniqueness for entries on the same date

### Multi-Day Event Handling

Multi-day events are stored once with their start date. The application expands them to display on all spanning days (matching the current in-memory behavior).
