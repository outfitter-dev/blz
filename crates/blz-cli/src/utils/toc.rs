use blz_core::TocEntry;

/// Count all headings within a table of contents, including nested children.
pub fn count_headings(entries: &[TocEntry]) -> usize {
    entries
        .iter()
        .map(|entry| 1 + count_headings(&entry.children))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_nested_headings() {
        let toc = vec![TocEntry {
            heading_path: vec!["Root".into()],
            lines: "1-10".into(),
            anchor: None,
            children: vec![TocEntry {
                heading_path: vec!["Root".into(), "Child".into()],
                lines: "2-5".into(),
                anchor: None,
                children: vec![TocEntry {
                    heading_path: vec!["Root".into(), "Child".into(), "Grandchild".into()],
                    lines: "3-4".into(),
                    anchor: None,
                    children: Vec::new(),
                }],
            }],
        }];

        assert_eq!(count_headings(&toc), 3);
    }

    #[test]
    fn empty_toc_returns_zero() {
        let toc: Vec<TocEntry> = Vec::new();
        assert_eq!(count_headings(&toc), 0);
    }
}
