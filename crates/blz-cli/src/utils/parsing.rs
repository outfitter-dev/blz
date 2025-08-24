//! # Input Parsing Utilities
//!
//! This module provides parsing functions for various user input formats,
//! particularly for line range specifications used by the `get` command.
//! The parsers are designed to be forgiving of whitespace while strictly
//! validating the semantic correctness of the input.

use anyhow::Result;

/// Represents different types of line range specifications
///
/// This enum captures the three different ways users can specify line ranges
/// in the CLI. Each variant is optimized for different use cases:
///
/// - [`Single`]: For retrieving a single specific line
/// - [`Range`]: For retrieving a contiguous block of lines  
/// - [`PlusCount`]: For retrieving a line plus N additional lines
///
/// # Examples
///
/// ```rust,no_run
/// use blz_cli::utils::LineRange;
///
/// let single = LineRange::Single(42);        // Line 42
/// let range = LineRange::Range(10, 20);      // Lines 10-20 (inclusive)
/// let plus = LineRange::PlusCount(100, 5);   // Line 100 plus 5 more (100-105)
/// ```
///
/// # Line Numbering
///
/// All line numbers are 1-based to match common editor conventions:
/// - Line 1 is the first line of the file
/// - Line 0 is invalid and will cause parsing errors
/// - Ranges are inclusive on both ends (e.g., 10:15 includes lines 10, 11, 12, 13, 14, 15)
///
/// # Validation
///
/// Line ranges are validated during parsing to ensure:
/// - All line numbers are >= 1
/// - Range start <= range end
/// - Plus counts are >= 1
/// - No integer overflow in calculations
#[derive(Debug, Clone)]
pub enum LineRange {
    /// Single line number
    Single(usize),
    /// Range from start to end (inclusive)
    Range(usize, usize),
    /// Start line plus count of additional lines
    PlusCount(usize, usize),
}

