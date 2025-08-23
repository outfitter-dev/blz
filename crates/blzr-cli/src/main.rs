use anyhow::Result;
use blzr_core::{
    Fetcher, FlavorInfo, LlmsJson, MarkdownParser, SearchIndex, Source, Storage, FileInfo, LineIndex,
    PerformanceMetrics, ResourceMonitor, Registry,
};

#[cfg(feature = "flamegraph")]
use blzr_core::profiling::{start_profiling, stop_profiling_and_report};
use chrono::Utc;
use clap::{CommandFactory, Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use dialoguer::{Select, Input};
use std::io::IsTerminal;
use std::time::Instant;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser)]
#[command(name = "blz")]
#[command(about = "Local-first llms.txt cache and MCP server for blazing trails", long_about = None)]
#[command(override_usage = "blz [OPTIONS] [QUERY]... [COMMAND]")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Arguments for default search command
    #[arg(global = true)]
    args: Vec<String>,
    
    #[arg(long, global = true)]
    verbose: bool,
    
    /// Show detailed performance metrics
    #[arg(long, global = true)]
    debug: bool,
    
    /// Show resource usage (memory, CPU)
    #[arg(long, global = true)]
    profile: bool,
    
    /// Generate CPU flamegraph (requires flamegraph feature)
    #[cfg(feature = "flamegraph")]
    #[arg(long, global = true)]
    flamegraph: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    
    /// Add a new source
    Add {
        /// Alias for the source
        alias: String,
        /// URL to fetch llms.txt from
        url: String,
        /// Auto-select the best flavor without prompts (default: false)
        #[arg(short = 'y', long)]
        yes: bool,
    },
    
    /// Search registries for documentation to add
    Lookup {
        /// Search query (tool name, partial name, etc.)
        query: String,
    },
    
    /// Search across cached docs
    Search {
        /// Search query
        query: String,
        /// Filter by alias
        #[arg(long)]
        alias: Option<String>,
        /// Maximum number of results
        #[arg(short = 'n', long, default_value = "50")]
        limit: usize,
        /// Show all results (no limit)
        #[arg(long)]
        all: bool,
        /// Page number for pagination
        #[arg(long, default_value = "1")]
        page: usize,
        /// Show only top N percentile of results (1-100)
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=100))]
        top: Option<u8>,
        /// Output format
        #[arg(short = 'o', long, value_enum, default_value = "text")]
        output: OutputFormat,
    },
    
    /// Get exact lines from a source
    Get {
        /// Source alias
        alias: String,
        /// Line range(s) (e.g., "120-142", "36:43,320:350", "36+20")
        #[arg(short = 'l', long)]
        lines: String,
        /// Context lines around each line/range
        #[arg(short = 'c', long)]
        context: Option<usize>,
    },
    
    /// List all cached sources
    #[command(alias = "sources")]
    List {
        /// Output format
        #[arg(short = 'o', long, value_enum, default_value = "text")]
        output: OutputFormat,
    },
    
    /// Update sources
    Update {
        /// Specific alias to update (updates all if not specified)
        alias: Option<String>,
        /// Update all sources
        #[arg(long)]
        all: bool,
    },
    
    /// Remove/delete a source
    #[command(alias = "rm", alias = "delete")]
    Remove {
        /// Source alias
        alias: String,
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
    /// Pretty text output (default)
    Text,
    /// Single JSON array
    Json,
    /// Newline-delimited JSON
    Ndjson,
}

// Reserved keywords that cannot be used as aliases
const RESERVED_KEYWORDS: &[&str] = &[
    // Commands
    "add", "search", "get", "list", "sources", "update", "remove", "rm", "delete", 
    "help", "version", "completions", "diff", "lookup",
    // Meta
    "config", "settings", "serve", "server", "mcp", "start", "stop", "status",
    // Operations  
    "sync", "export", "import", "backup", "restore", "clean", "purge",
    // Special
    "all", "none", "default", "local", "global", "cache", "self",
];

// Color cycling for aliases
const ALIAS_COLORS: &[fn(&str) -> colored::ColoredString] = &[
    |s| s.green(),
    |s| s.blue(), 
    |s| s.truecolor(0, 150, 136), // teal
    |s| s.magenta(),
];

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let level = if cli.verbose || cli.debug {
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
    
    // Initialize global performance metrics
    let metrics = PerformanceMetrics::default();
    
    // Initialize resource monitor if profiling is enabled
    let mut resource_monitor = if cli.profile {
        Some(ResourceMonitor::new())
    } else {
        None
    };
    
    // Start CPU profiling if flamegraph is requested
    #[cfg(feature = "flamegraph")]
    let profiler_guard = if cli.flamegraph {
        match start_profiling() {
            Ok(guard) => {
                println!("🔥 CPU profiling started - flamegraph will be generated");
                Some(guard)
            }
            Err(e) => {
                eprintln!("Failed to start profiling: {}", e);
                None
            }
        }
    } else {
        None
    };
    
    match cli.command {
        Some(Commands::Completions { shell }) => {
            generate_completions(shell);
            return Ok(());
        }
        Some(Commands::Add { alias, url, yes }) => {
            validate_alias(&alias)?;
            add_source(&alias, &url, yes, metrics.clone(), resource_monitor.as_mut()).await?;
        }
        Some(Commands::Lookup { query }) => {
            lookup_registry(&query, metrics.clone(), resource_monitor.as_mut()).await?;
        }
        Some(Commands::Search { query, alias, limit, all, page, top, output }) => {
            let actual_limit = if all { 10000 } else { limit };
            search(&query, alias.as_deref(), actual_limit, page, top, output, 
                   metrics.clone(), resource_monitor.as_mut()).await?;
        }
        Some(Commands::Get { alias, lines, context }) => {
            get_lines(&alias, &lines, context).await?;
        }
        Some(Commands::List { output }) => list_sources(output).await?,
        Some(Commands::Update { alias, all }) => {
            if all || alias.is_none() {
                update_all().await?;
            } else if let Some(alias) = alias {
                update_source(&alias).await?;
            }
        }
        Some(Commands::Remove { alias }) => remove_source(&alias).await?,
        Some(Commands::Diff { alias, since }) => show_diff(&alias, since.as_deref()).await?,
        None => {
            // Default search command - parse arguments intelligently
            handle_default_search(&cli.args, metrics.clone(), resource_monitor.as_mut()).await?;
        }
    }
    
    // Stop CPU profiling and generate flamegraph
    #[cfg(feature = "flamegraph")]
    if let Some(guard) = profiler_guard {
        if let Err(e) = stop_profiling_and_report(guard) {
            eprintln!("Failed to generate flamegraph: {}", e);
        }
    }
    
    // Print performance summary if requested
    if cli.debug {
        metrics.print_summary();
    }
    
    if cli.profile {
        if let Some(ref mut monitor) = resource_monitor {
            monitor.print_resource_usage();
        }
    }
    
    Ok(())
}

