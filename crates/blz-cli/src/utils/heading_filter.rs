//! Heading level filtering for search results
//!
//! This module provides types and functions for filtering search results based on
//! heading levels in markdown documents. Supports various filter syntaxes including
//! comparison operators, lists, and ranges.

use std::str::FromStr;

/// Filter for heading levels in search results
///
/// Supports multiple filter syntaxes:
/// - Exact match: `=2` or `2`
/// - Less than: `<4`
/// - Less than or equal: `<=2`
/// - Greater than: `>2`
/// - Greater than or equal: `>=3`
/// - List: `1,2,3`
/// - Range: `1-3` (inclusive)
///
/// # Examples
///
/// ```ignore
/// use blz_cli::utils::heading_filter::HeadingLevelFilter;
///
/// let filter: HeadingLevelFilter = "<=2".parse().unwrap();
/// assert!(filter.matches(1));
/// assert!(filter.matches(2));
/// assert!(!filter.matches(3));
///
/// let filter: HeadingLevelFilter = "1,2,3".parse().unwrap();
/// assert!(filter.matches(2));
/// assert!(!filter.matches(4));
///
/// let filter: HeadingLevelFilter = "1-3".parse().unwrap();
/// assert!(filter.matches(2));
/// assert!(!filter.matches(4));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadingLevelFilter {
    /// Exact level match (e.g., `=2` or `2`)
    Exact(u8),
    /// Less than level (e.g., `<4`)
    LessThan(u8),
    /// Less than or equal to level (e.g., `<=2`)
    LessThanOrEqual(u8),
    /// Greater than level (e.g., `>2`)
    GreaterThan(u8),
    /// Greater than or equal to level (e.g., `>=3`)
    GreaterThanOrEqual(u8),
    /// List of specific levels (e.g., `1,2,3`)
    List(Vec<u8>),
    /// Inclusive range of levels (e.g., `1-3`)
    Range(u8, u8),
}

impl HeadingLevelFilter {
    /// Check if a heading level matches this filter
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use blz_cli::utils::heading_filter::HeadingLevelFilter;
    ///
    /// let filter = HeadingLevelFilter::LessThanOrEqual(2);
    /// assert!(filter.matches(1));
    /// assert!(filter.matches(2));
    /// assert!(!filter.matches(3));
    /// ```
    #[must_use]
    pub fn matches(&self, level: u8) -> bool {
        match self {
            Self::Exact(n) => level == *n,
            Self::LessThan(n) => level < *n,
            Self::LessThanOrEqual(n) => level <= *n,
            Self::GreaterThan(n) => level > *n,
            Self::GreaterThanOrEqual(n) => level >= *n,
            Self::List(levels) => levels.contains(&level),
            Self::Range(start, end) => level >= *start && level <= *end,
        }
    }
}

impl FromStr for HeadingLevelFilter {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Check for comparison operators
        if let Some(num_str) = s.strip_prefix("<=") {
            let level = parse_level(num_str)?;
            return Ok(Self::LessThanOrEqual(level));
        }

        if let Some(num_str) = s.strip_prefix(">=") {
            let level = parse_level(num_str)?;
            return Ok(Self::GreaterThanOrEqual(level));
        }

        if let Some(num_str) = s.strip_prefix('<') {
            let level = parse_level(num_str)?;
            return Ok(Self::LessThan(level));
        }

        if let Some(num_str) = s.strip_prefix('>') {
            let level = parse_level(num_str)?;
            return Ok(Self::GreaterThan(level));
        }

        if let Some(num_str) = s.strip_prefix('=') {
            let level = parse_level(num_str)?;
            return Ok(Self::Exact(level));
        }

        // Check for list syntax (comma-separated)
        if s.contains(',') {
            let levels: Result<Vec<u8>, String> =
                s.split(',').map(|part| parse_level(part.trim())).collect();
            let levels = levels?;

            if levels.is_empty() {
                return Err("List cannot be empty".to_string());
            }

            return Ok(Self::List(levels));
        }

        // Check for range syntax (dash-separated)
        if s.contains('-') {
            let parts: Vec<&str> = s.split('-').collect();
            if parts.len() != 2 {
                return Err(format!(
                    "Invalid range syntax: '{s}'. Expected format: '1-3'"
                ));
            }

            let start = parse_level(parts[0].trim())?;
            let end = parse_level(parts[1].trim())?;

            if start > end {
                return Err(format!("Invalid range: start ({start}) > end ({end})"));
            }

            return Ok(Self::Range(start, end));
        }

        // Plain number is treated as exact match
        let level = parse_level(s)?;
        Ok(Self::Exact(level))
    }
}

