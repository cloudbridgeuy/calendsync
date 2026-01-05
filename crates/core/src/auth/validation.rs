/// Validates return_to URL to prevent open redirects.
///
/// Returns `Some(url)` if the URL is a valid relative path, `None` otherwise.
///
/// # Security
///
/// This function prevents open redirect attacks by ensuring URLs:
/// - Start with a single `/` (relative path)
/// - Do not start with `//` (protocol-relative URLs like `//evil.com`)
/// - Do not contain control characters (potential injection)
/// - Do not contain `://` (absolute URLs with schemes like `https://`, `javascript:`)
///
/// # Examples
///
/// ```
/// use calendsync_core::auth::validate_return_to;
///
/// // Valid relative paths
/// assert_eq!(validate_return_to("/calendar/123"), Some("/calendar/123"));
/// assert_eq!(validate_return_to("/"), Some("/"));
///
/// // Invalid: protocol-relative URL
/// assert_eq!(validate_return_to("//evil.com"), None);
///
/// // Invalid: absolute URL
/// assert_eq!(validate_return_to("https://evil.com"), None);
/// ```
pub fn validate_return_to(url: &str) -> Option<&str> {
    // Must start with /
    if !url.starts_with('/') {
        return None;
    }

    // Reject protocol-relative URLs (//evil.com)
    if url.starts_with("//") {
        return None;
    }

    // Reject control characters (potential injection attacks)
    if url.chars().any(|c| c.is_control()) {
        return None;
    }

    // Reject URLs with schemes (https://, javascript:, etc.)
    if url.contains("://") {
        return None;
    }

    Some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Valid paths tests

    #[test]
    fn accepts_simple_relative_path() {
        assert_eq!(validate_return_to("/calendar/123"), Some("/calendar/123"));
    }

    #[test]
    fn accepts_root_path() {
        assert_eq!(validate_return_to("/"), Some("/"));
    }

    #[test]
    fn accepts_path_with_query_string() {
        assert_eq!(validate_return_to("/search?q=test"), Some("/search?q=test"));
    }

    #[test]
    fn accepts_path_with_fragment() {
        assert_eq!(validate_return_to("/page#section"), Some("/page#section"));
    }

    #[test]
    fn accepts_complex_path() {
        let path = "/calendar/abc-123/entries?date=2024-01-01&view=week#today";
        assert_eq!(validate_return_to(path), Some(path));
    }

    #[test]
    fn accepts_path_with_encoded_characters() {
        assert_eq!(
            validate_return_to("/path%20with%20spaces"),
            Some("/path%20with%20spaces")
        );
    }

    // Absolute URLs tests

    #[test]
    fn rejects_https_url() {
        assert_eq!(validate_return_to("https://evil.com"), None);
    }

    #[test]
    fn rejects_http_url() {
        assert_eq!(validate_return_to("http://evil.com/path"), None);
    }

    #[test]
    fn rejects_ftp_url() {
        assert_eq!(validate_return_to("ftp://evil.com"), None);
    }

    #[test]
    fn rejects_url_without_leading_slash() {
        assert_eq!(validate_return_to("calendar/123"), None);
    }

    #[test]
    fn rejects_empty_string() {
        assert_eq!(validate_return_to(""), None);
    }

    // Protocol-relative URLs tests

    #[test]
    fn rejects_protocol_relative_url() {
        assert_eq!(validate_return_to("//evil.com"), None);
    }

    #[test]
    fn rejects_protocol_relative_with_path() {
        assert_eq!(validate_return_to("//evil.com/path"), None);
    }

    #[test]
    fn rejects_protocol_relative_with_credentials() {
        assert_eq!(validate_return_to("//user:pass@evil.com"), None);
    }

    // JavaScript and data URLs tests

    #[test]
    fn rejects_javascript_url() {
        assert_eq!(validate_return_to("javascript:alert(1)"), None);
    }

    #[test]
    fn rejects_javascript_url_uppercase() {
        // Note: This is rejected because it doesn't start with /
        assert_eq!(validate_return_to("JAVASCRIPT:alert(1)"), None);
    }

    #[test]
    fn rejects_data_url() {
        assert_eq!(validate_return_to("data:text/html,<script>"), None);
    }

    // Control characters tests

    #[test]
    fn rejects_newline_in_path() {
        assert_eq!(validate_return_to("/path\n/evil"), None);
    }

    #[test]
    fn rejects_carriage_return_in_path() {
        assert_eq!(validate_return_to("/path\r/evil"), None);
    }

    #[test]
    fn rejects_tab_in_path() {
        assert_eq!(validate_return_to("/path\t/evil"), None);
    }

    #[test]
    fn rejects_null_byte_in_path() {
        assert_eq!(validate_return_to("/path\0/evil"), None);
    }

    #[test]
    fn rejects_escape_character_in_path() {
        assert_eq!(validate_return_to("/path\x1b/evil"), None);
    }

    #[test]
    fn rejects_backspace_in_path() {
        assert_eq!(validate_return_to("/path\x08/evil"), None);
    }

    // Edge cases

    #[test]
    fn rejects_scheme_embedded_in_path() {
        // Path that tries to embed a scheme - rejected due to ://
        assert_eq!(validate_return_to("/redirect?url=https://evil.com"), None);
    }

    #[test]
    fn accepts_colon_without_double_slash() {
        // Colon alone is fine (e.g., port numbers in query strings)
        assert_eq!(
            validate_return_to("/proxy?host=localhost:8080"),
            Some("/proxy?host=localhost:8080")
        );
    }

    #[test]
    fn accepts_double_slash_in_middle_of_path() {
        // Double slash in middle is acceptable (though unusual)
        assert_eq!(validate_return_to("/a//b"), Some("/a//b"));
    }

    #[test]
    fn accepts_path_with_at_symbol() {
        // @ symbol is fine in paths
        assert_eq!(validate_return_to("/user@domain"), Some("/user@domain"));
    }
}
