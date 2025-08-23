use crate::{Diagnostic, DiagnosticSeverity, Error, HeadingBlock, Result, TocEntry};
use std::collections::VecDeque;
use tree_sitter::{Node, Parser, TreeCursor};

pub struct MarkdownParser {
    parser: Parser,
}

impl MarkdownParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_md::LANGUAGE.into())
            .map_err(|e| Error::Parse(format!("Failed to set language: {}", e)))?;

        Ok(Self { parser })
    }

    pub fn parse(&mut self, text: &str) -> Result<ParseResult> {
        let tree = self
            .parser
            .parse(text, None)
            .ok_or_else(|| Error::Parse("Failed to parse markdown".into()))?;

        let root = tree.root_node();
        let mut diagnostics = Vec::new();
        let mut heading_blocks = Vec::new();
        let mut toc = Vec::new();

        if root.has_error() {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Warn,
                message: "Parse tree contains errors, using fallback parsing".into(),
                line: None,
            });
        }

        let mut cursor = root.walk();
        self.extract_headings(&mut cursor, text, &mut heading_blocks, &mut toc)?;

        if heading_blocks.is_empty() {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Warn,
                message: "No headings found in document".into(),
                line: Some(1),
            });

            heading_blocks.push(HeadingBlock {
                path: vec!["Document".into()],
                content: text.to_string(),
                start_line: 1,
                end_line: text.lines().count(),
            });
        }

        let line_count = text.lines().count();

        Ok(ParseResult {
            heading_blocks,
            toc,
            diagnostics,
            line_count,
        })
    }

    fn extract_headings(
        &self,
        cursor: &mut TreeCursor,
        text: &str,
        blocks: &mut Vec<HeadingBlock>,
        toc: &mut Vec<TocEntry>,
    ) -> Result<()> {
        let mut current_path = Vec::new();
        let mut current_content = String::new();
        let mut current_start = 0;
        let mut stack: VecDeque<usize> = VecDeque::new();

        self.walk_tree(cursor, text, |node| {
            if node.kind() == "atx_heading" {
                if !current_content.is_empty() && !current_path.is_empty() {
                    blocks.push(HeadingBlock {
                        path: current_path.clone(),
                        content: current_content.clone(),
                        start_line: current_start + 1,
                        end_line: node.start_position().row,
                    });
                }

                let level = self.get_heading_level(node, text);
                let heading_text = self.get_heading_text(node, text);

                while stack.len() >= level {
                    stack.pop_back();
                    current_path.pop();
                }

                current_path.push(heading_text.clone());
                stack.push_back(level);

                current_content.clear();
                current_start = node.start_position().row;

                let entry = TocEntry {
                    heading_path: current_path.clone(),
                    lines: format!("{}-", current_start + 1),
                    children: Vec::new(),
                };

                self.add_to_toc(toc, entry, stack.len());
            }

            let node_text = &text[node.byte_range()];
            current_content.push_str(node_text);
            current_content.push('\n');
        });

        if !current_content.is_empty() && !current_path.is_empty() {
            let line_count = text.lines().count();
            blocks.push(HeadingBlock {
                path: current_path,
                content: current_content,
                start_line: current_start + 1,
                end_line: line_count,
            });
        }

        Ok(())
    }

    fn walk_tree<F>(&self, cursor: &mut TreeCursor, _text: &str, mut callback: F)
    where
        F: FnMut(Node),
    {
        loop {
            let node = cursor.node();
            callback(node);

            if cursor.goto_first_child() {
                continue;
            }

            if cursor.goto_next_sibling() {
                continue;
            }

            loop {
                if !cursor.goto_parent() {
                    return;
                }
                if cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn get_heading_level(&self, node: Node, _text: &str) -> usize {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "atx_h1_marker" {
                return 1;
            } else if child.kind() == "atx_h2_marker" {
                return 2;
            } else if child.kind() == "atx_h3_marker" {
                return 3;
            } else if child.kind() == "atx_h4_marker" {
                return 4;
            } else if child.kind() == "atx_h5_marker" {
                return 5;
            } else if child.kind() == "atx_h6_marker" {
                return 6;
            }
        }
        1
    }

    fn get_heading_text(&self, node: Node, text: &str) -> String {
        for child in node.children(&mut node.walk()) {
            if child.kind().contains("heading") && child.kind().contains("content") {
                return text[child.byte_range()].trim().to_string();
            }
        }

        let full_text = &text[node.byte_range()];
        full_text.trim_start_matches('#').trim().to_string()
    }

    fn add_to_toc(&self, toc: &mut Vec<TocEntry>, entry: TocEntry, depth: usize) {
        if depth == 1 {
            toc.push(entry);
        } else if let Some(parent) = toc.last_mut() {
            self.add_to_toc_recursive(&mut parent.children, entry, depth - 1);
        }
    }

    fn add_to_toc_recursive(&self, toc: &mut Vec<TocEntry>, entry: TocEntry, depth: usize) {
        if depth == 1 {
            toc.push(entry);
        } else if let Some(parent) = toc.last_mut() {
            self.add_to_toc_recursive(&mut parent.children, entry, depth - 1);
        }
    }
}

pub struct ParseResult {
    pub heading_blocks: Vec<HeadingBlock>,
    pub toc: Vec<TocEntry>,
    pub diagnostics: Vec<Diagnostic>,
    pub line_count: usize,
}

impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new().expect("Failed to create parser")
    }
}
