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

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting blz MCP server");

    let mut io = IoHandler::new();

    io.add_method("list_sources", |_params: Params| async {
        let storage = match Storage::new() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create storage: {}", e);
                return Err(RpcError {
                    code: ErrorCode::InternalError,
                    message: format!("Failed to access storage: {}", e),
                    data: None,
                });
            },
        };

        let sources = match storage.list_sources() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to list sources: {}", e);
                return Err(RpcError {
                    code: ErrorCode::InternalError,
                    message: format!("Failed to list sources: {}", e),
                    data: None,
                });
            },
        };

        let mut result = Vec::new();
        for source in sources {
            if let Ok(llms_json) = storage.load_llms_json(&source) {
                let path = storage
                    .llms_txt_path(&source)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| format!("~/.outfitter/blz/{}/llms.txt", source));

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
    });

    io.add_method("search", |params: Params| async {
        let params = match params.parse::<serde_json::Value>() {
            Ok(p) => p,
            Err(e) => {
                return Err(RpcError {
                    code: ErrorCode::InvalidParams,
                    message: format!("Invalid parameters: {}", e),
                    data: None,
                });
            },
        };

        let query = match params["query"].as_str() {
            Some(q) => q,
            None => {
                return Err(RpcError {
                    code: ErrorCode::InvalidParams,
                    message: "Missing required parameter 'query'".to_string(),
                    data: None,
                });
            },
        };

        let alias = params.get("alias").and_then(|v| v.as_str());
        let limit = params
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(10) as usize;

        let storage = match Storage::new() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create storage: {}", e);
                return Err(RpcError {
                    code: ErrorCode::InternalError,
                    message: format!("Failed to access storage: {}", e),
                    data: None,
                });
            },
        };

        let sources = if let Some(alias) = alias {
            vec![alias.to_string()]
        } else {
            match storage.list_sources() {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to list sources: {}", e);
                    return Err(RpcError {
                        code: ErrorCode::InternalError,
                        message: format!("Failed to list sources: {}", e),
                        data: None,
                    });
                },
            }
        };

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
    });

    io.add_method("get_lines", |params: Params| async {
        let params = match params.parse::<serde_json::Value>() {
            Ok(p) => p,
            Err(e) => {
                return Err(RpcError {
                    code: ErrorCode::InvalidParams,
                    message: format!("Invalid parameters: {}", e),
                    data: None,
                });
            },
        };

        let alias = match params["alias"].as_str() {
            Some(a) => a,
            None => {
                return Err(RpcError {
                    code: ErrorCode::InvalidParams,
                    message: "Missing required parameter 'alias'".to_string(),
                    data: None,
                });
            },
        };

        let _file = params
            .get("file")
            .and_then(|v| v.as_str())
            .unwrap_or("llms.txt");

        let start = match params["start"].as_u64() {
            Some(s) => s as usize,
            None => {
                return Err(RpcError {
                    code: ErrorCode::InvalidParams,
                    message: "Missing required parameter 'start'".to_string(),
                    data: None,
                });
            },
        };

        let end = match params["end"].as_u64() {
            Some(e) => e as usize,
            None => {
                return Err(RpcError {
                    code: ErrorCode::InvalidParams,
                    message: "Missing required parameter 'end'".to_string(),
                    data: None,
                });
            },
        };

        if start == 0 || start > end {
            return Err(RpcError {
                code: ErrorCode::InvalidParams,
                message: format!("Invalid line range: {}-{}", start, end),
                data: None,
            });
        }

        let storage = match Storage::new() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create storage: {}", e);
                return Err(RpcError {
                    code: ErrorCode::InternalError,
                    message: format!("Failed to access storage: {}", e),
                    data: None,
                });
            },
        };

        let content = match storage.load_llms_txt(alias) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to load content for {}: {}", alias, e);
                return Err(RpcError {
                    code: ErrorCode::InternalError,
                    message: format!("Failed to load content: {}", e),
                    data: None,
                });
            },
        };

        let lines: Vec<&str> = content.lines().collect();

        let mut result = String::new();
        for i in (start - 1)..end.min(lines.len()) {
            result.push_str(lines[i]);
            result.push('\n');
        }

        Ok(json!({
            "content": result,
            "mimeType": "text/plain"
        }))
    });

    let _server = ServerBuilder::new(io).build();

    // Keep the server alive - graceful Ctrl+C handling will come in a future PR
    std::thread::park();

    Ok(())
}