/// Parse line range specifications from a string
///
/// This function parses user-provided line range specifications and returns
/// a vector of [`LineRange`] objects. It supports multiple range formats and
/// can parse comma-separated lists of ranges in a single call.
///
/// # Supported Formats
///
/// ## Single Line
/// - `"42"` → [`LineRange::Single(42)`]
/// - Retrieves exactly one line
///
/// ## Inclusive Ranges
/// - `"120:142"` → [`LineRange::Range(120, 142)`] (colon syntax)
/// - `"120-142"` → [`LineRange::Range(120, 142)`] (dash syntax)  
/// - Both retrieve lines 120 through 142 (inclusive)
/// - Colon syntax matches many editors; dash syntax matches common CLI tools
///
/// ## Plus Count
/// - `"36+20"` → [`LineRange::PlusCount(36, 20)`]
/// - Retrieves line 36 plus 20 additional lines (lines 36-56)
/// - Useful when you know a starting point and want N lines from there
///
/// ## Multiple Ranges
/// - `"36:43,120-142,200+10"`
/// - Comma-separated list processed left to right
/// - Whitespace around commas and ranges is ignored
/// - Each range can use any of the supported formats
///
/// # Arguments
///
/// * `input` - String containing one or more line range specifications
///
/// # Returns
///
/// Returns `Ok(Vec<LineRange>)` with the parsed ranges, or `Err` if any
/// range is invalid.
///
/// # Errors
///
/// This function returns an error if:
/// - Line numbers are not valid positive integers
/// - Line numbers are 0 (lines are 1-indexed)
/// - Range start is greater than range end (e.g., "50:30")
/// - Plus count is 0 (e.g., "100+0")  
/// - Input contains invalid characters or malformed ranges
///
/// # Examples
///
/// ```rust,no_run
/// use blz_cli::utils::{parse_line_ranges, LineRange};
///
/// // Single line
/// let ranges = parse_line_ranges("42")?;
/// assert!(matches!(ranges[0], LineRange::Single(42)));
///
/// // Range formats
/// let ranges = parse_line_ranges("120:142")?;
/// assert!(matches!(ranges[0], LineRange::Range(120, 142)));
///
/// let ranges = parse_line_ranges("120-142")?; // Equivalent
/// assert!(matches!(ranges[0], LineRange::Range(120, 142)));
///
/// // Plus syntax
/// let ranges = parse_line_ranges("36+20")?;
/// assert!(matches!(ranges[0], LineRange::PlusCount(36, 20)));
///
/// // Multiple ranges
/// let ranges = parse_line_ranges("1:5,100,200+10")?;
/// assert_eq!(ranges.len(), 3);
/// assert!(matches!(ranges[0], LineRange::Range(1, 5)));
/// assert!(matches!(ranges[1], LineRange::Single(100)));
/// assert!(matches!(ranges[2], LineRange::PlusCount(200, 10)));
///
/// // Error cases
/// assert!(parse_line_ranges("0").is_err());        // Line 0 invalid
/// assert!(parse_line_ranges("50:30").is_err());    // Backwards range
/// assert!(parse_line_ranges("100+0").is_err());    // Zero count
/// assert!(parse_line_ranges("abc").is_err());      // Non-numeric
/// ```
///
/// # Implementation Notes
///
/// The parser is designed to be liberal in what it accepts (whitespace tolerance)
/// but conservative in what it produces (strict validation). This provides a
/// good user experience while maintaining data integrity.
///
/// Parsing is performed left-to-right with no backtracking, making it efficient
/// for typical CLI usage patterns. The function handles edge cases gracefully
/// and provides descriptive error messages for debugging.
pub fn parse_line_ranges(input: &str) -> Result<Vec<LineRange>> {
    let mut ranges = Vec::new();

    for part in input.split(',') {
        let part = part.trim();

        if let Some(colon_pos) = part.find(':') {
            ranges.push(parse_colon_range(part, colon_pos)?);
        } else if let Some(dash_pos) = part.find('-') {
            ranges.push(parse_dash_range(part, dash_pos)?);
        } else if let Some(plus_pos) = part.find('+') {
            ranges.push(parse_plus_range(part, plus_pos)?);
        } else {
            ranges.push(parse_single_line(part)?);
        }
    }

    Ok(ranges)
}

fn parse_colon_range(part: &str, colon_pos: usize) -> Result<LineRange> {
    let start_str = part[..colon_pos].trim();
    let end_str = part[colon_pos + 1..].trim();

    let start = parse_line_number(start_str, "start")?;
    let end = parse_line_number(end_str, "end")?;

    validate_range(start, end)?;
    Ok(LineRange::Range(start, end))
}

fn parse_dash_range(part: &str, dash_pos: usize) -> Result<LineRange> {
    let start_str = part[..dash_pos].trim();
    let end_str = part[dash_pos + 1..].trim();

    let start = parse_line_number(start_str, "start")?;
    let end = parse_line_number(end_str, "end")?;

    validate_range(start, end)?;
    Ok(LineRange::Range(start, end))
}

fn parse_plus_range(part: &str, plus_pos: usize) -> Result<LineRange> {
    let start_str = part[..plus_pos].trim();
    let count_str = part[plus_pos + 1..].trim();

    let start = parse_line_number(start_str, "start")?;
    let count: usize = count_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid count: {}", count_str))?;

    if count == 0 {
        return Err(anyhow::anyhow!("Count must be at least 1"));
    }

    Ok(LineRange::PlusCount(start, count))
}

fn parse_single_line(part: &str) -> Result<LineRange> {
    let line = parse_line_number(part, "line")?;
    Ok(LineRange::Single(line))
}

fn parse_line_number(s: &str, context: &str) -> Result<usize> {
    let line: usize = s
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid {} line: {}", context, s))?;

    if line == 0 {
        return Err(anyhow::anyhow!("Line numbers must be >= 1"));
    }

    Ok(line)
}

fn validate_range(start: usize, end: usize) -> Result<()> {
    if start > end {
        return Err(anyhow::anyhow!("Invalid range: {}-{}", start, end));
    }
    Ok(())
}
