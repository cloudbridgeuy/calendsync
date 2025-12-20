//! Pure pattern matching functions for cache keys.
//!
//! These functions support glob-style patterns with `*` wildcard
//! that matches any sequence of characters.

/// Checks if a cache key matches a glob pattern.
///
/// The pattern supports `*` as a wildcard that matches any sequence
/// of characters (including empty strings).
///
/// # Examples
///
/// ```
/// use calendsync_core::cache::pattern_matches;
///
/// // Exact match
/// assert!(pattern_matches("user:123", "user:123"));
///
/// // Wildcard at end
/// assert!(pattern_matches("calendar:123:entries:*", "calendar:123:entries:2024-01-01:2024-01-31"));
///
/// // Wildcard in middle
/// assert!(pattern_matches("calendar:*:entries:*", "calendar:456:entries:2024-02-01:2024-02-28"));
///
/// // No match
/// assert!(!pattern_matches("calendar:123:*", "user:456"));
/// ```
pub fn pattern_matches(pattern: &str, key: &str) -> bool {
    // Handle edge cases
    if pattern.is_empty() {
        return key.is_empty();
    }

    if pattern == "*" {
        return true;
    }

    // Split pattern by '*' to get segments
    let segments: Vec<&str> = pattern.split('*').collect();

    // If no wildcards, require exact match
    if segments.len() == 1 {
        return pattern == key;
    }

    let mut remaining = key;
    let starts_with_wildcard = pattern.starts_with('*');
    let ends_with_wildcard = pattern.ends_with('*');

    for (i, segment) in segments.iter().enumerate() {
        // Skip empty segments (from adjacent wildcards or leading/trailing *)
        if segment.is_empty() {
            continue;
        }

        let is_first = i == 0;
        let is_last = i == segments.len() - 1;

        if is_first && !starts_with_wildcard {
            // First segment must be at the start of the key
            if !remaining.starts_with(segment) {
                return false;
            }
            remaining = &remaining[segment.len()..];
        } else if is_last && !ends_with_wildcard {
            // Last segment must be at the end of the key
            if !remaining.ends_with(segment) {
                return false;
            }
            // No need to update remaining, we're done
        } else {
            // Middle segment (or first with leading *, or last with trailing *)
            // Just needs to be found somewhere in remaining
            match remaining.find(segment) {
                Some(pos) => {
                    remaining = &remaining[pos + segment.len()..];
                }
                None => return false,
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(pattern_matches("user:123", "user:123"));
        assert!(pattern_matches(
            "calendar:abc:entries",
            "calendar:abc:entries"
        ));
        assert!(!pattern_matches("user:123", "user:456"));
    }

    #[test]
    fn test_wildcard_at_end() {
        assert!(pattern_matches(
            "calendar:123:entries:*",
            "calendar:123:entries:2024-01-01:2024-01-31"
        ));
        assert!(pattern_matches(
            "calendar:123:entries:*",
            "calendar:123:entries:"
        ));
        assert!(pattern_matches("user:*", "user:anything-goes-here"));
        assert!(!pattern_matches(
            "calendar:123:entries:*",
            "calendar:456:entries:2024-01-01"
        ));
    }

    #[test]
    fn test_wildcard_at_start() {
        assert!(pattern_matches(
            "*:entries:2024-01-01",
            "calendar:123:entries:2024-01-01"
        ));
        assert!(pattern_matches(
            "*:entries:2024-01-01",
            "anything:entries:2024-01-01"
        ));
        assert!(!pattern_matches(
            "*:entries:2024-01-01",
            "calendar:123:entries:2024-01-02"
        ));
    }

    #[test]
    fn test_wildcard_in_middle() {
        assert!(pattern_matches(
            "calendar:*:entries",
            "calendar:123:entries"
        ));
        assert!(pattern_matches(
            "calendar:*:entries",
            "calendar:abc-def-ghi:entries"
        ));
        assert!(!pattern_matches("calendar:*:entries", "calendar:123:other"));
        assert!(!pattern_matches("calendar:*:entries", "user:123:entries"));
    }

    #[test]
    fn test_multiple_wildcards() {
        assert!(pattern_matches(
            "calendar:*:entries:*",
            "calendar:123:entries:2024-01-01:2024-01-31"
        ));
        assert!(pattern_matches("*:*:*", "a:b:c"));
        assert!(pattern_matches("*:middle:*", "start:middle:end"));
        assert!(!pattern_matches("*:middle:*", "start:other:end"));
    }

    #[test]
    fn test_wildcard_only() {
        assert!(pattern_matches("*", "anything"));
        assert!(pattern_matches("*", ""));
        assert!(pattern_matches(
            "*",
            "calendar:123:entries:2024-01-01:2024-01-31"
        ));
    }

    #[test]
    fn test_no_match() {
        assert!(!pattern_matches("calendar:123:*", "user:456"));
        assert!(!pattern_matches("user:*", "calendar:123"));
        assert!(!pattern_matches("prefix:*:suffix", "prefix:middle:other"));
    }

    #[test]
    fn test_empty_pattern() {
        assert!(pattern_matches("", ""));
        assert!(!pattern_matches("", "non-empty"));
    }

    #[test]
    fn test_empty_key() {
        assert!(pattern_matches("", ""));
        assert!(pattern_matches("*", ""));
        assert!(!pattern_matches("non-empty", ""));
        assert!(!pattern_matches("prefix:*", ""));
    }

    #[test]
    fn test_adjacent_wildcards() {
        // Adjacent wildcards should work like a single wildcard
        assert!(pattern_matches(
            "calendar:**:entries",
            "calendar:123:entries"
        ));
        assert!(pattern_matches("**", "anything"));
        assert!(pattern_matches("prefix:**:suffix", "prefix:a:b:c:suffix"));
    }

    #[test]
    fn test_real_cache_keys() {
        // Test with actual patterns from keys.rs
        let calendar_id = "00000000-0000-0000-0000-000000000000";

        // calendar_entries_pattern matches calendar_entries_key
        let pattern = format!("calendar:{}:entries:*", calendar_id);
        let key = format!("calendar:{}:entries:2024-01-01:2024-01-31", calendar_id);
        assert!(pattern_matches(&pattern, &key));

        // Pattern should not match different calendar
        let other_key =
            "calendar:11111111-1111-1111-1111-111111111111:entries:2024-01-01:2024-01-31";
        assert!(!pattern_matches(&pattern, other_key));

        // Pattern should not match entry keys
        let entry_key = format!("entry:{}", calendar_id);
        assert!(!pattern_matches(&pattern, &entry_key));

        // User key patterns
        assert!(pattern_matches(
            "user:*",
            "user:00000000-0000-0000-0000-000000000000"
        ));
        assert!(!pattern_matches(
            "user:*",
            "calendar:00000000-0000-0000-0000-000000000000"
        ));

        // Channel patterns
        assert!(pattern_matches(
            "channel:calendar:*",
            "channel:calendar:00000000-0000-0000-0000-000000000000"
        ));
    }
}
