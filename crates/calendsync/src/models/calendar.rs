use serde::Deserialize;

use calendsync_core::calendar::Calendar;

/// Request payload for creating a new calendar.
#[derive(Debug, Deserialize)]
pub struct CreateCalendar {
    pub name: String,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_color() -> String {
    "#3B82F6".to_string() // Blue
}

impl CreateCalendar {
    /// Converts the create request into a Calendar.
    pub fn into_calendar(self) -> Calendar {
        let mut calendar = Calendar::new(self.name, self.color);
        if let Some(description) = self.description {
            calendar = calendar.with_description(description);
        }
        calendar
    }
}

/// Request payload for updating a calendar.
#[derive(Debug, Deserialize)]
pub struct UpdateCalendar {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

impl UpdateCalendar {
    /// Applies the update to an existing calendar.
    pub fn apply_to(self, calendar: &mut Calendar) {
        if let Some(name) = self.name {
            calendar.name = name;
        }
        if let Some(color) = self.color {
            calendar.color = color;
        }
        if let Some(description) = self.description {
            calendar.description = Some(description);
        }
    }
}
