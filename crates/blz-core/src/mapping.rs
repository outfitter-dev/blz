use crate::{AnchorMapping, AnchorsMap, TocEntry};
use chrono::{DateTime, Utc};

/// Compute anchor remapping between two TOC trees.
///
/// Returns mappings for anchors whose line ranges changed between versions.
pub fn compute_anchor_mappings(old: &[TocEntry], new: &[TocEntry]) -> Vec<AnchorMapping> {
    let mut old_map = std::collections::HashMap::<String, (String, Vec<String>)>::new();
    fn collect(
        map: &mut std::collections::HashMap<String, (String, Vec<String>)>,
        list: &[TocEntry],
    ) {
        for e in list {
            if let Some(a) = &e.anchor {
                map.insert(a.clone(), (e.lines.clone(), e.heading_path.clone()));
            }
            if !e.children.is_empty() {
                collect(map, &e.children);
            }
        }
    }
    collect(&mut old_map, old);

    let mut mappings = Vec::new();
    fn walk(
        mappings: &mut Vec<AnchorMapping>,
        old_map: &std::collections::HashMap<String, (String, Vec<String>)>,
        list: &[TocEntry],
    ) {
        for e in list {
            if let (Some(anchor), new_lines) = (e.anchor.as_ref(), &e.lines) {
                if let Some((old_lines, path)) = old_map.get(anchor) {
                    if old_lines != new_lines {
                        mappings.push(AnchorMapping {
                            anchor: anchor.clone(),
                            old_lines: old_lines.clone(),
                            new_lines: new_lines.clone(),
                            heading_path: path.clone(),
                        });
                    }
                }
            }
            if !e.children.is_empty() {
                walk(mappings, old_map, &e.children);
            }
        }
    }
    walk(&mut mappings, &old_map, new);
    mappings
}

/// Convenience to build an `AnchorsMap` with a timestamp.
pub const fn build_anchors_map(mappings: Vec<AnchorMapping>, ts: DateTime<Utc>) -> AnchorsMap {
    AnchorsMap {
        updated_at: ts,
        mappings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MarkdownParser, ParseResult};

    fn parse_toc(s: &str) -> ParseResult {
        let mut p = MarkdownParser::new().expect("parser");
        p.parse(s).expect("parse")
    }

    #[test]
    fn compute_mappings_detects_moved_section() {
        let v1 = r"
# Title

## A
alpha

## B
bravo

## C
charlie
";
        let v2 = r"
# Title

## C
charlie

## A
alpha

## B
bravo
";
        let r1 = parse_toc(v1);
        let r2 = parse_toc(v2);

        // Anchors should be stable across moves (same heading text)
        fn find_anchor<'a>(list: &'a [TocEntry], name: &str) -> Option<&'a str> {
            for e in list {
                if e.heading_path.last().map(std::string::String::as_str) == Some(name) {
                    if let Some(a) = e.anchor.as_deref() {
                        return Some(a);
                    }
                }
                if let Some(a) = find_anchor(&e.children, name) {
                    return Some(a);
                }
            }
            None
        }
        let a1 = find_anchor(&r1.toc, "A").expect("anchor A v1");
        let a2 = find_anchor(&r2.toc, "A").expect("anchor A v2");
        assert_eq!(a1, a2, "anchor should be stable for A");

        let mappings = compute_anchor_mappings(&r1.toc, &r2.toc);
        // Expect at least one mapping (A or C moved)
        assert!(!mappings.is_empty(), "should detect moved sections");
        // Ensure mapping for A exists and old/new lines differ
        let m_a = mappings
            .iter()
            .find(|m| m.anchor == a1)
            .expect("mapping for A");
        assert_ne!(m_a.old_lines, m_a.new_lines);
    }
}
