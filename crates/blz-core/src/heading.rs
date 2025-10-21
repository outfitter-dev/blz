use html_escape::decode_html_entities;
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

/// Variants derived from a raw heading segment.
#[derive(Debug, Clone)]
pub struct HeadingSegmentVariants {
    /// Human-friendly display text (markdown links & empty anchors removed).
    pub display: String,
    /// Normalized search string (lowercase, punctuation stripped, diacritics removed).
    pub normalized: String,
    /// Tokenized representation of the normalized string.
    pub tokens: Vec<String>,
}

/// Compute display and normalized variants for a raw heading segment.
///
/// - Markdown links (`[Label](url)`) are reduced to `Label`
/// - Empty HTML anchors (`<a id=\"foo\"></a>`) and surrounding tags are removed
/// - HTML entities are decoded
/// - Normalized form lowercases, strips diacritics/punctuation, and collapses whitespace
pub fn segment_variants(raw: &str) -> HeadingSegmentVariants {
    let stripped = strip_links_and_anchors(raw);
    let display = decode_html_entities(&stripped).trim().to_string();
    let normalized = normalize_for_search(&display);

    let tokens: Vec<String> = normalized
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(std::string::ToString::to_string)
        .collect();

    HeadingSegmentVariants {
        display,
        normalized,
        tokens,
    }
}

/// Aggregated heading path variants for display and search.
#[derive(Debug, Clone)]
pub struct HeadingPathVariants {
    /// Sanitized segments suitable for presentation.
    pub display_segments: Vec<String>,
    /// Lowercased, punctuation-stripped segments per heading level.
    pub normalized_segments: Vec<String>,
    /// Tokenized representation of the normalized path for indexing.
    pub tokens: Vec<String>,
}

/// Compute display + normalized variants for an entire heading path.
pub fn path_variants(path: &[String]) -> HeadingPathVariants {
    let mut display_segments = Vec::with_capacity(path.len());
    let mut normalized_segments = Vec::with_capacity(path.len());
    let mut tokens = Vec::new();

    for segment in path {
        let HeadingSegmentVariants {
            display,
            normalized,
            tokens: mut seg_tokens,
        } = segment_variants(segment);

        let display_segment = if display.is_empty() {
            segment.clone()
        } else {
            display
        };

        let normalized_segment = if normalized.is_empty() {
            display_segment.to_lowercase()
        } else {
            normalized
        };

        if seg_tokens.is_empty() {
            seg_tokens = normalized_segment
                .split_whitespace()
                .map(std::string::ToString::to_string)
                .collect();
        }

        display_segments.push(display_segment.clone());
        normalized_segments.push(normalized_segment);
        tokens.extend(seg_tokens);
    }

    HeadingPathVariants {
        display_segments,
        normalized_segments,
        tokens,
    }
}

fn strip_links_and_anchors(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'[' => {
                if let Some((label_end, link_end)) = find_markdown_link(bytes, i) {
                    output.push_str(&input[i + 1..label_end]);
                    i = link_end + 1;
                    continue;
                }
                output.push('[');
                i += 1;
            },
            b'<' => {
                if let Some(next_gt) = memchr::memchr(b'>', &bytes[i + 1..]).map(|pos| pos + i + 1)
                {
                    let tag = &input[i + 1..next_gt];
                    let tag_lower = tag.trim().to_ascii_lowercase();
                    if tag_lower.starts_with("a ") || tag_lower.starts_with("a>") {
                        i = next_gt + 1;
                        continue;
                    }
                    if tag_lower.starts_with("/a") {
                        i = next_gt + 1;
                        continue;
                    }
                }
                output.push('<');
                i += 1;
            },
            _ => {
                output.push(bytes[i] as char);
                i += 1;
            },
        }
    }

    output
}

fn find_markdown_link(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    let mut idx = start + 1;
    while idx < bytes.len() {
        match bytes[idx] {
            b'\\' => idx += 2,
            b']' => {
                if idx + 1 < bytes.len() && bytes[idx + 1] == b'(' {
                    if let Some(close_paren) = find_matching_paren(bytes, idx + 2) {
                        return Some((idx, close_paren));
                    }
                }
                return None;
            },
            _ => idx += 1,
        }
    }
    None
}

fn find_matching_paren(bytes: &[u8], mut pos: usize) -> Option<usize> {
    let mut depth = 1;
    while pos < bytes.len() {
        match bytes[pos] {
            b'\\' => pos += 2,
            b'(' => {
                depth += 1;
                pos += 1;
            },
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(pos);
                }
                pos += 1;
            },
            _ => pos += 1,
        }
    }
    None
}

fn normalize_for_search(display: &str) -> String {
    let mut normalized = String::with_capacity(display.len());
    let mut prev_was_space = true;

    for ch in display.nfkd() {
        if is_combining_mark(ch) {
            continue;
        }

        for lower in ch.to_lowercase() {
            if lower.is_ascii_alphanumeric() {
                normalized.push(lower);
                prev_was_space = false;
            } else if lower.is_whitespace()
                || matches!(
                    lower,
                    '-' | '_'
                        | '/'
                        | '.'
                        | '#'
                        | ':'
                        | '`'
                        | '\''
                        | '"'
                        | '('
                        | ')'
                        | '['
                        | ']'
                        | '{'
                        | '}'
                )
                || lower.is_ascii()
            {
                push_space(&mut normalized, &mut prev_was_space);
            } else if lower.is_alphanumeric() {
                normalized.push(lower);
                prev_was_space = false;
            } else {
                push_space(&mut normalized, &mut prev_was_space);
            }
        }
    }

    normalized.trim().to_string()
}

/// Public helper to normalize arbitrary text using the same rules as headings.
#[must_use]
pub fn normalize_text_for_search(text: &str) -> String {
    normalize_for_search(text)
}

fn push_space(normalized: &mut String, prev_was_space: &mut bool) {
    if !*prev_was_space && !normalized.is_empty() {
        normalized.push(' ');
        *prev_was_space = true;
    }
}
