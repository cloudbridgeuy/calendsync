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
        entry.title, kind_str, entry.id, entry.calendar_id, entry.date
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
