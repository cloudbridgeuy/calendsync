//! Pretty output formatting.

use calendsync_core::calendar::{Calendar, CalendarEntry, User};

/// Format a user for display.
pub fn format_user(user: &User) -> String {
    format!("{}\n  ID: {}\n  Email: {}", user.name, user.id, user.email)
}

/// Format users for display.
pub fn format_users(users: &[User]) -> String {
    if users.is_empty() {
        return "No users found.".to_string();
    }
    let mut output = format!("USERS ({})\n", users.len());
    output.push_str(&"-".repeat(40));
    for user in users {
        output.push_str(&format!("\n{}", format_user(user)));
        output.push('\n');
    }
    output
}

/// Format a calendar for display.
pub fn format_calendar(calendar: &Calendar) -> String {
    let mut output = format!(
        "{} ({})\n  ID: {}",
        calendar.name, calendar.color, calendar.id
    );
    if let Some(desc) = &calendar.description {
        output.push_str(&format!("\n  Description: {}", desc));
    }
    output
}

/// Format calendars for display.
pub fn format_calendars(calendars: &[Calendar]) -> String {
    if calendars.is_empty() {
        return "No calendars found.".to_string();
    }
    let mut output = format!("CALENDARS ({})\n", calendars.len());
    output.push_str(&"-".repeat(40));
    for calendar in calendars {
        output.push_str(&format!("\n{}", format_calendar(calendar)));
        output.push('\n');
    }
    output
}

/// Format an entry for display.
pub fn format_entry(entry: &CalendarEntry) -> String {
    let kind_str = entry.kind.css_class();
    let mut output = format!(
        "{} [{}]\n  ID: {}\n  Calendar: {}\n  Date: {}",
        entry.title, kind_str, entry.id, entry.calendar_id, entry.start_date
    );
    if let Some(desc) = &entry.description {
        output.push_str(&format!("\n  Description: {}", desc));
    }
    if let Some(loc) = &entry.location {
        output.push_str(&format!("\n  Location: {}", loc));
    }
    if let Some(color) = &entry.color {
        output.push_str(&format!("\n  Color: {}", color));
    }
    output
}

/// Format entries for display.
pub fn format_entries(entries: &[CalendarEntry]) -> String {
    if entries.is_empty() {
        return "No entries found.".to_string();
    }
    let mut output = format!("ENTRIES ({})\n", entries.len());
    output.push_str(&"-".repeat(40));
    for entry in entries {
        output.push_str(&format!("\n{}", format_entry(entry)));
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use calendsync_core::calendar::{Calendar, CalendarEntry, EntryKind};
    use chrono::{NaiveDate, Utc};
    use uuid::Uuid;

    fn make_user(name: &str, email: &str) -> User {
        let now = Utc::now();
        User {
            id: Uuid::new_v4(),
            name: name.to_string(),
            email: email.to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn make_calendar(name: &str, color: &str) -> Calendar {
        let now = Utc::now();
        Calendar {
            id: Uuid::new_v4(),
            name: name.to_string(),
            color: color.to_string(),
            description: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_entry(title: &str, date: NaiveDate) -> CalendarEntry {
        let now = Utc::now();
        CalendarEntry {
            id: Uuid::new_v4(),
            calendar_id: Uuid::new_v4(),
            title: title.to_string(),
            description: None,
            location: None,
            start_date: date,
            end_date: date,
            color: None,
            kind: EntryKind::AllDay,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_format_user() {
        let user = make_user("Alice", "alice@example.com");
        let output = format_user(&user);

        assert!(output.contains("Alice"));
        assert!(output.contains("alice@example.com"));
        assert!(output.contains(&user.id.to_string()));
    }

    #[test]
    fn test_format_users_empty() {
        let output = format_users(&[]);
        assert_eq!(output, "No users found.");
    }

    #[test]
    fn test_format_users_multiple() {
        let users = vec![
            make_user("Alice", "alice@example.com"),
            make_user("Bob", "bob@example.com"),
        ];
        let output = format_users(&users);

        assert!(output.contains("USERS (2)"));
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
    }

    #[test]
    fn test_format_calendar() {
        let calendar = make_calendar("Work", "#3B82F6");
        let output = format_calendar(&calendar);

        assert!(output.contains("Work"));
        assert!(output.contains("#3B82F6"));
        assert!(output.contains(&calendar.id.to_string()));
    }

    #[test]
    fn test_format_calendar_with_description() {
        let mut calendar = make_calendar("Work", "#3B82F6");
        calendar.description = Some("My work calendar".to_string());
        let output = format_calendar(&calendar);

        assert!(output.contains("Description: My work calendar"));
    }

    #[test]
    fn test_format_calendars_empty() {
        let output = format_calendars(&[]);
        assert_eq!(output, "No calendars found.");
    }

    #[test]
    fn test_format_entry() {
        let entry = make_entry("Meeting", NaiveDate::from_ymd_opt(2024, 6, 15).unwrap());
        let output = format_entry(&entry);

        assert!(output.contains("Meeting"));
        assert!(output.contains("2024-06-15"));
        assert!(output.contains(&entry.id.to_string()));
    }

    #[test]
    fn test_format_entry_with_location() {
        let mut entry = make_entry("Meeting", NaiveDate::from_ymd_opt(2024, 6, 15).unwrap());
        entry.location = Some("Room 101".to_string());
        let output = format_entry(&entry);

        assert!(output.contains("Location: Room 101"));
    }

    #[test]
    fn test_format_entries_empty() {
        let output = format_entries(&[]);
        assert_eq!(output, "No entries found.");
    }

    #[test]
    fn test_format_entries_multiple() {
        let entries = vec![
            make_entry("Meeting 1", NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()),
            make_entry("Meeting 2", NaiveDate::from_ymd_opt(2024, 6, 16).unwrap()),
        ];
        let output = format_entries(&entries);

        assert!(output.contains("ENTRIES (2)"));
        assert!(output.contains("Meeting 1"));
        assert!(output.contains("Meeting 2"));
    }
}
