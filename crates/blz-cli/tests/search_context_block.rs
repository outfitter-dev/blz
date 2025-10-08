#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::blz_cmd;
use serde_json::Value;
use std::convert::TryFrom;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

fn run_search_json(tmp: &tempfile::TempDir, args: &[&str]) -> anyhow::Result<Value> {
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

fn doc_lines(doc: &str) -> Vec<&str> {
    doc.lines().collect()
}

fn slice_as_string(lines: &[&str]) -> String {
    lines.join(
        "
",
    )
}

fn range_string(start: usize, end: usize) -> String {
    format!("{start}-{end}")
}

fn assert_context_window(
    context: &Value,
    expected_start: usize,
    expected_end: usize,
    expected_slice: &str,
    expect_truncated: bool,
) {
    assert_eq!(
        context["lines"].as_str().expect("context lines string"),
        range_string(expected_start, expected_end)
    );

    let numbers = context["lineNumbers"]
        .as_array()
        .expect("line numbers array");
    assert_eq!(numbers.len(), expected_end - expected_start + 1);
    for (offset, value) in numbers.iter().enumerate() {
        let parsed = value
            .as_u64()
            .and_then(|num| usize::try_from(num).ok())
            .expect("numeric line number");
        assert_eq!(parsed, expected_start + offset);
    }

    let context_body = context["content"].as_str().expect("context content string");
    assert_eq!(context_body, expected_slice);

    if let Some(value) = context.get("truncated") {
        let flag = value.as_bool().expect("truncated flag should be a boolean");
        assert_eq!(flag, expect_truncated, "unexpected truncation state");
    } else {
        assert!(!expect_truncated, "expected truncation flag");
    }
}

#[tokio::test]
async fn search_context_defaults_to_five_lines() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = "# Title

## Section A
line 1
line 2
line 3
line 4
line 5
line 6
line 7
line 8 target
line 9
line 10

## Section B
line 11
";
    let lines = doc_lines(doc);
    let total_lines = lines.len();

    seed_source(&tmp, &server, "ctx", doc).await?;

    let payload = run_search_json(
        &tmp,
        &[
            "search",
            "target",
            "--source",
            "ctx",
            "--context",
            "-f",
            "json",
        ],
    )?;
    let first = &payload["results"][0];
    let base_lines = first["lines"].as_str().expect("lines string present");
    let mut parts = base_lines.split('-');
    let base_start = parts
        .next()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
    let base_end = parts
        .next()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(base_start);

    let expected_start = base_start.saturating_sub(5).max(1);
    let expected_end = (base_end + 5).min(total_lines);
    let expected_slice = slice_as_string(&lines[expected_start - 1..expected_end]);
    assert_context_window(
        &first["context"],
        expected_start,
        expected_end,
        &expected_slice,
        false,
    );

    let explicit = run_search_json(
        &tmp,
        &[
            "search",
            "target",
            "--source",
            "ctx",
            "--context",
            "2",
            "-f",
            "json",
        ],
    )?;
    let explicit_ctx = &explicit["results"][0]["context"];
    let explicit_start = base_start.saturating_sub(2).max(1);
    let explicit_end = (base_end + 2).min(total_lines);
    let explicit_slice = slice_as_string(&lines[explicit_start - 1..explicit_end]);
    assert_context_window(
        explicit_ctx,
        explicit_start,
        explicit_end,
        &explicit_slice,
        false,
    );

    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn search_block_returns_heading_section_and_honors_max_lines() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;

    let doc = "# Title

## Section A
alpha
beta target
gamma
delta

### Subsection
epsilon

## Section B
zeta
";
    let lines = doc_lines(doc);
    let primary_heading_line = lines
        .iter()
        .position(|line| line == &"## Section A")
        .map(|idx| idx + 1)
        .expect("section heading present");
    let next_heading_line = lines
        .iter()
        .position(|line| line == &"## Section B")
        .map(|idx| idx + 1)
        .expect("next section heading present");
    let section_a_end = next_heading_line.saturating_sub(1);

    seed_source(&tmp, &server, "block", doc).await?;

    let block_json = run_search_json(
        &tmp,
        &[
            "search", "target", "--source", "block", "--block", "-f", "json",
        ],
    )?;
    let block_ctx = &block_json["results"][0]["context"];
    let section_a_effective_end = section_a_end.saturating_sub(1);
    assert_eq!(
        block_ctx["lines"].as_str().unwrap(),
        range_string(primary_heading_line, section_a_effective_end)
    );
    let expected_numbers: Vec<usize> =
        ((primary_heading_line + 1)..=section_a_effective_end).collect();
    let numbers = block_ctx["lineNumbers"]
        .as_array()
        .expect("line numbers array");
    let actual_numbers: Vec<usize> = numbers
        .iter()
        .map(|value| {
            value
                .as_u64()
                .and_then(|num| usize::try_from(num).ok())
                .expect("numeric line number")
        })
        .collect();
    assert_eq!(actual_numbers, expected_numbers);

    let mut expected_lines = vec![lines[primary_heading_line - 1]];
    expected_lines.extend(expected_numbers.iter().map(|line| lines[line - 1]));
    let expected_block = expected_lines.join("\n");
    assert_eq!(block_ctx["content"].as_str().unwrap(), expected_block);
    assert!(block_ctx.get("truncated").is_none());

    let truncated_json = run_search_json(
        &tmp,
        &[
            "search",
            "target",
            "--source",
            "block",
            "--block",
            "--max-lines",
            "3",
            "-f",
            "json",
        ],
    )?;
    let truncated_ctx = &truncated_json["results"][0]["context"];
    let truncated_numbers: Vec<usize> = expected_numbers.iter().copied().take(3).collect();
    let truncated_end = truncated_numbers
        .last()
        .copied()
        .unwrap_or(primary_heading_line);
    assert_eq!(
        truncated_ctx["lines"].as_str().unwrap(),
        range_string(primary_heading_line, truncated_end)
    );
    let actual_truncated: Vec<usize> = truncated_ctx["lineNumbers"]
        .as_array()
        .expect("truncated line numbers")
        .iter()
        .map(|value| {
            value
                .as_u64()
                .and_then(|num| usize::try_from(num).ok())
                .expect("numeric line number")
        })
        .collect();
    assert_eq!(actual_truncated, truncated_numbers);
    let mut truncated_lines = vec![lines[primary_heading_line - 1]];
    truncated_lines.extend(truncated_numbers.iter().map(|line| lines[line - 1]));
    let truncated_expected = truncated_lines.join("\n");
    assert_eq!(
        truncated_ctx["content"].as_str().unwrap(),
        truncated_expected
    );
    assert_eq!(truncated_ctx["truncated"].as_bool(), Some(true));

    Ok(())
}

#[tokio::test]
async fn search_rejects_context_and_block_together() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let server = MockServer::start().await;
    let doc = "# Title

## Section
text target
";

    seed_source(&tmp, &server, "conflict", doc).await?;

    blz_cmd()
        .env("BLZ_DATA_DIR", tmp.path())
        .args([
            "search",
            "target",
            "--source",
            "conflict",
            "--context",
            "--block",
        ])
        .assert()
        .failure();

    Ok(())
}
