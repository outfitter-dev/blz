use anyhow::Result;
use blz_core::{SearchIndex, Storage};
use jsonrpc_core::{IoHandler, Params, Value};
use jsonrpc_stdio_server::ServerBuilder;
use serde_json::json;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("Starting blz MCP server");
    
    let mut io = IoHandler::new();
    
    io.add_method("list_sources", |_params: Params| async {
        let storage = Storage::new().unwrap();
        let sources = storage.list_sources().unwrap();
        
        let mut result = Vec::new();
        for source in sources {
            if let Ok(llms_json) = storage.load_llms_json(&source) {
                result.push(json!({
                    "alias": source,
                    "path": storage.llms_txt_path(&source),
                    "fetchedAt": llms_json.source.fetched_at,
                    "etag": llms_json.source.etag,
                    "size": llms_json.line_index.total_lines,
                }));
            }
        }
        
        Ok(Value::Array(result))
    });
    
    io.add_method("search", |params: Params| async {
        let params = params.parse::<serde_json::Value>().unwrap();
        let query = params["query"].as_str().unwrap();
        let alias = params.get("alias").and_then(|v| v.as_str());
        let limit = params.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        
        let storage = Storage::new().unwrap();
        let sources = if let Some(alias) = alias {
            vec![alias.to_string()]
        } else {
            storage.list_sources().unwrap()
        };
        
        let mut all_hits = Vec::new();
        
        for source in sources {
            let index_path = storage.index_dir(&source);
            if index_path.exists() {
                let index = SearchIndex::open(&index_path).unwrap();
                let hits = index.search(query, Some(&source), limit).unwrap();
                
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
        let params = params.parse::<serde_json::Value>().unwrap();
        let alias = params["alias"].as_str().unwrap();
        let _file = params.get("file").and_then(|v| v.as_str()).unwrap_or("llms.txt");
        let start = params["start"].as_u64().unwrap() as usize;
        let end = params["end"].as_u64().unwrap() as usize;
        
        let storage = Storage::new().unwrap();
        let content = storage.load_llms_txt(alias).unwrap();
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
    
    let server = ServerBuilder::new(io);
    let _ = server.build();
    
    Ok(())
}