use anyhow::Result;
use cache_core::{
    Fetcher, LlmsJson, MarkdownParser, SearchIndex, Source, Storage, FileInfo, LineIndex,
};
use chrono::Utc;
use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "cache")]
#[command(about = "Local-first llms.txt cache and MCP server", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new source
    Add {
        /// Alias for the source
        alias: String,
        /// URL to fetch llms.txt from
        url: String,
    },
    
    /// Search across cached docs
    Search {
        /// Search query
        query: String,
        /// Filter by alias
        #[arg(long)]
        alias: Option<String>,
        /// Maximum number of results
        #[arg(long, default_value = "10")]
        limit: usize,
        /// Output format
        #[arg(long, value_enum, default_value = "pretty")]
        format: OutputFormat,
    },
    
    /// Get exact lines from a source
    Get {
        /// Source alias
        alias: String,
        /// Line range (e.g., "120-142")
        #[arg(long)]
        lines: String,
    },
    
    /// List all cached sources
    Sources {
        /// Output format
        #[arg(long, value_enum, default_value = "pretty")]
        format: OutputFormat,
    },
    
    /// Update sources
    Update {
        /// Specific alias to update (updates all if not specified)
        alias: Option<String>,
        /// Update all sources
        #[arg(long)]
        all: bool,
    },
    
    /// View diffs
    Diff {
        /// Source alias
        alias: String,
        /// Show changes since timestamp
        #[arg(long)]
        since: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum OutputFormat {
    Json,
    Pretty,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)?;
    
    match cli.command {
        Commands::Add { alias, url } => add_source(&alias, &url).await?,
        Commands::Search { query, alias, limit, format } => {
            search(&query, alias.as_deref(), limit, format).await?
        }
        Commands::Get { alias, lines } => get_lines(&alias, &lines).await?,
        Commands::Sources { format } => list_sources(format).await?,
        Commands::Update { alias, all } => {
            if all || alias.is_none() {
                update_all().await?;
            } else if let Some(alias) = alias {
                update_source(&alias).await?;
            }
        }
        Commands::Diff { alias, since } => show_diff(&alias, since.as_deref()).await?,
    }
    
    Ok(())
}

async fn add_source(alias: &str, url: &str) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Fetching {}", url));
    
    let fetcher = Fetcher::new()?;
    let (content, sha256) = fetcher.fetch(url).await?;
    pb.set_message("Parsing markdown");
    
    let mut parser = MarkdownParser::new()?;
    let parse_result = parser.parse(&content)?;
    
    pb.set_message("Building index");
    
    let storage = Storage::new()?;
    storage.save_llms_txt(alias, &content)?;
    
    let llms_json = LlmsJson {
        alias: alias.to_string(),
        source: Source {
            url: url.to_string(),
            etag: None,
            last_modified: None,
            fetched_at: Utc::now(),
            sha256: sha256.clone(),
        },
        toc: parse_result.toc,
        files: vec![FileInfo {
            path: "llms.txt".to_string(),
            sha256,
        }],
        line_index: LineIndex {
            total_lines: parse_result.line_count,
            byte_offsets: false,
        },
        diagnostics: parse_result.diagnostics,
    };
    
    storage.save_llms_json(alias, &llms_json)?;
    
    let index_path = storage.index_dir(alias);
    let mut index = SearchIndex::create(&index_path)?;
    index.index_blocks(alias, "llms.txt", &parse_result.heading_blocks)?;
    
    pb.finish_with_message(format!(
        "✓ Added {} ({} headings, {} lines)",
        alias.green(),
        parse_result.heading_blocks.len(),
        parse_result.line_count
    ));
    
    Ok(())
}

async fn search(query: &str, alias: Option<&str>, limit: usize, format: OutputFormat) -> Result<()> {
    let storage = Storage::new()?;
    
    let sources = if let Some(alias) = alias {
        vec![alias.to_string()]
    } else {
        storage.list_sources()?
    };
    
    if sources.is_empty() {
        println!("No sources found. Use 'cache add' to add sources.");
        return Ok(());
    }
    
    let mut all_hits = Vec::new();
    
    for source in sources {
        let index_path = storage.index_dir(&source);
        if index_path.exists() {
            let index = SearchIndex::open(&index_path)?;
            let hits = index.search(query, Some(&source), limit)?;
            all_hits.extend(hits);
        }
    }
    
    all_hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    all_hits.truncate(limit);
    
    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&all_hits)?;
            println!("{}", json);
        }
        OutputFormat::Pretty => {
            if all_hits.is_empty() {
                println!("No results found for '{}'", query);
            } else {
                println!("\nSearch results for '{}':\n", query.cyan());
                
                for (i, hit) in all_hits.iter().enumerate() {
                    println!("{}. {} (score: {:.2})", i + 1, hit.alias.green(), hit.score);
                    println!("   {} {}", "Path:".bright_black(), hit.heading_path.join(" > "));
                    println!("   {} L{}", "Lines:".bright_black(), hit.lines);
                    println!("   {} {}", "Snippet:".bright_black(), 
                        hit.snippet.chars().take(100).collect::<String>());
                    println!();
                }
            }
        }
    }
    
    Ok(())
}

async fn get_lines(alias: &str, lines: &str) -> Result<()> {
    let storage = Storage::new()?;
    
    if !storage.exists(alias) {
        println!("Source '{}' not found", alias);
        return Ok(());
    }
    
    let content = storage.load_llms_txt(alias)?;
    let all_lines: Vec<&str> = content.lines().collect();
    
    let parts: Vec<&str> = lines.split('-').collect();
    if parts.len() != 2 {
        println!("Invalid line range format. Use 'start-end' (e.g., '120-142')");
        return Ok(());
    }
    
    let start: usize = parts[0].parse()?;
    let end: usize = parts[1].parse()?;
    
    if start == 0 || end == 0 || start > end || end > all_lines.len() {
        println!("Invalid line range");
        return Ok(());
    }
    
    for i in (start - 1)..end {
        if i < all_lines.len() {
            println!("{:4} │ {}", i + 1, all_lines[i]);
        }
    }
    
    Ok(())
}

async fn list_sources(format: OutputFormat) -> Result<()> {
    let storage = Storage::new()?;
    let sources = storage.list_sources()?;
    
    if sources.is_empty() {
        println!("No sources found. Use 'cache add' to add sources.");
        return Ok(());
    }
    
    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&sources)?;
            println!("{}", json);
        }
        OutputFormat::Pretty => {
            println!("\nCached sources:\n");
            for source in sources {
                if let Ok(llms_json) = storage.load_llms_json(&source) {
                    println!("  {} {}", source.green(), llms_json.source.url.bright_black());
                    println!("    Fetched: {}", llms_json.source.fetched_at.format("%Y-%m-%d %H:%M:%S"));
                    println!("    Lines: {}", llms_json.line_index.total_lines);
                    println!();
                }
            }
        }
    }
    
    Ok(())
}

async fn update_source(_alias: &str) -> Result<()> {
    println!("Update functionality not yet implemented");
    Ok(())
}

async fn update_all() -> Result<()> {
    println!("Update all functionality not yet implemented");
    Ok(())
}

async fn show_diff(_alias: &str, _since: Option<&str>) -> Result<()> {
    println!("Diff functionality not yet implemented");
    Ok(())
}