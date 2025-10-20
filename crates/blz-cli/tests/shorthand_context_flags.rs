#![allow(missing_docs, clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::blz_cmd_with_dirs;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn write_fixture(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, contents).expect("failed to write fixture");
    path
}

fn run_search(data_dir: &Path, config_dir: &Path, args: &[&str]) -> Value {
    let output = blz_cmd_with_dirs(data_dir, config_dir)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).expect("valid json output")
}

#[tokio::test]
async fn shorthand_context_flags_are_recognized_without_json_alias() {
    let data_dir = tempdir().expect("data dir");
    let config_dir = tempdir().expect("config dir");

    let doc = r"# Title

## Section A
alpha beta gamma
context target line
closing text
";

    let _local = write_fixture(data_dir.path(), "ctx.md", doc);

    let server = MockServer::start().await;
    Mock::given(method("HEAD"))
        .and(path("/llms.txt"))
        .respond_with(
            ResponseTemplate::new(200).insert_header("content-length", doc.len().to_string()),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/llms.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(doc))
        .mount(&server)
        .await;

    blz_cmd_with_dirs(data_dir.path(), config_dir.path())
        .args([
            "add",
            "ctx",
            format!("{}/llms.txt", server.uri()).as_str(),
            "-y",
        ])
        .assert()
        .success();

    let payload = run_search(
        data_dir.path(),
        config_dir.path(),
        &[
            "target",
            "--context",
            "all",
            "--format",
            "json",
            "--source",
            "ctx",
        ],
    );
    assert!(payload["results"].as_array().is_some_and(|a| !a.is_empty()));

    let payload = run_search(
        data_dir.path(),
        config_dir.path(),
        &["target", "-C5", "--format", "json", "--source", "ctx"],
    );
    assert!(payload["results"].as_array().is_some_and(|a| !a.is_empty()));

    let payload = run_search(
        data_dir.path(),
        config_dir.path(),
        &[
            "target", "-A2", "-B1", "--format", "json", "--source", "ctx",
        ],
    );
    assert!(payload["results"].as_array().is_some_and(|a| !a.is_empty()));
}
