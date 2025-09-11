//! Utility functions for safe string operations

/// Safely truncate a string at a valid UTF-8 boundary
///
/// This function ensures that the string is truncated at a valid character boundary,
/// preventing panics when dealing with multi-byte UTF-8 characters.
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_bytes` - Maximum number of bytes to keep
///
/// # Returns
/// A string slice that is at most `max_bytes` long and ends at a valid UTF-8 boundary
///
/// # Examples
/// ```
/// use blz_core::utils::safe_truncate;
///
/// let text = "Hello 世界";
/// assert_eq!(safe_truncate(text, 5), "Hello");
/// assert_eq!(safe_truncate(text, 8), "Hello "); // Won't cut in middle of 世
/// ```
pub fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }

    // Find the last character whose start position is before max_bytes
    // and whose end position is within max_bytes
    let mut last_valid_end = 0;

    for (i, c) in s.char_indices() {
        let char_end = i + c.len_utf8();
        if char_end <= max_bytes {
            last_valid_end = char_end;
        } else {
            break;
        }
    }

    &s[..last_valid_end]
}

/// Find a safe UTF-8 boundary near a given byte position
///
/// # Arguments
/// * `s` - The string to search in
/// * `target_pos` - The target byte position
///
/// # Returns
/// The nearest valid UTF-8 boundary at or before `target_pos`
pub fn find_char_boundary(s: &str, target_pos: usize) -> usize {
    if target_pos >= s.len() {
        return s.len();
    }

    if target_pos == 0 {
        return 0;
    }

    // Find the start of the character at or before target_pos
    s.char_indices()
        .take_while(|(i, _)| *i <= target_pos)
        .last()
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Extract a snippet around a match position with safe UTF-8 boundaries
///
/// # Arguments
/// * `content` - The full content string
/// * `match_pos` - Byte position of the match
/// * `match_len` - Length of the match in bytes
/// * `context_before` - Number of bytes of context to show before the match
/// * `context_after` - Number of bytes of context to show after the match
///
/// # Returns
/// A tuple of (snippet, needs_prefix_ellipsis, needs_suffix_ellipsis)
pub fn extract_snippet_safe(
    content: &str,
    match_pos: usize,
    match_len: usize,
    context_before: usize,
    context_after: usize,
) -> (String, bool, bool) {
    let byte_start = match_pos.saturating_sub(context_before);
    let byte_end = (match_pos + match_len + context_after).min(content.len());

    // Find safe UTF-8 boundaries
    let start = if byte_start == 0 {
        0
    } else {
        // Find the last character boundary at or before byte_start
        find_char_boundary(content, byte_start)
    };

    let end = if byte_end >= content.len() {
        content.len()
    } else {
        // Find the character boundary just after byte_end (to include the character at byte_end)
        content.char_indices()
            .find(|(i, _)| *i > byte_end)
            .map(|(i, _)| i)
            .unwrap_or(content.len())
    };

    let snippet = content[start..end].to_string();
    let needs_prefix = start > 0;
    let needs_suffix = end < content.len();

    (snippet, needs_prefix, needs_suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_truncate_ascii() {
        let text = "Hello, World!";
        assert_eq!(safe_truncate(text, 5), "Hello");
        assert_eq!(safe_truncate(text, 100), "Hello, World!");
        assert_eq!(safe_truncate(text, 0), "");
    }

    #[test]
    fn test_safe_truncate_unicode() {
        let text = "Hello 世界";
        assert_eq!(safe_truncate(text, 5), "Hello");
        assert_eq!(safe_truncate(text, 6), "Hello ");
        assert_eq!(safe_truncate(text, 7), "Hello "); // Won't cut 世 in half
        assert_eq!(safe_truncate(text, 8), "Hello "); // Still won't include partial 世
        assert_eq!(safe_truncate(text, 9), "Hello 世");
    }

    #[test]
    fn test_safe_truncate_emoji() {
        let text = "Hi 👋 there";
        assert_eq!(safe_truncate(text, 2), "Hi");
        assert_eq!(safe_truncate(text, 3), "Hi ");
        assert_eq!(safe_truncate(text, 4), "Hi "); // Won't cut emoji
        assert_eq!(safe_truncate(text, 7), "Hi 👋");
    }

    #[test]
    fn test_find_char_boundary() {
        let text = "Hello 世界";
        assert_eq!(find_char_boundary(text, 0), 0);
        assert_eq!(find_char_boundary(text, 5), 5);
        assert_eq!(find_char_boundary(text, 7), 6); // Middle of 世
        assert_eq!(find_char_boundary(text, 100), text.len());
    }

    #[test]
    fn test_extract_snippet_safe() {
        let content = "The quick brown fox jumps over the lazy dog";
        let (snippet, prefix, suffix) = extract_snippet_safe(content, 10, 5, 6, 5); // "brown" starting at pos 10
        
        // With context_before=6 and context_after=5:
        // Start: 10 - 6 = 4 (position of 'q')
        // End: 10 + 5 + 5 = 20 (position of 'j')
        // So we should get "quick brown fox j"
        assert_eq!(snippet, "quick brown fox j");
        assert!(prefix);
        assert!(suffix);
    }

    #[test]
    fn test_extract_snippet_safe_unicode() {
        let content = "Hello 世界, this is a test";
        let (snippet, prefix, suffix) = extract_snippet_safe(content, 6, 6, 3, 3); // "世界"
        
        assert!(snippet.contains("世界"));
        assert!(prefix);
        assert!(suffix);
    }
}
