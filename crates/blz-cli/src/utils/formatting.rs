//! Formatting utilities

use colored::Colorize;
use terminal_size::{Width, terminal_size};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Color cycling functions for aliases
pub const ALIAS_COLORS: &[fn(&str) -> colored::ColoredString] = &[
    |s| s.green(),
    |s| s.blue(),
    |s| s.truecolor(0, 150, 136), // teal
    |s| s.magenta(),
];

const SEPARATOR: &str = " > ";
const ELLIPSIS: &str = "...";

/// Get a color for an alias based on its index
pub fn get_alias_color(alias: &str, index: usize) -> colored::ColoredString {
    let color_fn = ALIAS_COLORS[index % ALIAS_COLORS.len()];
    color_fn(alias)
}

/// Best-effort detection of the active terminal width in characters.
///
/// Falls back to `None` when the width cannot be determined (e.g., when stdout is
/// redirected). Callers should provide a sensible default in that case.
pub fn terminal_width() -> Option<usize> {
    terminal_size().map(|(Width(w), _)| usize::from(w))
}

/// Format a heading path so that it fits within the provided maximum width.
///
/// The function preserves the first and last segments, inserting an ellipsis when
/// intermediate segments need to be collapsed. When space is extremely limited it
/// gracefully degrades to showing only the trailing segment (possibly truncated).
pub fn format_heading_path(segments: &[String], max_width: usize) -> String {
    if segments.is_empty() || max_width == 0 {
        return String::new();
    }

    let plain_segments: Vec<&str> = segments.iter().map(String::as_str).collect();
    let mut pieces = build_components(&plain_segments, 1);

    if components_width(&pieces) <= max_width {
        return pieces_to_string(&pieces);
    }

    if plain_segments.len() == 1 {
        let truncated = truncate_to_width(plain_segments[0], max_width);
        return pieces_to_string(&[PathPiece::Segment(truncated)]);
    }

    for include_from in 2..plain_segments.len() {
        pieces = build_components(&plain_segments, include_from);
        let width = components_width(&pieces);
        if width <= max_width {
            return pieces_to_string(&pieces);
        }

        if let Some(adjusted) = shrink_last_segment(&pieces, max_width) {
            return pieces_to_string(&adjusted);
        }
    }

    // Final attempt: show ellipsis + trailing segment if it fits.
    let last = plain_segments.last().map(|s| {
        let reserved = ELLIPSIS.len() + SEPARATOR.len();
        let available = max_width.saturating_sub(reserved);
        truncate_to_width(s, available)
    });
    if let Some(last_segment) = last {
        let minimal = vec![
            PathPiece::Ellipsis,
            PathPiece::Segment(last_segment.clone()),
        ];
        if components_width(&minimal) <= max_width {
            return pieces_to_string(&minimal);
        }

        let truncated_only = truncate_to_width(&last_segment, max_width);
        return pieces_to_string(&[PathPiece::Segment(truncated_only)]);
    }

    String::new()
}

#[derive(Clone, Debug)]
enum PathPiece {
    Segment(String),
    Ellipsis,
}

fn build_components(segments: &[&str], include_from: usize) -> Vec<PathPiece> {
    let mut pieces = Vec::new();
    if let Some(first) = segments.first() {
        pieces.push(PathPiece::Segment((*first).to_string()));
    }

    if include_from > 1 {
        pieces.push(PathPiece::Ellipsis);
    }

    for segment in segments.iter().skip(include_from) {
        pieces.push(PathPiece::Segment((*segment).to_string()));
    }

    pieces
}

fn pieces_to_string(pieces: &[PathPiece]) -> String {
    let last_segment_index = pieces
        .iter()
        .rposition(|piece| matches!(piece, PathPiece::Segment(_)));

    let mut parts: Vec<String> = pieces
        .iter()
        .map(|piece| match piece {
            PathPiece::Segment(text) => text.clone(),
            PathPiece::Ellipsis => ELLIPSIS.to_string(),
        })
        .collect();

    if let Some(idx) = last_segment_index {
        if let Some(segment) = parts.get_mut(idx) {
            *segment = segment.as_str().bold().to_string();
        }
    }

    parts.join(SEPARATOR)
}

fn components_width(pieces: &[PathPiece]) -> usize {
    let mut width: usize = 0;
    for (idx, piece) in pieces.iter().enumerate() {
        if idx > 0 {
            width = width.saturating_add(SEPARATOR.len());
        }
        width = width.saturating_add(match piece {
            PathPiece::Segment(text) => UnicodeWidthStr::width(text.as_str()),
            PathPiece::Ellipsis => ELLIPSIS.len(),
        });
    }
    width
}

