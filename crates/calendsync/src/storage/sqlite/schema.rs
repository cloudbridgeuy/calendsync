//! SQLite schema definitions and SQL query constants.
//!
//! This module contains all SQL statements used by the SQLite repository,
//! following the Functional Core pattern - pure data, no I/O.

/// SQL statement to create all tables.
pub const CREATE_TABLES: &str = r#"
-- Users table
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Calendars table
CREATE TABLE IF NOT EXISTS calendars (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    color TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Calendar entries table
CREATE TABLE IF NOT EXISTS entries (
    id TEXT PRIMARY KEY,
    calendar_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    location TEXT,
    kind TEXT NOT NULL,
    date TEXT NOT NULL,
    color TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (calendar_id) REFERENCES calendars(id) ON DELETE CASCADE
);

-- Calendar memberships table
CREATE TABLE IF NOT EXISTS memberships (
    calendar_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (calendar_id, user_id),
    FOREIGN KEY (calendar_id) REFERENCES calendars(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_entries_calendar_id ON entries(calendar_id);
CREATE INDEX IF NOT EXISTS idx_entries_date ON entries(date);
CREATE INDEX IF NOT EXISTS idx_entries_calendar_date ON entries(calendar_id, date);
CREATE INDEX IF NOT EXISTS idx_memberships_user_id ON memberships(user_id);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
"#;

// User queries
pub const INSERT_USER: &str = r#"
INSERT INTO users (id, name, email, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5)
"#;

pub const SELECT_USER_BY_ID: &str = r#"
SELECT id, name, email, created_at, updated_at
FROM users
WHERE id = ?1
"#;

pub const SELECT_USER_BY_EMAIL: &str = r#"
SELECT id, name, email, created_at, updated_at
FROM users
WHERE email = ?1
"#;

pub const UPDATE_USER: &str = r#"
UPDATE users
SET name = ?2, email = ?3, updated_at = ?4
WHERE id = ?1
"#;

// Calendar queries
pub const INSERT_CALENDAR: &str = r#"
INSERT INTO calendars (id, name, color, description, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6)
"#;

pub const SELECT_CALENDAR_BY_ID: &str = r#"
SELECT id, name, color, description, created_at, updated_at
FROM calendars
WHERE id = ?1
"#;

pub const UPDATE_CALENDAR: &str = r#"
UPDATE calendars
SET name = ?2, color = ?3, description = ?4, updated_at = ?5
WHERE id = ?1
"#;

pub const DELETE_CALENDAR: &str = r#"
DELETE FROM calendars
WHERE id = ?1
"#;

// Entry queries
pub const INSERT_ENTRY: &str = r#"
INSERT INTO entries (id, calendar_id, title, description, location, kind, date, color, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
"#;

pub const SELECT_ENTRY_BY_ID: &str = r#"
SELECT id, calendar_id, title, description, location, kind, date, color, created_at, updated_at
FROM entries
WHERE id = ?1
"#;

pub const SELECT_ENTRIES_BY_CALENDAR_AND_DATE_RANGE: &str = r#"
SELECT id, calendar_id, title, description, location, kind, date, color, created_at, updated_at
FROM entries
WHERE calendar_id = ?1 AND date >= ?2 AND date <= ?3
ORDER BY date ASC
"#;

pub const UPDATE_ENTRY: &str = r#"
UPDATE entries
SET title = ?2, description = ?3, location = ?4, kind = ?5, date = ?6, color = ?7, updated_at = ?8
WHERE id = ?1
"#;

pub const DELETE_ENTRY: &str = r#"
DELETE FROM entries
WHERE id = ?1
"#;

// Membership queries
pub const INSERT_MEMBERSHIP: &str = r#"
INSERT INTO memberships (calendar_id, user_id, role, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5)
"#;

pub const SELECT_MEMBERSHIP: &str = r#"
SELECT calendar_id, user_id, role, created_at, updated_at
FROM memberships
WHERE calendar_id = ?1 AND user_id = ?2
"#;

pub const SELECT_CALENDARS_FOR_USER: &str = r#"
SELECT c.id, c.name, c.color, c.description, c.created_at, c.updated_at, m.role
FROM calendars c
INNER JOIN memberships m ON c.id = m.calendar_id
WHERE m.user_id = ?1
"#;

pub const SELECT_USERS_FOR_CALENDAR: &str = r#"
SELECT u.id, u.name, u.email, u.created_at, u.updated_at, m.role
FROM users u
INNER JOIN memberships m ON u.id = m.user_id
WHERE m.calendar_id = ?1
"#;

pub const DELETE_MEMBERSHIP: &str = r#"
DELETE FROM memberships
WHERE calendar_id = ?1 AND user_id = ?2
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tables_is_valid_sql() {
        // Verify the SQL contains expected table names
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS users"));
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS calendars"));
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS entries"));
        assert!(CREATE_TABLES.contains("CREATE TABLE IF NOT EXISTS memberships"));
    }

    #[test]
    fn test_queries_contain_expected_keywords() {
        // User queries
        assert!(INSERT_USER.contains("INSERT"));
        assert!(SELECT_USER_BY_ID.contains("SELECT"));
        assert!(SELECT_USER_BY_EMAIL.contains("email"));
        assert!(UPDATE_USER.contains("UPDATE"));

        // Calendar queries
        assert!(INSERT_CALENDAR.contains("INSERT"));
        assert!(SELECT_CALENDAR_BY_ID.contains("SELECT"));
        assert!(UPDATE_CALENDAR.contains("UPDATE"));
        assert!(DELETE_CALENDAR.contains("DELETE"));

        // Entry queries
        assert!(INSERT_ENTRY.contains("INSERT"));
        assert!(SELECT_ENTRY_BY_ID.contains("SELECT"));
        assert!(SELECT_ENTRIES_BY_CALENDAR_AND_DATE_RANGE.contains("date >="));
        assert!(UPDATE_ENTRY.contains("UPDATE"));
        assert!(DELETE_ENTRY.contains("DELETE"));

        // Membership queries
        assert!(INSERT_MEMBERSHIP.contains("INSERT"));
        assert!(SELECT_MEMBERSHIP.contains("SELECT"));
        assert!(SELECT_CALENDARS_FOR_USER.contains("JOIN"));
        assert!(SELECT_USERS_FOR_CALENDAR.contains("JOIN"));
        assert!(DELETE_MEMBERSHIP.contains("DELETE"));
    }
}