/// Parse a heading level from a string
///
/// Validates that the level is between 1 and 6 (valid markdown heading levels)
fn parse_level(s: &str) -> Result<u8, String> {
    let s = s.trim();

    let level = s
        .parse::<u8>()
        .map_err(|_| format!("Invalid level: '{s}'. Must be a number between 1 and 6"))?;

    if !(1..=6).contains(&level) {
        return Err(format!("Invalid level: {level}. Must be between 1 and 6"));
    }

    Ok(level)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let filter: HeadingLevelFilter = "2".parse().unwrap();
        assert_eq!(filter, HeadingLevelFilter::Exact(2));
        assert!(!filter.matches(1));
        assert!(filter.matches(2));
        assert!(!filter.matches(3));

        let filter: HeadingLevelFilter = "=3".parse().unwrap();
        assert_eq!(filter, HeadingLevelFilter::Exact(3));
        assert!(filter.matches(3));
        assert!(!filter.matches(2));
    }

    #[test]
    fn test_less_than() {
        let filter: HeadingLevelFilter = "<4".parse().unwrap();
        assert_eq!(filter, HeadingLevelFilter::LessThan(4));
        assert!(filter.matches(1));
        assert!(filter.matches(2));
        assert!(filter.matches(3));
        assert!(!filter.matches(4));
        assert!(!filter.matches(5));
    }

    #[test]
    fn test_less_than_or_equal() {
        let filter: HeadingLevelFilter = "<=2".parse().unwrap();
        assert_eq!(filter, HeadingLevelFilter::LessThanOrEqual(2));
        assert!(filter.matches(1));
        assert!(filter.matches(2));
        assert!(!filter.matches(3));
    }

    #[test]
    fn test_greater_than() {
        let filter: HeadingLevelFilter = ">2".parse().unwrap();
        assert_eq!(filter, HeadingLevelFilter::GreaterThan(2));
        assert!(!filter.matches(1));
        assert!(!filter.matches(2));
        assert!(filter.matches(3));
        assert!(filter.matches(4));
    }

    #[test]
    fn test_greater_than_or_equal() {
        let filter: HeadingLevelFilter = ">=3".parse().unwrap();
        assert_eq!(filter, HeadingLevelFilter::GreaterThanOrEqual(3));
        assert!(!filter.matches(1));
        assert!(!filter.matches(2));
        assert!(filter.matches(3));
        assert!(filter.matches(4));
    }

    #[test]
    fn test_list() {
        let filter: HeadingLevelFilter = "1,2,3".parse().unwrap();
        assert_eq!(filter, HeadingLevelFilter::List(vec![1, 2, 3]));
        assert!(filter.matches(1));
        assert!(filter.matches(2));
        assert!(filter.matches(3));
        assert!(!filter.matches(4));

        let filter: HeadingLevelFilter = "2,4,6".parse().unwrap();
        assert!(filter.matches(2));
        assert!(!filter.matches(3));
        assert!(filter.matches(4));
    }

    #[test]
    fn test_range() {
        let filter: HeadingLevelFilter = "1-3".parse().unwrap();
        assert_eq!(filter, HeadingLevelFilter::Range(1, 3));
        assert!(filter.matches(1));
        assert!(filter.matches(2));
        assert!(filter.matches(3));
        assert!(!filter.matches(4));

        let filter: HeadingLevelFilter = "2-5".parse().unwrap();
        assert!(!filter.matches(1));
        assert!(filter.matches(2));
        assert!(filter.matches(5));
        assert!(!filter.matches(6));
    }

    #[test]
    fn test_invalid_level_zero() {
        let result: Result<HeadingLevelFilter, _> = "0".parse();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("between 1 and 6"));
    }

    #[test]
    fn test_invalid_level_seven() {
        let result: Result<HeadingLevelFilter, _> = "7".parse();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("between 1 and 6"));
    }

    #[test]
    fn test_invalid_syntax() {
        let result: Result<HeadingLevelFilter, _> = "abc".parse();
        assert!(result.is_err());

        let result: Result<HeadingLevelFilter, _> = "1.5".parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_range() {
        let result: Result<HeadingLevelFilter, _> = "3-1".parse();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("start (3) > end (1)"));
    }

    #[test]
    fn test_whitespace_handling() {
        let filter: HeadingLevelFilter = " 2 ".parse().unwrap();
        assert_eq!(filter, HeadingLevelFilter::Exact(2));

        let filter: HeadingLevelFilter = " <= 3 ".parse().unwrap();
        assert_eq!(filter, HeadingLevelFilter::LessThanOrEqual(3));

        let filter: HeadingLevelFilter = "1 , 2 , 3".parse().unwrap();
        assert_eq!(filter, HeadingLevelFilter::List(vec![1, 2, 3]));

        let filter: HeadingLevelFilter = " 1 - 3 ".parse().unwrap();
        assert_eq!(filter, HeadingLevelFilter::Range(1, 3));
    }

    #[test]
    fn test_all_valid_levels() {
        for level in 1..=6 {
            let filter: HeadingLevelFilter = level.to_string().parse().unwrap();
            assert!(filter.matches(level));
        }
    }

    #[test]
    fn test_complex_list() {
        let filter: HeadingLevelFilter = "1,3,5".parse().unwrap();
        assert!(filter.matches(1));
        assert!(!filter.matches(2));
        assert!(filter.matches(3));
        assert!(!filter.matches(4));
        assert!(filter.matches(5));
        assert!(!filter.matches(6));
    }
}
