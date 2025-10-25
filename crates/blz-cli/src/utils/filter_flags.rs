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
/// ```ignore
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
        None => FilterFlags::default(),              // All disabled
        Some(s) if s == "all" => FilterFlags::all(), // All enabled
        Some(s) => {
            let mut flags = FilterFlags::default();
            for name in s.split(',') {
                match name.trim() {
                    "lang" | "language" => flags.language = true,
                    // Future filter types can be added here:
                    // "deprecated" => flags.deprecated = true,
                    // "beta" => flags.beta = true,
                    unknown if !unknown.is_empty() => {
                        eprintln!("Warning: unknown filter '{unknown}'");
                    },
                    _ => {}, // Skip empty strings from trailing commas
                }
            }
            flags
        },
    }
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
}
