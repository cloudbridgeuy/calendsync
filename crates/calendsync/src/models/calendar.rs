use calendsync_core::serde::deserialize_optional_string;
use serde::Deserialize;

/// Request payload for creating a new calendar.
#[derive(Debug, Deserialize)]
pub struct CreateCalendar {
    pub name: String,
    pub color: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
}

/// Request payload for updating a calendar.
#[derive(Debug, Deserialize)]
pub struct UpdateCalendar {
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub color: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
}