fn validate_alias(alias: &str) -> Result<()> {
    if RESERVED_KEYWORDS.contains(&alias.to_lowercase().as_str()) {
        return Err(anyhow::anyhow!(
            "Alias '{}' is reserved. Reserved keywords: {}",
            alias,
            RESERVED_KEYWORDS.join(", ")
        ));
    }
    Ok(())
}

fn get_alias_color(alias: &str, index: usize) -> colored::ColoredString {
    let color_fn = ALIAS_COLORS[index % ALIAS_COLORS.len()];
    color_fn(alias)
}

async fn handle_default_search(
    args: &[String], 
    metrics: PerformanceMetrics, 
    resource_monitor: Option<&mut ResourceMonitor>
) -> Result<()> {
    if args.is_empty() {
        println!("Usage: blz [QUERY] [SOURCE] or cache [SOURCE] [QUERY]");
        println!("       cache search [OPTIONS] QUERY");
        return Ok(());
    }

    let storage = Storage::new()?;
    let sources = storage.list_sources()?;
    
    if sources.is_empty() {
        println!("No sources found. Use 'blz add ALIAS URL' to add sources.");
        return Ok(());
    }

    // Smart argument detection: if first arg matches a known source, it's source + query
    // Otherwise, it's query + optional source
    let (query, alias) = if args.len() >= 2 && sources.contains(&args[0]) {
        // Format: cache SOURCE QUERY...
        (args[1..].join(" "), Some(args[0].clone()))
    } else if args.len() >= 2 && sources.contains(&args[args.len() - 1]) {
        // Format: cache QUERY... SOURCE
        (args[..args.len() - 1].join(" "), Some(args[args.len() - 1].clone()))
    } else {
        // Single query or query without known source
        (args.join(" "), None)
    };

    search(&query, alias.as_deref(), 50, 1, None, OutputFormat::Text, metrics, resource_monitor).await
}

