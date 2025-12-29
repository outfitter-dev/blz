#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::needless_raw_string_hashes,
    clippy::items_after_statements,
    clippy::uninlined_format_args
)]

mod common;

use common::blz_cmd;
use serde_json::Value;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Seed test source with sample TOC
async fn seed_source(
    tmp: &tempfile::TempDir,
    server: &MockServer,
    alias: &str,
    doc: &str,
) -> anyhow::Result<()> {
    let url = format!("{}/llms.txt", server.uri());

    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(
            ResponseTemplate::new(200).insert_header("content-length", doc.len().to_string()),
        )
        .mount(server)
        .await;

    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(doc))
        .mount(server)
        .await;

    blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(["add", alias, url.as_str(), "-y"])
        .assert()
        .success();

    Ok(())
}

/// Helper to run toc command and parse JSON output
fn run_toc_json(tmp: &tempfile::TempDir, args: &[&str]) -> anyhow::Result<Value> {
    let stdout = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    Ok(serde_json::from_slice(&stdout)?)
}

#[tokio::test]
async fn test_heading_level_filter_lte() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = r#"# Level 1
Some content

## Level 2
More content

### Level 3
Even more

#### Level 4
Deep content
"#;

    seed_source(&tmp, &server, "docs", doc).await?;

    let json = run_toc_json(&tmp, &["toc", "docs", "-H", "<=2", "-f", "json"])?;
    let entries = json["entries"].as_array().expect("expected entries array");

    // Collect all entries including nested ones
    fn collect_levels(entries: &[Value]) -> Vec<u64> {
        let mut levels = Vec::new();
        for entry in entries {
            if let Some(level) = entry["headingLevel"].as_u64() {
                levels.push(level);
            }
            if let Some(children) = entry["children"].as_array() {
                levels.extend(collect_levels(children));
            }
        }
        levels
    }

    let levels = collect_levels(entries);
    assert!(!levels.is_empty(), "expected at least one heading");

    for level in &levels {
        assert!(*level <= 2, "Expected level <= 2, got {}", level);
    }

    // Should have both level 1 and level 2
    assert!(levels.contains(&1), "should include level 1 headings");
    assert!(levels.contains(&2), "should include level 2 headings");
    assert!(!levels.contains(&3), "should not include level 3 headings");
    assert!(!levels.contains(&4), "should not include level 4 headings");

    Ok(())
}

#[tokio::test]
async fn test_heading_level_filter_gt() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = r#"# Level 1
## Level 2
### Level 3
#### Level 4
"#;

    seed_source(&tmp, &server, "docs", doc).await?;

    let json = run_toc_json(&tmp, &["toc", "docs", "-H", ">2", "-f", "json"])?;
    let entries = json["entries"].as_array().expect("expected entries array");

    fn collect_levels(entries: &[Value]) -> Vec<u64> {
        let mut levels = Vec::new();
        for entry in entries {
            if let Some(level) = entry["headingLevel"].as_u64() {
                levels.push(level);
            }
            if let Some(children) = entry["children"].as_array() {
                levels.extend(collect_levels(children));
            }
        }
        levels
    }

    let levels = collect_levels(entries);
    assert!(!levels.is_empty(), "expected at least one heading");

    for level in &levels {
        assert!(*level > 2, "Expected level > 2, got {}", level);
    }

    assert!(!levels.contains(&1), "should not include level 1");
    assert!(!levels.contains(&2), "should not include level 2");
    assert!(levels.contains(&3), "should include level 3");
    assert!(levels.contains(&4), "should include level 4");

    Ok(())
}

#[tokio::test]
async fn test_heading_level_filter_gte() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = r#"# Level 1
## Level 2
### Level 3
#### Level 4
"#;

    seed_source(&tmp, &server, "docs", doc).await?;

    let json = run_toc_json(&tmp, &["toc", "docs", "-H", ">=3", "-f", "json"])?;
    let entries = json["entries"].as_array().expect("expected entries array");

    fn collect_levels(entries: &[Value]) -> Vec<u64> {
        let mut levels = Vec::new();
        for entry in entries {
            if let Some(level) = entry["headingLevel"].as_u64() {
                levels.push(level);
            }
            if let Some(children) = entry["children"].as_array() {
                levels.extend(collect_levels(children));
            }
        }
        levels
    }

    let levels = collect_levels(entries);
    assert!(!levels.is_empty(), "expected at least one heading");

    for level in &levels {
        assert!(*level >= 3, "Expected level >= 3, got {}", level);
    }

    assert!(!levels.contains(&1), "should not include level 1");
    assert!(!levels.contains(&2), "should not include level 2");
    assert!(levels.contains(&3), "should include level 3");
    assert!(levels.contains(&4), "should include level 4");

    Ok(())
}

