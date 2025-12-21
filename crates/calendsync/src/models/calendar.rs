use serde::{Deserialize, Deserializer};

/// Deserialize an optional string, treating empty strings as None.
fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    Ok(s.filter(|s| !s.trim().is_empty()))
}

/// Request payload for creating a new calendar.
#[derive(Debug, Deserialize)]
pub struct CreateCalendar {
    pub name: String,
    pub color: String,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub description: Option<String>,
}