fn shrink_last_segment(pieces: &[PathPiece], max_width: usize) -> Option<Vec<PathPiece>> {
    let last_segment_index = pieces
        .iter()
        .rposition(|piece| matches!(piece, PathPiece::Segment(_)))?;

    let mut prefix_width: usize = 0;
    for (idx, piece) in pieces.iter().enumerate() {
        if idx >= last_segment_index {
            break;
        }
        if idx > 0 {
            prefix_width = prefix_width.saturating_add(SEPARATOR.len());
        }
        prefix_width = prefix_width.saturating_add(match piece {
            PathPiece::Segment(text) => UnicodeWidthStr::width(text.as_str()),
            PathPiece::Ellipsis => ELLIPSIS.len(),
        });
    }

    if last_segment_index > 0 {
        prefix_width = prefix_width.saturating_add(SEPARATOR.len());
    }

    if prefix_width >= max_width {
        return None;
    }

    let available = max_width.saturating_sub(prefix_width);
    let mut adjusted = pieces.to_vec();
    if let Some(PathPiece::Segment(text)) = adjusted.get_mut(last_segment_index) {
        if UnicodeWidthStr::width(text.as_str()) <= available {
            return Some(adjusted);
        }
        *text = truncate_to_width(text.as_str(), available);
        return Some(adjusted);
    }

    None
}

fn truncate_to_width<S: AsRef<str>>(segment: S, max_width: usize) -> String {
    let segment_str = segment.as_ref();
    if UnicodeWidthStr::width(segment_str) <= max_width {
        return segment_str.to_string();
    }

    if max_width == 0 {
        return String::new();
    }

    if max_width <= ELLIPSIS.len() {
        return ELLIPSIS[..max_width].to_string();
    }

    let ellipsis_width = ELLIPSIS.len();
    let mut collected = Vec::new();
    let mut current_width = 0usize;

    for ch in segment_str.chars() {
        let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + char_width + ellipsis_width > max_width {
            break;
        }
        collected.push(ch);
        current_width = current_width.saturating_add(char_width);
    }

    if collected.is_empty() {
        return ELLIPSIS[..max_width].to_string();
    }

    let mut result: String = collected.into_iter().collect();
    result.push_str(ELLIPSIS);
    result
}

#[cfg(test)]
mod tests {
    use super::{
        ELLIPSIS, build_components, components_width, format_heading_path, truncate_to_width,
    };

    fn strip_ansi_codes(input: &str) -> String {
        let mut output = String::new();
        let mut chars = input.chars();
        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' {
                for next in chars.by_ref() {
                    if next == 'm' {
                        break;
                    }
                }
            } else {
                output.push(ch);
            }
        }
        output
    }

    #[test]
    fn keeps_full_path_when_space_allows() {
        let segments = vec![
            "Section Alpha".to_string(),
            "Section Beta".to_string(),
            "Section Gamma".to_string(),
        ];

        let formatted = format_heading_path(&segments, 80);
        assert!(
            strip_ansi_codes(&formatted).contains("Section Alpha > Section Beta > Section Gamma")
        );
    }

    #[test]
    fn collapses_middle_segments() {
        let segments = vec![
            "Root".to_string(),
            "Intermediate".to_string(),
            "Penultimate".to_string(),
            "Leaf".to_string(),
        ];

        let formatted = format_heading_path(&segments, 39);
        assert_eq!(
            strip_ansi_codes(&formatted),
            "Root > ... > Penultimate > Leaf"
        );
    }

    #[test]
    fn falls_back_to_tail_only_when_needed() {
        let segments = vec![
            "Root".to_string(),
            "Intermediate".to_string(),
            "Penultimate".to_string(),
            "Leaf".to_string(),
        ];

        let formatted = format_heading_path(&segments, 12);
        assert_eq!(strip_ansi_codes(&formatted), "... > Leaf");
    }

    #[test]
    fn truncates_last_segment_for_tight_width() {
        let segments = vec![
            "SuperCalifragilistic".to_string(),
            "Expialidocious".to_string(),
        ];

        let formatted = format_heading_path(&segments, 10);
        let plain = strip_ansi_codes(&formatted);
        assert!(
            plain.ends_with("E..."),
            "expected trailing segment to be truncated, got {plain}"
        );
    }

    #[test]
    fn truncate_helper_handles_short_width() {
        assert_eq!(truncate_to_width("abcdef", 3), ELLIPSIS.to_string());
        assert_eq!(truncate_to_width("abcdef", 6), "abcdef".to_string());
        assert_eq!(truncate_to_width("abcdef", 5), "ab...".to_string());
    }

    #[test]
    fn components_width_accounts_for_separators() {
        let pieces = build_components(&["A", "B", "C"], 1);
        assert_eq!(components_width(&pieces), "A > B > C".len());
    }
}