#[tokio::test]
async fn test_heading_level_filter_lt() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = r#"# Level 1
## Level 2
### Level 3
#### Level 4
"#;

    seed_source(&tmp, &server, "docs", doc).await?;

    let json = run_toc_json(&tmp, &["toc", "docs", "-H", "<4", "-f", "json"])?;
    let entries = json["entries"].as_array().expect("expected entries array");

    fn collect_levels(entries: &[Value]) -> Vec<u64> {
        let mut levels = Vec::new();
        for entry in entries {
            if let Some(level) = entry["headingLevel"].as_u64() {
                levels.push(level);
            }
            if let Some(children) = entry["children"].as_array() {
                levels.extend(collect_levels(children));
            }
        }
        levels
    }

    let levels = collect_levels(entries);
    assert!(!levels.is_empty(), "expected at least one heading");

    for level in &levels {
        assert!(*level < 4, "Expected level < 4, got {}", level);
    }

    assert!(levels.contains(&1), "should include level 1");
    assert!(levels.contains(&2), "should include level 2");
    assert!(levels.contains(&3), "should include level 3");
    assert!(!levels.contains(&4), "should not include level 4");

    Ok(())
}

#[tokio::test]
async fn test_heading_level_filter_eq() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = r#"# Level 1
## Level 2
### Level 3
"#;

    seed_source(&tmp, &server, "docs", doc).await?;

    let json = run_toc_json(&tmp, &["toc", "docs", "-H", "=2", "-f", "json"])?;
    let entries = json["entries"].as_array().expect("expected entries array");

    fn collect_levels(entries: &[Value]) -> Vec<u64> {
        let mut levels = Vec::new();
        for entry in entries {
            if let Some(level) = entry["headingLevel"].as_u64() {
                levels.push(level);
            }
            if let Some(children) = entry["children"].as_array() {
                levels.extend(collect_levels(children));
            }
        }
        levels
    }

    let levels = collect_levels(entries);
    assert!(!levels.is_empty(), "expected at least one heading");

    for level in &levels {
        assert_eq!(*level, 2, "Expected level = 2, got {}", level);
    }

    assert!(!levels.contains(&1), "should not include level 1");
    assert!(levels.contains(&2), "should include level 2");
    assert!(!levels.contains(&3), "should not include level 3");

    Ok(())
}

#[tokio::test]
async fn test_heading_level_filter_range() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = r#"# Level 1
## Level 2
### Level 3
#### Level 4
"#;

    seed_source(&tmp, &server, "docs", doc).await?;

    let json = run_toc_json(&tmp, &["toc", "docs", "-H", "2-3", "-f", "json"])?;
    let entries = json["entries"].as_array().expect("expected entries array");

    fn collect_levels(entries: &[Value]) -> Vec<u64> {
        let mut levels = Vec::new();
        for entry in entries {
            if let Some(level) = entry["headingLevel"].as_u64() {
                levels.push(level);
            }
            if let Some(children) = entry["children"].as_array() {
                levels.extend(collect_levels(children));
            }
        }
        levels
    }

    let levels = collect_levels(entries);
    assert!(!levels.is_empty(), "expected at least one heading");

    for level in &levels {
        assert!((2..=3).contains(level), "Expected level 2-3, got {}", level);
    }

    assert!(!levels.contains(&1), "should not include level 1");
    assert!(levels.contains(&2), "should include level 2");
    assert!(levels.contains(&3), "should include level 3");
    assert!(!levels.contains(&4), "should not include level 4");

    Ok(())
}

#[tokio::test]
async fn test_heading_level_filter_list() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = r#"# Level 1
## Level 2
### Level 3
#### Level 4
"#;

    seed_source(&tmp, &server, "docs", doc).await?;

    let json = run_toc_json(&tmp, &["toc", "docs", "-H", "1,3", "-f", "json"])?;
    let entries = json["entries"].as_array().expect("expected entries array");

    fn collect_levels(entries: &[Value]) -> Vec<u64> {
        let mut levels = Vec::new();
        for entry in entries {
            if let Some(level) = entry["headingLevel"].as_u64() {
                levels.push(level);
            }
            if let Some(children) = entry["children"].as_array() {
                levels.extend(collect_levels(children));
            }
        }
        levels
    }

    let levels = collect_levels(entries);
    assert!(!levels.is_empty(), "expected at least one heading");

    for level in &levels {
        assert!(
            *level == 1 || *level == 3,
            "Expected level 1 or 3, got {}",
            level
        );
    }

    assert!(levels.contains(&1), "should include level 1");
    assert!(!levels.contains(&2), "should not include level 2");
    assert!(levels.contains(&3), "should include level 3");
    assert!(!levels.contains(&4), "should not include level 4");

    Ok(())
}

