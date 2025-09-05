//! MCP (Model Context Protocol) server for blz
//!
//! Provides a JSON-RPC interface for AI assistants to search cached llms.txt documentation.

use anyhow::Result;
use blz_core::{SearchIndex, Storage};
use jsonrpc_core::{Error as RpcError, ErrorCode, IoHandler, Params, Value};
use jsonrpc_stdio_server::ServerBuilder;
use serde_json::json;
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber;

/// Handle the `list_sources` RPC method
async fn handle_list_sources(_params: Params) -> Result<Value, RpcError> {
    let storage = Storage::new().map_err(|e| {
        error!("Failed to create storage: {}", e);
        RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to access storage: {e}"),
            data: None,
        }
    })?;

    let sources = storage.list_sources();

    let mut result = Vec::new();
    for source in sources {
        if let Ok(llms_json) = storage.load_llms_json(&source) {
            let path = storage.llms_txt_path(&source).map_or_else(
                |_| format!("~/.outfitter/blz/{source}/llms.txt"),
                |p| p.to_string_lossy().to_string(),
            );

            result.push(json!({
                "alias": source,
                "path": path,
                "fetchedAt": llms_json.source.fetched_at,
                "etag": llms_json.source.etag,
                "size": llms_json.line_index.total_lines,
            }));
        }
    }

    Ok(Value::Array(result))
}

/// Handle the search RPC method
async fn handle_search(params: Params) -> Result<Value, RpcError> {
    let params = params.parse::<serde_json::Value>().map_err(|e| RpcError {
        code: ErrorCode::InvalidParams,
        message: format!("Invalid parameters: {e}"),
        data: None,
    })?;

    let query = params["query"].as_str().ok_or_else(|| RpcError {
        code: ErrorCode::InvalidParams,
        message: "Missing required parameter 'query'".to_string(),
        data: None,
    })?;

    let alias = params.get("alias").and_then(|v| v.as_str());
    let limit = params
        .get("limit")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(10)
        .min(100) // Reasonable limit for search results
        as usize;

    let storage = Storage::new().map_err(|e| {
        error!("Failed to create storage: {}", e);
        RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to access storage: {e}"),
            data: None,
        }
    })?;

    let sources = alias.map_or_else(|| storage.list_sources(), |a| vec![a.to_string()]);

    let mut all_hits = Vec::new();

    for source in sources {
        let index_path = match storage.index_dir(&source) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to get index dir for {}: {}", source, e);
                continue;
            },
        };

        if index_path.exists() {
            let index = match SearchIndex::open(&index_path) {
                Ok(i) => i,
                Err(e) => {
                    error!("Failed to open index for {}: {}", source, e);
                    continue;
                },
            };

            let hits = match index.search(query, Some(&source), limit) {
                Ok(h) => h,
                Err(e) => {
                    error!("Search failed for {}: {}", source, e);
                    continue;
                },
            };

            for hit in hits {
                all_hits.push(json!({
                    "alias": hit.alias,
                    "file": hit.file,
                    "headingPath": hit.heading_path,
                    "lines": hit.lines,
                    "snippet": hit.snippet,
                    "score": hit.score,
                    "sourceUrl": hit.source_url,
                    "checksum": hit.checksum,
                }));
            }
        }
    }

    Ok(json!({
        "hits": all_hits
    }))
}

/// Handle the `get_lines` RPC method
async fn handle_get_lines(params: Params) -> Result<Value, RpcError> {
    let params = params.parse::<serde_json::Value>().map_err(|e| RpcError {
        code: ErrorCode::InvalidParams,
        message: format!("Invalid parameters: {e}"),
        data: None,
    })?;

    let alias = params["alias"].as_str().ok_or_else(|| RpcError {
        code: ErrorCode::InvalidParams,
        message: "Missing required parameter 'alias'".to_string(),
        data: None,
    })?;

    let _file = params
        .get("file")
        .and_then(|v| v.as_str())
        .unwrap_or("llms.txt");

    let start = usize::try_from(params["start"].as_u64().ok_or_else(|| RpcError {
        code: ErrorCode::InvalidParams,
        message: "Missing required parameter 'start'".to_string(),
        data: None,
    })?)
    .map_err(|_| RpcError {
        code: ErrorCode::InvalidParams,
        message: "Invalid start value: too large for platform".to_string(),
        data: None,
    })?;

    let end = usize::try_from(params["end"].as_u64().ok_or_else(|| RpcError {
        code: ErrorCode::InvalidParams,
        message: "Missing required parameter 'end'".to_string(),
        data: None,
    })?)
    .map_err(|_| RpcError {
        code: ErrorCode::InvalidParams,
        message: "Invalid end value: too large for platform".to_string(),
        data: None,
    })?;

    if start == 0 || start > end {
        return Err(RpcError {
            code: ErrorCode::InvalidParams,
            message: format!("Invalid line range: {start}-{end}"),
            data: None,
        });
    }

    let storage = Storage::new().map_err(|e| {
        error!("Failed to create storage: {}", e);
        RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to access storage: {e}"),
            data: None,
        }
    })?;

    let content = storage.load_llms_txt(alias).map_err(|e| {
        error!("Failed to load content for {}: {}", alias, e);
        RpcError {
            code: ErrorCode::InternalError,
            message: format!("Failed to load content: {e}"),
            data: None,
        }
    })?;

    let lines: Vec<&str> = content.lines().collect();

    let mut result = String::new();
    let end_line = end.min(lines.len());
    for line in lines.iter().skip(start - 1).take(end_line - (start - 1)) {
        result.push_str(line);
        result.push('\n');
    }

    Ok(json!({
        "content": result,
        "mimeType": "text/plain"
    }))
}

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting blz MCP server");

    let mut io = IoHandler::new();

    io.add_method("list_sources", handle_list_sources);

    io.add_method("search", handle_search);

    io.add_method("get_lines", handle_get_lines);

    let _server = ServerBuilder::new(io).build();

    // Keep the server alive - graceful Ctrl+C handling will come in a future PR
    std::thread::park();

    Ok(())
}