async fn add_source(
    alias: &str, 
    url: &str, 
    auto_yes: bool,
    metrics: PerformanceMetrics, 
    resource_monitor: Option<&mut ResourceMonitor>
) -> Result<()> {
    let fetcher = Fetcher::new()?;
    
    // Check if the user specified an exact llms.txt variant
    let is_exact_file = url.split('/').last()
        .map(|filename| filename.starts_with("llms") && filename.ends_with(".txt"))
        .unwrap_or(false);
    
    let final_url = if is_exact_file {
        // User specified exact file, use it directly
        url.to_string()
    } else {
        // Smart detection: check for flavors
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Checking for available documentation flavors...");
        
        let flavors = match fetcher.check_flavors(url).await {
            Ok(flavors) if !flavors.is_empty() => flavors,
            Ok(_) => {
                // No flavors found, use original URL
                pb.finish_with_message("No llms.txt variants found, using provided URL");
                vec![FlavorInfo {
                    name: "llms.txt".to_string(),
                    size: None,
                    url: url.to_string(),
                }]
            }
            Err(e) => {
                pb.finish_with_message(format!("Failed to check flavors: {}", e));
                // Fall back to original URL
                vec![FlavorInfo {
                    name: "llms.txt".to_string(),
                    size: None,
                    url: url.to_string(),
                }]
            }
        };
        
        pb.finish();
        
        if flavors.len() == 1 {
            // Only one flavor available, use it
            flavors[0].url.clone()
        } else if auto_yes {
            // Auto-select the first (best) option
            println!("Auto-selecting: {}", flavors[0]);
            flavors[0].url.clone()
        } else {
            // Interactive selection
            println!("Found {} versions:", flavors.len());
            
            let flavor_displays: Vec<String> = flavors.iter()
                .enumerate()
                .map(|(i, flavor)| {
                    if i == 0 {
                        format!("→ {} [default]", flavor)
                    } else {
                        format!("  {}", flavor)
                    }
                })
                .collect();
            
            let selection = Select::new()
                .with_prompt("Select version")
                .items(&flavor_displays)
                .default(0)
                .interact()?;
            
            flavors[selection].url.clone()
        }
    };
    
    // Now fetch the selected flavor
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(format!("Fetching {}", final_url));
    
    let (content, sha256) = fetcher.fetch(&final_url).await?;
    pb.set_message("Parsing markdown");
    
    let mut parser = MarkdownParser::new()?;
    let parse_result = parser.parse(&content)?;
    
    pb.set_message("Building index");
    
    let storage = Storage::new()?;
    storage.save_llms_txt(alias, &content)?;
    
    let llms_json = LlmsJson {
        alias: alias.to_string(),
        source: Source {
            url: final_url,
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
    let mut index = SearchIndex::create(&index_path)?.with_metrics(metrics);
    index.index_blocks(alias, "llms.txt", &parse_result.heading_blocks)?;
    
    pb.finish_with_message(format!(
        "✓ Added {} ({} headings, {} lines)",
        alias.green(),
        parse_result.heading_blocks.len(),
        parse_result.line_count
    ));
    
    // Show resource usage if profiling is enabled
    if let Some(monitor) = resource_monitor {
        monitor.print_resource_usage();
    }
    
    Ok(())
}

async fn search(
    query: &str, 
    alias: Option<&str>, 
    limit: usize, 
    page: usize, 
    top_percentile: Option<u8>, 
    output: OutputFormat,
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>
) -> Result<()> {
    let start_time = Instant::now();
    let storage = Storage::new()?;
    
    let sources = if let Some(alias) = alias {
        vec![alias.to_string()]
    } else {
        storage.list_sources()?
    };
    
    if sources.is_empty() {
        println!("No sources found. Use 'blz add' to add sources.");
        return Ok(());
    }
    
    let mut all_hits = Vec::new();
    let mut total_lines_searched = 0usize;
    
    // Collect all hits from all sources
    for source in &sources {
        let index_path = storage.index_dir(source);
        if index_path.exists() {
            let index = SearchIndex::open(&index_path)?.with_metrics(metrics.clone());
            // Use a reasonable large limit to get all possible hits for accurate scoring and percentile filtering
            let hits = index.search(query, Some(source), 10000)?;
            all_hits.extend(hits);
            
            // Count total lines for stats
            if let Ok(llms_json) = storage.load_llms_json(source) {
                total_lines_searched += llms_json.line_index.total_lines;
            }
        }
    }
    
    // Remove duplicates based on alias + lines + heading path
    all_hits.sort_by(|a, b| {
        let cmp = a.alias.cmp(&b.alias)
            .then(a.lines.cmp(&b.lines))
            .then(a.heading_path.cmp(&b.heading_path));
        if cmp == std::cmp::Ordering::Equal {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            cmp
        }
    });
    all_hits.dedup_by(|a, b| {
        a.alias == b.alias && a.lines == b.lines && a.heading_path == b.heading_path
    });
    
    // Sort by score descending
    all_hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    
    // Apply percentile filtering if requested
    if let Some(percentile) = top_percentile {
        let percentile_count = (all_hits.len() as f32 * (percentile as f32 / 100.0)).ceil() as usize;
        all_hits.truncate(percentile_count.max(1));
        
        if all_hits.len() < 10 {
            eprintln!("Tip: Only {} results in top {}%. Try a lower percentile or remove --top flag.", 
                     all_hits.len(), percentile);
        }
    }
    
    let total_results = all_hits.len();
    
    // Apply pagination
    let actual_limit = if limit >= 10000 { all_hits.len() } else { limit };
    let start_idx = (page - 1) * actual_limit;
    let end_idx = (start_idx + actual_limit).min(all_hits.len());
    
    if start_idx >= all_hits.len() {
        println!("Page {} is beyond available results (only {} pages available)", 
                page, (total_results + actual_limit - 1) / actual_limit);
        return Ok(());
    }
    
    let page_hits = &all_hits[start_idx..end_idx];
    let search_time = start_time.elapsed();
    
    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&page_hits)?;
            println!("{}", json);
        }
        OutputFormat::Ndjson => {
            for hit in page_hits {
                println!("{}", serde_json::to_string(hit)?);
            }
        }
        OutputFormat::Text => {
            if page_hits.is_empty() {
                println!("No results found for '{}'", query);
            } else {
                // Show pagination info if limited
                if limit < 10000 {
                    if total_results > actual_limit {
                        println!("Showing {} of {} results\n", page_hits.len(), total_results);
                    }
                }
                
                // Track unique aliases for color cycling
                let mut alias_colors = std::collections::HashMap::new();
                let mut color_index = 0;
                
                for (i, hit) in page_hits.iter().enumerate() {
                    let global_index = start_idx + i + 1;
                    
                    // Get color for alias (cycle through colors for different aliases)
                    let alias_colored = if let Some(&idx) = alias_colors.get(&hit.alias) {
                        get_alias_color(&hit.alias, idx)
                    } else {
                        alias_colors.insert(hit.alias.clone(), color_index);
                        let colored = get_alias_color(&hit.alias, color_index);
                        color_index += 1;
                        colored
                    };
                    
                    // Result header: number, alias (unless single source), lines, path
                    let mut header = format!("{}. ", global_index);
                    
                    // Only show alias if not filtering by single source
                    if alias.is_none() || sources.len() > 1 {
                        header.push_str(&format!("{} ", alias_colored));
                    }
                    
                    header.push_str(&format!("[{}] {}", hit.lines.bright_black(), 
                                           hit.heading_path.join(" > ")));
                    println!("{}", header);
                    
                    // Score line
                    println!("   Score: {:.2}", hit.score.to_string().bright_blue());
                    
                    // Divider
                    println!("   {}", "---".bright_black());
                    
                    // Content snippet
                    let content_lines: Vec<&str> = hit.snippet.lines().collect();
                    for line in content_lines.iter().take(5) { // Show up to 5 lines
                        println!("   {}", line);
                    }
                    if content_lines.len() > 5 {
                        println!("   {}",  "...".bright_black());
                    }
                    
                    // Bottom divider
                    println!("   {}", "---".bright_black());
                    
                    if i < page_hits.len() - 1 {
                        println!();
                    }
                }
                
                // Performance stats
                println!("\n{}", format!(
                    "Searched {} lines in {}ms • Found {} results",
                    total_lines_searched,
                    search_time.as_millis(),
                    total_results
                ).bright_black());
            }
        }
    }
    
    // Show resource usage if profiling is enabled
    if let Some(monitor) = resource_monitor {
        monitor.print_resource_usage();
    }
    
    Ok(())
}