#[tokio::test]
async fn test_backward_compatibility_max_depth() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = r#"# Level 1
## Level 2
### Level 3
"#;

    seed_source(&tmp, &server, "docs", doc).await?;

    // Run with --max-depth
    let json1 = run_toc_json(&tmp, &["toc", "docs", "--max-depth", "2", "-f", "json"])?;

    // Run with -H <=2
    let json2 = run_toc_json(&tmp, &["toc", "docs", "-H", "<=2", "-f", "json"])?;

    // Both should produce identical results
    assert_eq!(
        json1, json2,
        "--max-depth 2 should behave identically to -H <=2"
    );

    Ok(())
}

#[tokio::test]
async fn test_tree_view_rendering() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = r#"# Root
## Child 1
### Grandchild 1
## Child 2
"#;

    seed_source(&tmp, &server, "docs", doc).await?;

    let output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("NO_COLOR", "1") // Disable colors for easier testing
        .args(["toc", "docs", "--tree", "-f", "text"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output)?;

    // Verify tree characters are present
    assert!(
        text.contains("├─") || text.contains("└─"),
        "Tree output should contain branch characters (├─ or └─)\nGot: {}",
        text
    );
    assert!(
        text.contains("│") || text.contains("├─") || text.contains("└─"),
        "Tree output should contain tree-drawing characters\nGot: {}",
        text
    );

    // Verify headings are present
    assert!(text.contains("Root"), "Tree should include root heading");
    assert!(text.contains("Child"), "Tree should include child headings");

    Ok(())
}

#[tokio::test]
async fn test_multi_source_with_source_flag() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc1 = "# Docs 1\n## Section A\n";
    let doc2 = "# Docs 2\n## Section B\n";

    seed_source(&tmp, &server, "docs1", doc1).await?;

    // Reset server for second source
    server.reset().await;
    seed_source(&tmp, &server, "docs2", doc2).await?;

    let json = run_toc_json(&tmp, &["toc", "--source", "docs1,docs2", "-f", "json"])?;
    let entries = json["entries"].as_array().expect("expected entries array");

    // Collect aliases from all entries
    fn collect_aliases(entries: &[Value]) -> Vec<String> {
        let mut aliases = Vec::new();
        for entry in entries {
            if let Some(alias) = entry["alias"].as_str() {
                aliases.push(alias.to_string());
            }
            if let Some(children) = entry["children"].as_array() {
                aliases.extend(collect_aliases(children));
            }
        }
        aliases
    }

    let aliases = collect_aliases(entries);
    assert!(
        aliases.contains(&"docs1".to_string()),
        "should include docs1"
    );
    assert!(
        aliases.contains(&"docs2".to_string()),
        "should include docs2"
    );

    Ok(())
}

#[tokio::test]
async fn test_multi_source_with_all_flag() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc1 = "# Docs 1\n## Section A\n";
    let doc2 = "# Docs 2\n## Section B\n";

    seed_source(&tmp, &server, "docs1", doc1).await?;

    // Reset server for second source
    server.reset().await;
    seed_source(&tmp, &server, "docs2", doc2).await?;

    let json = run_toc_json(&tmp, &["toc", "--all", "-f", "json"])?;
    let entries = json["entries"].as_array().expect("expected entries array");

    // Collect aliases from all entries
    fn collect_aliases(entries: &[Value]) -> Vec<String> {
        let mut aliases = Vec::new();
        for entry in entries {
            if let Some(alias) = entry["alias"].as_str() {
                aliases.push(alias.to_string());
            }
            if let Some(children) = entry["children"].as_array() {
                aliases.extend(collect_aliases(children));
            }
        }
        aliases
    }

    let aliases = collect_aliases(entries);
    assert!(
        aliases.contains(&"docs1".to_string()),
        "should include docs1"
    );
    assert!(
        aliases.contains(&"docs2".to_string()),
        "should include docs2"
    );

    Ok(())
}

#[tokio::test]
async fn test_combining_heading_level_and_text_filter() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    // Use a structure where the parent doesn't contain "API"
    // so we can test filtering more precisely
    let doc = r#"# Documentation
