//! Content filter flag parsing and management
//!
//! This module provides extensible parsing of the `--filter` flag for content filtering options.
//! Currently supports language filtering with a design that allows easy addition of future filters.

/// Configuration for content filters
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterFlags {
    /// Enable language filtering (remove non-English content)
    pub language: bool,
    // Future fields can be added here:
    // pub deprecated: bool,
    // pub beta: bool,
}

impl FilterFlags {
    /// Create a new `FilterFlags` with all filters enabled
    #[must_use]
    pub const fn all() -> Self {
        Self {
            language: true,
            // Future: deprecated: true,
            // Future: beta: true,
        }
    }

    /// Check if any filters are enabled
    #[must_use]
    pub const fn any_enabled(&self) -> bool {
        self.language
        // Future: || self.deprecated || self.beta
    }
}

/// Parse filter flag values into a `FilterFlags` struct
///
/// Supports multiple input formats:
/// - `None`: All filters disabled (default)
/// - `Some("all")`: All filters enabled
/// - `Some("lang")` or `Some("language")`: Only language filter enabled
/// - `Some("lang,future")`: Comma-separated list (warns on unknown filters)
///
/// # Examples
///
/// ```rust,ignore
/// use blz_cli::utils::filter_flags::parse_filter_flags;
///
/// // No flag provided - all disabled
/// let flags = parse_filter_flags(None);
/// assert!(!flags.language);
///
/// // --filter (no value) - all enabled
/// let all_str = "all".to_string();
/// let flags = parse_filter_flags(Some(&all_str));
/// assert!(flags.language);
///
/// // --filter lang - only language enabled
/// let lang_str = "lang".to_string();
/// let flags = parse_filter_flags(Some(&lang_str));
/// assert!(flags.language);
/// ```
#[must_use]
pub fn parse_filter_flags(filter: Option<&String>) -> FilterFlags {
    match filter {
        None => FilterFlags::default(),
        Some(s) => {
            let normalized = s.trim();
            if normalized.is_empty() {
                return FilterFlags::default();
            }

            if normalized.eq_ignore_ascii_case("all") {
                return FilterFlags::all();
            }

            let mut flags = FilterFlags::default();
            for name in normalized.split(',') {
                let original = name.trim();
                if original.is_empty() {
                    continue;
                }
                let token = original.to_ascii_lowercase();
                match token.as_str() {
                    "lang" | "language" => flags.language = true,
                    // Future filter types can be added here:
                    // "deprecated" => flags.deprecated = true,
                    // "beta" => flags.beta = true,
                    _ => {
                        eprintln!("Warning: unknown filter '{original}'");
                    },
                }
            }
            flags
        },
    }
}

/// Detect whether the provided value contains only known filter tokens.
#[must_use]
pub fn is_known_filter_expression(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.eq_ignore_ascii_case("all") {
        return true;
    }

    let mut found = false;
    for token in trimmed.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let normalized = token.to_ascii_lowercase();
        match normalized.as_str() {
            "lang" | "language" => {
                found = true;
            },
            _ => return false,
        }
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_none() {
        let flags = parse_filter_flags(None);
        assert!(!flags.language);
        assert!(!flags.any_enabled());
    }

    #[test]
    fn test_parse_all() {
        let all_str = "all".to_string();
        let flags = parse_filter_flags(Some(&all_str));
        assert!(flags.language);
        assert!(flags.any_enabled());
    }

    #[test]
    fn test_parse_lang() {
        let lang_str = "lang".to_string();
        let flags = parse_filter_flags(Some(&lang_str));
        assert!(flags.language);
        assert!(flags.any_enabled());
    }

    #[test]
    fn test_parse_language() {
        let language_str = "language".to_string();
        let flags = parse_filter_flags(Some(&language_str));
        assert!(flags.language);
        assert!(flags.any_enabled());
    }

    #[test]
    fn test_parse_multiple() {
        let multi_str = "lang,future".to_string();
        let flags = parse_filter_flags(Some(&multi_str));
        assert!(flags.language);
        // Future filters would be checked here
    }

    #[test]
    fn test_parse_empty_string() {
        let empty_str = String::new();
        let flags = parse_filter_flags(Some(&empty_str));
        assert!(!flags.language);
        assert!(!flags.any_enabled());
    }

    #[test]
    fn test_parse_whitespace() {
        let ws_str = " lang ".to_string();
        let flags = parse_filter_flags(Some(&ws_str));
        assert!(flags.language);
    }

    #[test]
    fn test_parse_trailing_comma() {
        let trailing_str = "lang,".to_string();
        let flags = parse_filter_flags(Some(&trailing_str));
        assert!(flags.language);
    }

    #[test]
    fn test_filter_flags_all() {
        let flags = FilterFlags::all();
        assert!(flags.language);
        assert!(flags.any_enabled());
    }

    #[test]
    fn test_filter_flags_default() {
        let flags = FilterFlags::default();
        assert!(!flags.language);
        assert!(!flags.any_enabled());
    }

    #[test]
    fn test_is_known_filter_expression_all() {
        assert!(is_known_filter_expression("ALL"));
    }

    #[test]
    fn test_is_known_filter_expression_unknown() {
        assert!(!is_known_filter_expression("react"));
        assert!(!is_known_filter_expression("lang,unknown"));
    }
}
