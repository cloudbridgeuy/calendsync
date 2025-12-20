mod error;
mod keys;
mod patterns;
mod serialization;
mod traits;

pub use error::{CacheError, Result};
pub use keys::{
    calendar_channel, calendar_entries_key, calendar_entries_pattern, calendar_key,
    calendar_tracking_key, entry_key, extract_calendar_id_from_key,
    extract_calendar_id_from_pattern, is_calendar_entry_key, is_calendar_metadata_key, user_key,
};
pub use patterns::pattern_matches;
pub use serialization::{
    deserialize_calendar, deserialize_entries, deserialize_entry, serialize_calendar,
    serialize_entries, serialize_entry, SerializationError,
};
pub use traits::{Cache, CachePubSub, FullCache};