## API Reference
### API Details
## API Authentication
### Auth Flow
## User Management
### User Permissions
## Database
### Schema
"#;

    seed_source(&tmp, &server, "docs", doc).await?;

    let json = run_toc_json(
        &tmp,
        &[
            "toc",
            "docs",
            "-H",
            "<=2",
            "--filter",
            "(API OR Auth) AND NOT Database",
            "-f",
            "json",
        ],
    )?;
    let entries = json["entries"].as_array().expect("expected entries array");

    fn collect_entries(entries: &[Value]) -> Vec<(u64, String, Vec<String>)> {
        let mut result = Vec::new();
        for entry in entries {
            if let (Some(level), Some(heading_path)) = (
                entry["headingLevel"].as_u64(),
                entry["headingPath"].as_array(),
            ) {
                let path: Vec<String> = heading_path
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();

                // Get the last element as the heading name
                if let Some(heading) = path.last() {
                    result.push((level, heading.clone(), path));
                }
            }
        }
        result
    }

    let entries_list = collect_entries(entries);
    assert!(
        !entries_list.is_empty(),
        "expected at least one result with level <= 2 and path containing 'API'"
    );

    // All entries should have level <= 2
    // Note: The filter matches against the full heading path (joined with space),
    // so "Documentation" -> "API Reference" will match because the path contains "API"
    for (level, _heading, path) in &entries_list {
        assert!(*level <= 2, "Expected level <= 2, got {}", level);

        // The full path (when joined) should contain "API"
        let full_path = path.join(" ");
        assert!(
            full_path.to_lowercase().contains("api"),
            "Expected path '{}' to contain 'API' (case-insensitive)",
            full_path
        );
    }

    // Verify we have the expected headings
    let headings: Vec<String> = entries_list.iter().map(|(_, h, _)| h.clone()).collect();

    // These should be included (level 2 with "API" in path)
    assert!(
        headings.contains(&"API Reference".to_string()),
        "Should include 'API Reference' (level 2, contains 'API')"
    );
    assert!(
        headings.contains(&"API Authentication".to_string()),
        "Should include 'API Authentication' (level 2, contains 'API')"
    );

    // These should NOT be included (level 2 but no "API" in path)
    assert!(
        !headings.contains(&"User Management".to_string()),
        "Should not include 'User Management' (no 'API' in path)"
    );
    assert!(
        !headings.contains(&"Database".to_string()),
        "Should not include 'Database' (no 'API' in path)"
    );

    Ok(())
}

#[tokio::test]
async fn test_tree_view_with_filters() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = r#"# Root
## Child 1
### Grandchild 1
## Child 2
### Grandchild 2
#### Great Grandchild
"#;

    seed_source(&tmp, &server, "docs", doc).await?;

    let output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("NO_COLOR", "1")
        .args(["toc", "docs", "--tree", "-H", "<=3", "-f", "text"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output)?;

    // Should have tree characters
    assert!(
        text.contains("├─") || text.contains("└─"),
        "Tree output should contain branch characters\nGot: {}",
        text
    );

    // Should not include level 4 heading
    assert!(
        !text.contains("Great Grandchild"),
        "Should not include level 4 heading with -H <=3"
    );

    // Should include level 3 headings
    assert!(
        text.contains("Grandchild"),
        "Should include level 3 headings"
    );

    Ok(())
}

#[tokio::test]
async fn test_tree_spacing_h1_to_h1_with_children() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    // Create a document with H1 sections, some with children, some without
    let doc = r#"# H1 No Children 1

# H1 With Children
## H2 Child 1
## H2 Child 2

# H1 No Children 2

# H1 No Children 3
"#;

    seed_source(&tmp, &server, "docs", doc).await?;

    let output = blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .env("NO_COLOR", "1")
        .args(["toc", "docs", "--tree", "-H", "1-2", "-f", "text"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output)?;
    let lines: Vec<&str> = text.lines().collect();

    // Find the line with "H2 Child 2"
    let child2_idx = lines
        .iter()
        .position(|l| l.contains("H2 Child 2"))
        .expect("Should find H2 Child 2");

    // The next non-empty line should be an H1
    let mut next_h1_idx = child2_idx + 1;
    while next_h1_idx < lines.len() && lines[next_h1_idx].trim().is_empty() {
        next_h1_idx += 1;
    }

    // Count blank lines between the last child and next H1
    let blank_count = next_h1_idx - child2_idx - 1;

    // There should be exactly ONE blank line when transitioning from H2 to H1
    // (when the previous H1 had children)
    assert_eq!(
        blank_count,
        1,
        "Expected exactly 1 blank line between H2 child and next H1, found {}\nLines around transition:\n{:?}",
        blank_count,
        &lines[child2_idx..child2_idx.saturating_add(4).min(lines.len())]
    );

    // Verify the next H1 is "H1 No Children 2"
    assert!(
        lines[next_h1_idx].contains("H1 No Children 2"),
        "Expected next line to be 'H1 No Children 2', got: {}",
        lines[next_h1_idx]
    );

    // Also verify no double spacing: look for consecutive blank lines
    for (i, window) in lines.windows(2).enumerate() {
        assert!(
            !(window[0].trim().is_empty() && window[1].trim().is_empty()),
            "Found consecutive blank lines at positions {} and {}\nContext:\n{:?}",
            i,
            i + 1,
            &lines[i.saturating_sub(2)..i.saturating_add(4).min(lines.len())]
        );
    }

    Ok(())
}