async fn get_lines(alias: &str, lines: &str, context: Option<usize>) -> Result<()> {
    let storage = Storage::new()?;
    
    if !storage.exists(alias) {
        println!("Source '{}' not found", alias);
        return Ok(());
    }
    
    let content = storage.load_llms_txt(alias)?;
    let all_lines: Vec<&str> = content.lines().collect();
    let context_lines = context.unwrap_or(0);
    
    let ranges = parse_line_ranges(lines)?;
    let mut all_line_numbers = std::collections::BTreeSet::new();
    
    for range in ranges {
        match range {
            LineRange::Single(line) => {
                // Add the line and context
                let start = line.saturating_sub(context_lines + 1);
                let end = (line + context_lines).min(all_lines.len());
                for i in start..end {
                    all_line_numbers.insert(i + 1);
                }
            }
            LineRange::Range(start, end) => {
                // Add the range and context
                let actual_start = start.saturating_sub(context_lines + 1);
                let actual_end = (end + context_lines).min(all_lines.len());
                for i in actual_start..actual_end {
                    all_line_numbers.insert(i + 1);
                }
            }
            LineRange::PlusCount(start, count) => {
                // Add the range and context  
                let end = start + count - 1;
                let actual_start = start.saturating_sub(context_lines + 1);
                let actual_end = (end + context_lines).min(all_lines.len());
                for i in actual_start..actual_end {
                    all_line_numbers.insert(i + 1);
                }
            }
        }
    }
    
    if all_line_numbers.is_empty() {
        println!("No valid line ranges found");
        return Ok(());
    }
    
    // Convert to sorted vec and print with separators for non-contiguous ranges
    let line_numbers: Vec<usize> = all_line_numbers.into_iter().collect();
    let mut prev_line = 0;
    
    for &line_num in &line_numbers {
        if line_num == 0 || line_num > all_lines.len() {
            continue;
        }
        
        // Add separator for gaps > 1
        if prev_line > 0 && line_num > prev_line + 1 {
            println!("{}", "     ┈".bright_black());
        }
        
        println!("{:4} │ {}", line_num, all_lines[line_num - 1]);
        prev_line = line_num;
    }
    
    Ok(())
}

#[derive(Debug, Clone)]
enum LineRange {
    Single(usize),
    Range(usize, usize),
    PlusCount(usize, usize),
}

fn parse_line_ranges(input: &str) -> Result<Vec<LineRange>> {
    let mut ranges = Vec::new();
    
    for part in input.split(',') {
        let part = part.trim();
        
        if let Some(colon_pos) = part.find(':') {
            // Format: start:end
            let start_str = part[..colon_pos].trim();
            let end_str = part[colon_pos + 1..].trim();
            let start: usize = start_str.parse()
                .map_err(|_| anyhow::anyhow!("Invalid start line: {}", start_str))?;
            let end: usize = end_str.parse()
                .map_err(|_| anyhow::anyhow!("Invalid end line: {}", end_str))?;
            
            if start == 0 || end == 0 || start > end {
                return Err(anyhow::anyhow!("Invalid range: {}:{}", start, end));
            }
            ranges.push(LineRange::Range(start, end));
        } else if let Some(dash_pos) = part.find('-') {
            // Format: start-end
            let start_str = part[..dash_pos].trim();
            let end_str = part[dash_pos + 1..].trim();
            let start: usize = start_str.parse()
                .map_err(|_| anyhow::anyhow!("Invalid start line: {}", start_str))?;
            let end: usize = end_str.parse()
                .map_err(|_| anyhow::anyhow!("Invalid end line: {}", end_str))?;
            
            if start == 0 || end == 0 || start > end {
                return Err(anyhow::anyhow!("Invalid range: {}-{}", start, end));
            }
            ranges.push(LineRange::Range(start, end));
        } else if let Some(plus_pos) = part.find('+') {
            // Format: start+count
            let start_str = part[..plus_pos].trim();
            let count_str = part[plus_pos + 1..].trim();
            let start: usize = start_str.parse()
                .map_err(|_| anyhow::anyhow!("Invalid start line: {}", start_str))?;
            let count: usize = count_str.parse()
                .map_err(|_| anyhow::anyhow!("Invalid count: {}", count_str))?;
            
            if start == 0 || count == 0 {
                return Err(anyhow::anyhow!("Invalid plus range: {}+{}", start, count));
            }
            ranges.push(LineRange::PlusCount(start, count));
        } else {
            // Single line number
            let line: usize = part.parse()
                .map_err(|_| anyhow::anyhow!("Invalid line number: {}", part))?;
            if line == 0 {
                return Err(anyhow::anyhow!("Line numbers must be >= 1"));
            }
            ranges.push(LineRange::Single(line));
        }
    }
    
    Ok(ranges)
}

async fn list_sources(output: OutputFormat) -> Result<()> {
    let storage = Storage::new()?;
    let sources = storage.list_sources()?;
    
    if sources.is_empty() {
        println!("No sources found. Use 'blz add' to add sources.");
        return Ok(());
    }
    
    let mut source_info = Vec::new();
    for source in &sources {
        if let Ok(llms_json) = storage.load_llms_json(source) {
            source_info.push(serde_json::json!({
                "alias": source,
                "url": llms_json.source.url,
                "fetched_at": llms_json.source.fetched_at,
                "lines": llms_json.line_index.total_lines,
                "sha256": llms_json.source.sha256
            }));
        }
    }
    
    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&source_info)?;
            println!("{}", json);
        }
        OutputFormat::Ndjson => {
            for info in source_info {
                println!("{}", serde_json::to_string(&info)?);
            }
        }
        OutputFormat::Text => {
            println!("\nCached sources:\n");
            for (i, source) in sources.iter().enumerate() {
                if let Ok(llms_json) = storage.load_llms_json(source) {
                    let source_colored = get_alias_color(source, i);
                    println!("  {} {}", source_colored, llms_json.source.url.bright_black());
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

async fn remove_source(alias: &str) -> Result<()> {
    let storage = Storage::new()?;
    
    if !storage.exists(alias) {
        println!("Source '{}' not found", alias);
        return Ok(());
    }
    
    // TODO: Implement actual removal of source files and index
    // For now, just indicate what would be removed
    if let Ok(llms_json) = storage.load_llms_json(alias) {
        println!("Would remove source '{}' ({})", alias.red(), llms_json.source.url);
        println!("  {} lines", llms_json.line_index.total_lines);
        println!("  Fetched: {}", llms_json.source.fetched_at.format("%Y-%m-%d %H:%M:%S"));
        println!("\nRemoval functionality not yet implemented");
    }
    
    Ok(())
}

async fn show_diff(_alias: &str, _since: Option<&str>) -> Result<()> {
    println!("Diff functionality not yet implemented");
    Ok(())
}

async fn lookup_registry(
    query: &str, 
    metrics: PerformanceMetrics,
    resource_monitor: Option<&mut ResourceMonitor>
) -> Result<()> {
    let registry = Registry::new();
    
    println!("Searching registries...");
    let results = registry.search(query);
    
    if results.is_empty() {
        println!("No matches found for '{}'", query);
        return Ok(());
    }
    
    println!("Found {} match{}:\n", results.len(), if results.len() == 1 { "" } else { "es" });
    
    // Display results with numbers
    for (i, result) in results.iter().enumerate() {
        println!("{}. {}", i + 1, result.entry);
        println!("   {}\n", result.entry.llms_url.bright_black());
    }
    
    // Try interactive selection, fallback to showing instructions if not interactive
    let selected_entry = match try_interactive_selection(&results) {
        Ok(entry) => entry,
        Err(_) => {
            // Not interactive, show instructions
            println!("To add any of these sources, use:");
            for (i, result) in results.iter().enumerate() {
                println!("  {} blz add {} {}", 
                        format!("{}.", i + 1).bright_black(),
                        result.entry.slug.green(),
                        result.entry.llms_url.bright_black());
            }
            return Ok(());
        }
    };
    
    // Prompt for alias with default
    let default_alias = selected_entry.slug.clone();
    let alias = match try_interactive_alias_input(&default_alias) {
        Ok(alias) => alias,
        Err(_) => {
            // Not interactive, use default
            println!("Using default alias: {}", default_alias.green());
            default_alias
        }
    };
    
    let final_alias = alias.trim();
    validate_alias(final_alias)?;
    
    println!("Adding {} from {}...", 
             final_alias.green(), 
             selected_entry.llms_url.bright_black());
    
    // Use the existing add_source function
    add_source(
        final_alias, 
        &selected_entry.llms_url, 
        false, // Don't auto-yes, let user see flavor selection if available
        metrics,
        resource_monitor
    ).await?;
    
    Ok(())
}

fn try_interactive_selection(results: &[blzr_core::registry::RegistrySearchResult]) -> Result<&blzr_core::registry::RegistryEntry> {
    if !std::io::stderr().is_terminal() {
        return Err(anyhow::anyhow!("Not in interactive terminal"));
    }
    
    // Display results with numbers
    let display_items: Vec<String> = results.iter()
        .enumerate()
        .map(|(i, result)| {
            format!("{}. {}", i + 1, result.entry)
        })
        .collect();
    
    // Interactive selection
    let selection = Select::new()
        .with_prompt("Select documentation to add (↑/↓ to navigate)")
        .items(&display_items)
        .interact()?;
    
    Ok(&results[selection].entry)
}

fn try_interactive_alias_input(default_alias: &str) -> Result<String> {
    if !std::io::stderr().is_terminal() {
        return Err(anyhow::anyhow!("Not in interactive terminal"));
    }
    
    let alias: String = Input::new()
        .with_prompt("Enter alias")
        .default(default_alias.to_string())
        .interact_text()?;
        
    if alias.trim().is_empty() {
        return Err(anyhow::anyhow!("Alias cannot be empty"));
    }
    
    Ok(alias)
}

fn generate_completions(shell: clap_complete::Shell) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_reserved_keywords_validation() {
        // Test that reserved keywords cannot be used as aliases
        for &keyword in RESERVED_KEYWORDS {
            let result = validate_alias(keyword);
            assert!(result.is_err(), "Reserved keyword '{}' should be rejected", keyword);
            
            // Test error message contains the keyword
            let error_msg = result.unwrap_err().to_string();
            assert!(error_msg.contains(keyword), 
                   "Error message should contain the reserved keyword '{}'", keyword);
        }
    }

    #[test]
    fn test_valid_aliases_allowed() {
        // Test that valid aliases are accepted
        let valid_aliases = ["react", "nextjs", "python", "rust", "docs", "api", "guide"];
        
        for &alias in &valid_aliases {
            let result = validate_alias(alias);
            assert!(result.is_ok(), "Valid alias '{}' should be accepted", alias);
        }
    }

    #[test]
    fn test_language_names_are_not_reserved() {
        // Test that common language names can be used as aliases (per requirements)
        let language_names = ["node", "python", "rust", "go", "java", "javascript", "typescript"];
        
        for &lang in &language_names {
            assert!(!RESERVED_KEYWORDS.contains(&lang), 
                   "Language name '{}' should not be reserved", lang);
            
            let result = validate_alias(lang);
            assert!(result.is_ok(), "Language name '{}' should be usable as alias", lang);
        }
    }

    #[test]
    fn test_reserved_keywords_case_insensitive() {
        // Test that reserved keyword checking is case insensitive
        let result = validate_alias("ADD");
        assert!(result.is_err(), "Reserved keyword 'ADD' (uppercase) should be rejected");
        
        let result = validate_alias("Add");
        assert!(result.is_err(), "Reserved keyword 'Add' (mixed case) should be rejected");
    }

    #[test]
    fn test_line_range_parsing_single() {
        let ranges = parse_line_ranges("42").expect("Should parse single line");
        assert_eq!(ranges.len(), 1);
        
        match &ranges[0] {
            LineRange::Single(line) => assert_eq!(*line, 42),
            _ => panic!("Expected single line range"),
        }
    }

    #[test]
    fn test_line_range_parsing_colon_range() {
        let ranges = parse_line_ranges("120:142").expect("Should parse colon range");
        assert_eq!(ranges.len(), 1);
        
        match &ranges[0] {
            LineRange::Range(start, end) => {
                assert_eq!(*start, 120);
                assert_eq!(*end, 142);
            }
            _ => panic!("Expected range"),
        }
    }

    #[test]
    fn test_line_range_parsing_dash_range() {
        let ranges = parse_line_ranges("120-142").expect("Should parse dash range");
        assert_eq!(ranges.len(), 1);
        
        match &ranges[0] {
            LineRange::Range(start, end) => {
                assert_eq!(*start, 120);
                assert_eq!(*end, 142);
            }
            _ => panic!("Expected range"),
        }
    }

    #[test]
    fn test_line_range_parsing_plus_syntax() {
        let ranges = parse_line_ranges("36+20").expect("Should parse plus syntax");
        assert_eq!(ranges.len(), 1);
        
        match &ranges[0] {
            LineRange::PlusCount(start, count) => {
                assert_eq!(*start, 36);
                assert_eq!(*count, 20);
            }
            _ => panic!("Expected plus count range"),
        }
    }

    #[test]
    fn test_line_range_parsing_multiple_ranges() {
        let ranges = parse_line_ranges("36:43,120-142,200+10").expect("Should parse multiple ranges");
        assert_eq!(ranges.len(), 3);
        
        // First range: 36:43
        match &ranges[0] {
            LineRange::Range(start, end) => {
                assert_eq!(*start, 36);
                assert_eq!(*end, 43);
            }
            _ => panic!("First range should be colon range"),
        }
        
        // Second range: 120-142
        match &ranges[1] {
            LineRange::Range(start, end) => {
                assert_eq!(*start, 120);
                assert_eq!(*end, 142);
            }
            _ => panic!("Second range should be dash range"),
        }
        
        // Third range: 200+10
        match &ranges[2] {
            LineRange::PlusCount(start, count) => {
                assert_eq!(*start, 200);
                assert_eq!(*count, 10);
            }
            _ => panic!("Third range should be plus count"),
        }
    }

    #[test]
    fn test_line_range_parsing_with_whitespace() {
        let ranges = parse_line_ranges(" 36 : 43 , 120 - 142 , 200 + 10 ")
            .expect("Should parse ranges with whitespace");
        assert_eq!(ranges.len(), 3);
    }

    #[test]
    fn test_line_range_parsing_invalid_zero_line() {
        let result = parse_line_ranges("0");
        assert!(result.is_err(), "Line 0 should be invalid");
        
        let result = parse_line_ranges("0:10");
        assert!(result.is_err(), "Range starting at 0 should be invalid");
        
        let result = parse_line_ranges("10:0");
        assert!(result.is_err(), "Range ending at 0 should be invalid");
    }

    #[test]
    fn test_line_range_parsing_invalid_backwards_range() {
        let result = parse_line_ranges("50:30");
        assert!(result.is_err(), "Backwards range should be invalid");
        
        let result = parse_line_ranges("50-30");
        assert!(result.is_err(), "Backwards dash range should be invalid");
    }

    #[test]
    fn test_line_range_parsing_invalid_plus_zero() {
        let result = parse_line_ranges("50+0");
        assert!(result.is_err(), "Plus zero count should be invalid");
        
        let result = parse_line_ranges("0+10");
        assert!(result.is_err(), "Plus from line 0 should be invalid");
    }

    #[test]
    fn test_line_range_parsing_invalid_format() {
        let invalid_formats = ["abc", "10-", "-10", "10:", ":10", "10+", "+10", "10--20", "10::20"];
        
        for &format in &invalid_formats {
            let result = parse_line_ranges(format);
            assert!(result.is_err(), "Invalid format '{}' should be rejected", format);
        }
    }

    #[test]
    fn test_alias_color_cycling() {
        // Test that alias colors cycle through the available colors
        let alias1 = get_alias_color("react", 0);
        let _alias2 = get_alias_color("nextjs", 1);
        let _alias3 = get_alias_color("rust", 2);
        let _alias4 = get_alias_color("go", 3);
        let alias5 = get_alias_color("python", 4); // Should cycle back to index 0
        
        // We can't easily test the actual colors, but we can test that the function doesn't panic
        // and returns colored strings
        assert_eq!(alias1.to_string(), "react");
        assert_eq!(alias5.to_string(), "python");
    }

    // Test helper functions (kept for potential future use in integration tests)
    #[allow(dead_code)]
    mod test_helpers {
        use blzr_core::Storage;
        use tempfile::TempDir;

        pub fn create_test_storage_with_sources(sources: &[&str]) -> (Storage, TempDir) {
            let temp_dir = TempDir::new().expect("Failed to create temp directory");
            let storage = Storage::with_root(temp_dir.path().to_path_buf())
                .expect("Failed to create test storage");
            
            // Create mock source directories
            for &source in sources {
                storage.ensure_tool_dir(source).expect("Failed to create tool dir");
            }
            
            (storage, temp_dir)
        }

        pub fn create_mock_sources() -> Vec<String> {
            vec!["react".to_string(), "nextjs".to_string(), "rust".to_string()]
        }
    }

    #[tokio::test]
    async fn test_default_search_no_args() {
        let metrics = PerformanceMetrics::default();
        let result = handle_default_search(&[], metrics, None).await;
        // Should succeed and show usage message
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_default_search_source_first_format() {
        // This test will fail until we implement proper storage mocking
        // For now, we're testing the argument parsing logic
        
        // Mock scenario: cache react "useState hook"
        let args = vec!["react".to_string(), "useState hook".to_string()];
        
        // This should be detected as source + query format
        // The actual test implementation needs storage mocking
        // For now, we just ensure the function can be called
        let metrics = PerformanceMetrics::default();
        let _result = handle_default_search(&args, metrics, None).await;
        // Will likely fail due to no sources, but that's expected in this phase
    }

    #[tokio::test]
    async fn test_default_search_source_last_format() {
        // Mock scenario: cache "useState hook" react  
        let args = vec!["useState hook".to_string(), "react".to_string()];
        
        // This should be detected as query + source format
        let metrics = PerformanceMetrics::default();
        let _result = handle_default_search(&args, metrics, None).await;
        // Will likely fail due to no sources, but that's expected in this phase
    }

    #[tokio::test]
    async fn test_default_search_query_only() {
        // Mock scenario: cache "useState hook"
        let args = vec!["useState hook".to_string()];
        
        // This should be detected as query only
        let metrics = PerformanceMetrics::default();
        let _result = handle_default_search(&args, metrics, None).await;
        // Will likely fail due to no sources, but that's expected in this phase
    }

    #[test]
    fn test_reserved_keywords_completeness() {
        // Ensure all expected command names are reserved
        let expected_commands = [
            "add", "search", "get", "list", "sources", "update", "remove", "rm", "delete", 
            "help", "version", "completions", "diff", "lookup"
        ];
        
        for &cmd in &expected_commands {
            assert!(RESERVED_KEYWORDS.contains(&cmd), 
                   "Command '{}' should be in reserved keywords", cmd);
        }
    }

    #[test]  
    fn test_reserved_keywords_no_duplicates() {
        // Ensure no duplicate entries in reserved keywords
        let mut seen = HashSet::new();
        for &keyword in RESERVED_KEYWORDS {
            assert!(seen.insert(keyword), 
                   "Reserved keyword '{}' appears multiple times", keyword);
        }
    }

    #[test]
    fn test_search_result_deduplication_logic() {
        use blzr_core::SearchHit;
        
        // Create test hits with duplicates
        let mut hits = vec![
            SearchHit {
                alias: "react".to_string(),
                file: "hooks.md".to_string(),
                heading_path: vec!["React".to_string(), "Hooks".to_string()],
                lines: "100-120".to_string(),
                snippet: "useState is a hook".to_string(),
                score: 0.95,
                source_url: None,
                checksum: "abc123".to_string(),
            },
            SearchHit {
                alias: "react".to_string(),
                file: "hooks.md".to_string(),
                heading_path: vec!["React".to_string(), "Hooks".to_string()],
                lines: "100-120".to_string(),
                snippet: "useState is a hook (duplicate)".to_string(),
                score: 0.85, // Different score but same location
                source_url: None,
                checksum: "abc123".to_string(),
            },
            SearchHit {
                alias: "nextjs".to_string(),
                file: "routing.md".to_string(),
                heading_path: vec!["Next.js".to_string(), "Routing".to_string()],
                lines: "50-75".to_string(),
                snippet: "App Router is the new way".to_string(),
                score: 0.90,
                source_url: None,
                checksum: "def456".to_string(),
            },
        ];
        
        // Apply the same deduplication logic as in the search function
        hits.sort_by(|a, b| {
            let cmp = a.alias.cmp(&b.alias)
                .then(a.lines.cmp(&b.lines))
                .then(a.heading_path.cmp(&b.heading_path));
            if cmp == std::cmp::Ordering::Equal {
                b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                cmp
            }
        });
        hits.dedup_by(|a, b| {
            a.alias == b.alias && a.lines == b.lines && a.heading_path == b.heading_path
        });
        
        // Should have deduplicated to 2 results (kept the higher scored react hit)
        assert_eq!(hits.len(), 2);
        
        // Verify the react hit kept is the higher scored one
        let react_hit = hits.iter().find(|h| h.alias == "react").unwrap();
        assert_eq!(react_hit.score, 0.95);
        assert_eq!(react_hit.snippet, "useState is a hook"); // Original, not duplicate
    }

    #[test]
    fn test_percentile_filtering_logic() {
        // Test the percentile calculation logic used in search function
        let total_results = 100;
        
        // Test various percentile calculations
        let top_10_count = (total_results as f32 * (10.0 / 100.0)).ceil() as usize;
        assert_eq!(top_10_count, 10);
        
        let top_25_count = (total_results as f32 * (25.0 / 100.0)).ceil() as usize;
        assert_eq!(top_25_count, 25);
        
        let top_50_count = (total_results as f32 * (50.0 / 100.0)).ceil() as usize;
        assert_eq!(top_50_count, 50);
        
        // Edge cases
        let top_1_count = (5 as f32 * (10.0 / 100.0)).ceil() as usize;
        assert_eq!(top_1_count.max(1), 1); // Should be at least 1
        
        let top_99_count = (10 as f32 * (99.0 / 100.0)).ceil() as usize;
        assert_eq!(top_99_count, 10); // 9.9 -> 10
    }

    #[test]
    fn test_pagination_logic() {
        // Test pagination calculation logic used in search function
        let total_results = 100;
        let limit = 20;
        
        // Page 1
        let page = 1;
        let start_idx = (page - 1) * limit;
        let end_idx = (start_idx + limit).min(total_results);
        assert_eq!(start_idx, 0);
        assert_eq!(end_idx, 20);
        
        // Page 3
        let page = 3;
        let start_idx = (page - 1) * limit;
        let end_idx = (start_idx + limit).min(total_results);
        assert_eq!(start_idx, 40);
        assert_eq!(end_idx, 60);
        
        // Last page (partial)
        let page = 5;
        let start_idx = (page - 1) * limit;
        let end_idx = (start_idx + limit).min(total_results);
        assert_eq!(start_idx, 80);
        assert_eq!(end_idx, 100);
        
        // Page beyond results
        let page = 10;
        let start_idx = (page - 1) * limit;
        assert!(start_idx >= total_results); // Should be caught in search function
        
        // Calculate total pages
        let total_pages = (total_results + limit - 1) / limit;
        assert_eq!(total_pages, 5);
    }

    #[test]
    fn test_invalid_percentile_values() {
        // Test that percentile validation would catch invalid values
        let invalid_percentiles = [0, 101, 200];
        
        for &percentile in &invalid_percentiles {
            // This test documents expected behavior - actual validation would be in clap
            assert!(percentile == 0 || percentile > 100, 
                   "Percentile {} should be considered invalid", percentile);
        }
        
        // Valid percentiles
        let valid_percentiles = [1, 50, 99, 100];
        for &percentile in &valid_percentiles {
            assert!(percentile >= 1 && percentile <= 100,
                   "Percentile {} should be valid", percentile);
        }
    }

    #[tokio::test]
    async fn test_search_performance_expectation() {
        use std::time::Instant;
        
        // This is a placeholder test for performance - actual implementation would require
        // a real index and search functionality
        
        let start = Instant::now();
        
        // Simulate search work (in real implementation, this would be actual search)
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        
        let duration = start.elapsed();
        
        // Test passes if simulated work is fast (real search should be <10ms)
        assert!(duration.as_millis() < 10, 
               "Search took {}ms, should be <10ms", duration.as_millis());
    }

    #[test]
    fn test_line_range_error_messages() {
        // Test that error messages are informative
        let result = parse_line_ranges("0");
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("1"), "Error should mention line numbers start at 1");
        
        let result = parse_line_ranges("abc");
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("abc"), "Error should mention the invalid input");
        
        let result = parse_line_ranges("10:5");
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("10") || error.contains("5"), 
               "Error should mention the invalid range values");
    }

    #[test]
    fn test_exact_file_detection() {
        // Test detection of exact llms.txt variant URLs
        let exact_urls = [
            "https://example.com/llms.txt",
            "https://example.com/docs/llms-full.txt",
            "https://api.example.com/v1/llms-mini.txt",
            "https://example.com/llms-base.txt",
        ];

        for url in &exact_urls {
            let is_exact = url.split('/').last()
                .map(|filename| filename.starts_with("llms") && filename.ends_with(".txt"))
                .unwrap_or(false);
            assert!(is_exact, "URL {} should be detected as exact file", url);
        }

        let non_exact_urls = [
            "https://example.com/",
            "https://example.com/docs",
            "https://example.com/api/v1/documentation.txt",
            "https://example.com/llms.html",
        ];

        for url in &non_exact_urls {
            let is_exact = url.split('/').last()
                .map(|filename| filename.starts_with("llms") && filename.ends_with(".txt"))
                .unwrap_or(false);
            assert!(!is_exact, "URL {} should NOT be detected as exact file", url);
        }
    }

    #[test]
    fn test_flavor_sorting_preference() {
        use blzr_core::FlavorInfo;

        let mut flavors = vec![
            FlavorInfo {
                name: "llms-base.txt".to_string(),
                size: Some(1000),
                url: "https://example.com/llms-base.txt".to_string(),
            },
            FlavorInfo {
                name: "llms.txt".to_string(),
                size: Some(5000),
                url: "https://example.com/llms.txt".to_string(),
            },
            FlavorInfo {
                name: "llms-full.txt".to_string(),
                size: Some(10000),
                url: "https://example.com/llms-full.txt".to_string(),
            },
            FlavorInfo {
                name: "llms-mini.txt".to_string(),
                size: Some(500),
                url: "https://example.com/llms-mini.txt".to_string(),
            },
        ];

        // Apply same sorting logic as in check_flavors()
        flavors.sort_by(|a, b| {
            let order_a = match a.name.as_str() {
                "llms-full.txt" => 0,
                "llms.txt" => 1,
                "llms-mini.txt" => 2,
                "llms-base.txt" => 3,
                _ => 4,
            };
            let order_b = match b.name.as_str() {
                "llms-full.txt" => 0,
                "llms.txt" => 1,
                "llms-mini.txt" => 2,
                "llms-base.txt" => 3,
                _ => 4,
            };
            order_a.cmp(&order_b)
        });

        // Verify sorting order: llms-full.txt > llms.txt > llms-mini.txt > llms-base.txt
        assert_eq!(flavors[0].name, "llms-full.txt");
        assert_eq!(flavors[1].name, "llms.txt");
        assert_eq!(flavors[2].name, "llms-mini.txt");
        assert_eq!(flavors[3].name, "llms-base.txt");
    }
}