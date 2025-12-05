# blz-cli Development Guide for Agents

## Context
This is the CLI binary crate providing user-facing commands.
**User experience is critical** - focus on helpful error messages, intuitive commands, and shell integration.

## Key Patterns Used Here

- @./.agents/rules/conventions/rust/async-patterns.md - Async main and tokio patterns
- @./.agents/rules/conventions/rust/compiler-loop.md - For debugging CLI build issues

### CLI Error Handling
```rust
// Use anyhow for CLI - users need helpful, actionable messages
use anyhow::{bail, Context, Result};

pub fn run_search(query: &str, alias: Option<&str>) -> Result<()> {
    let results = core_search(query, alias)
        .context("Search failed")?
        .with_context(|| {
            if alias.is_some() {
                "Check if the source exists with 'blz list'"
            } else {
                "Check if you have any sources added with 'blz list'"
            }
        })?;
    
    if results.hits.is_empty() {
        bail!(
            "No results found for query: '{}'\n\nTry:\n  • blz search '{}' --limit 50\n  • blz list # to see available sources", 
            query, query
        );
    }
    
    print_results(results)?;
    Ok(())
}

// Provide context for common user errors
pub fn load_config() -> Result<Config> {
    Config::load().context(
        "Failed to load configuration.\n\n\
         This might be your first time using blz. Try:\n  \
         • blz add <alias> <url> to add your first source"
    )
}
```

### Async Main Pattern
```rust
use clap::{Args, Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "blz")]
#[command(about = "Fast local search for llms.txt documentation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Enable debug logging
    #[arg(long, short = 'v')]
    verbose: bool,
    
    /// Output format
    #[arg(long, value_enum, default_value = "text")]
    output: OutputFormat,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for content across sources
    Search(SearchArgs),
    /// Add a new source
    Add(AddArgs),
    /// List configured sources
    List(ListArgs),
    // ... other commands
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging based on verbosity
    init_logging(cli.verbose)?;
    
    // Execute command
    match cli.command {
        Commands::Search(args) => run_search(args, cli.output).await,
        Commands::Add(args) => run_add(args).await,
        Commands::List(args) => run_list(args, cli.output).await,
        // ... other commands
    }
}
```

### User-Friendly Output Formatting
```rust
use crate::output::{Formatter, OutputFormat};

pub async fn run_search(args: SearchArgs, format: OutputFormat) -> Result<()> {
    let results = search_core(&args.query, args.alias.as_deref()).await?;
    
    let formatter = Formatter::new(format);
    formatter.print_results(&results)?;
    
    // Show helpful tips for no results
    if results.hits.is_empty() {
        formatter.print_tips(&[
            "Try a broader search term",
            "Check available sources with 'blz list'",
            "Add more sources with 'blz add <alias> <url>'",
        ])?;
    }
    
    Ok(())
}

// JSON output for programmatic use
pub fn format_json(results: &SearchResults) -> Result<String> {
    let output = serde_json::json!({
        "query": results.query,
        "total_hits": results.total_count,
        "execution_time_ms": results.execution_time.as_millis(),
        "hits": results.hits.iter().map(|hit| {
            serde_json::json!({
                "alias": hit.alias,
                "title": hit.title,
                "content": hit.content,
                "score": hit.score,
                "line_start": hit.line_range.start,
                "line_end": hit.line_range.end,
            })
        }).collect::<Vec<_>>()
    });
    
    serde_json::to_string_pretty(&output)
        .context("Failed to serialize results to JSON")
}
```

### Progress Indication for Long Operations
```rust
use indicatif::{ProgressBar, ProgressStyle};

pub async fn run_update_all() -> Result<()> {
    let sources = list_sources().await?;
    
    let pb = ProgressBar::new(sources.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")?
        .progress_chars("#>-"));
    
    for source in sources {
        pb.set_message(format!("Updating {}", source.alias));
        
        update_source(&source).await
            .with_context(|| format!("Failed to update source '{}'", source.alias))?;
        
        pb.inc(1);
    }
    
    pb.finish_with_message("All sources updated successfully");
    Ok(())
}
```

### Shell Integration
```rust
// build.rs - Generate shell completions at build time
use clap::CommandFactory;
use clap_complete::{generate_to, shells::*};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let outdir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    
    let mut app = Cli::command();
    generate_to(Bash, &mut app, "blz", &outdir)?;
    generate_to(Fish, &mut app, "blz", &outdir)?;
    generate_to(Zsh, &mut app, "blz", &outdir)?;
    
    Ok(())
}

// Expose completion generation as a command
#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    shell: Shell,
}

pub fn run_completions(args: CompletionsArgs) -> Result<()> {
    let mut app = Cli::command();
    match args.shell {
        Shell::Bash => generate(Bash, &mut app, "blz", &mut std::io::stdout()),
        Shell::Fish => generate(Fish, &mut app, "blz", &mut std::io::stdout()),
        Shell::Zsh => generate(Zsh, &mut app, "blz", &mut std::io::stdout()),
        Shell::PowerShell => generate(PowerShell, &mut app, "blz", &mut std::io::stdout()),
    }
    Ok(())
}
```

### Configuration Management
```rust
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CliConfig {
    pub default_limit: usize,
    pub color_mode: ColorMode,
    pub editor: Option<String>,
    pub aliases: HashMap<String, String>,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            default_limit: 20,
            color_mode: ColorMode::Auto,
            editor: std::env::var("EDITOR").ok(),
            aliases: HashMap::new(),
        }
    }
}

impl CliConfig {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            // Create default config
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }
        
        let content = std::fs::read_to_string(&config_path)
            .context("Failed to read config file")?;
            
        toml::from_str(&content)
            .context("Failed to parse config file")
    }
    
    fn config_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("dev", "outfitter", "blz")
            .context("Unable to determine config directory")?;
            
        let config_dir = proj_dirs.config_dir();
        std::fs::create_dir_all(config_dir)
            .context("Failed to create config directory")?;
            
        Ok(config_dir.join("config.toml"))
    }
}
```

### Input Validation and Sanitization
```rust
use url::Url;

pub fn validate_source_url(url_str: &str) -> Result<Url> {
    let url = Url::parse(url_str)
        .context("Invalid URL format")?;
    
    match url.scheme() {
        "http" | "https" => Ok(url),
        scheme => bail!("Unsupported URL scheme '{}'. Only http and https are supported.", scheme),
    }
}

pub fn validate_alias(alias: &str) -> Result<()> {
    if alias.is_empty() {
        bail!("Alias cannot be empty");
    }
    
    if alias.len() > 64 {
        bail!("Alias cannot be longer than 64 characters");
    }
    
    if !alias.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        bail!("Alias can only contain letters, numbers, hyphens, and underscores");
    }
    
    if alias.starts_with('-') {
        bail!("Alias cannot start with a hyphen");
    }
    
    Ok(())
}
```

## User Experience Principles

### 1. **Helpful Error Messages**
```rust
// ❌ Bad: Technical error without context
return Err(anyhow::anyhow!("HTTP 404"));

// ✅ Good: Actionable error with suggestions
return Err(anyhow::anyhow!(
    "Source not found (HTTP 404): {}\n\n\
     The URL might be incorrect or the server might be down.\n\
     Try:\n  • Check the URL in your browser\n  \
     • Verify the llms.txt file exists at that location",
    url
));
```

### 2. **Progressive Disclosure**
```rust
// Show basic info by default, more with --verbose
pub fn print_source_info(source: &Source, verbose: bool) {
    println!("{}: {}", source.alias, source.url);
    
    if verbose {
        println!("  Last updated: {}", source.last_updated);
        println!("  Document count: {}", source.document_count);
        println!("  Index size: {}", format_size(source.index_size));
    }
}
```

### 3. **Consistent Command Structure**
```bash
# All commands follow consistent patterns
blz <action> [target] [options]

blz search "async rust"           # Search across all sources
blz search "async rust" --source bun  # Search specific source
blz add react https://react.dev/llms.txt  # Add new source
blz remove react                 # Remove source
blz list                          # List all sources
blz refresh --all                 # Refresh all sources (deprecated alias: blz update --all)
blz refresh react                 # Refresh specific source (deprecated alias: blz update react)
```

## Testing CLI Applications
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use assert_cmd::Command;
    use predicates::prelude::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_search_no_sources() {
        let temp_dir = TempDir::new().unwrap();
        
        let mut cmd = Command::cargo_bin("blz").unwrap();
        cmd.env("BLZ_CONFIG_DIR", temp_dir.path())
           .args(&["search", "rust"])
           .assert()
           .failure()
           .stderr(predicate::str::contains("No sources configured"));
    }
    
    #[test]
    fn test_add_invalid_url() {
        let mut cmd = Command::cargo_bin("blz").unwrap();
        cmd.args(&["add", "test", "not-a-url"])
           .assert()
           .failure()
           .stderr(predicate::str::contains("Invalid URL format"));
    }
    
    #[tokio::test]
    async fn test_json_output() {
        let results = mock_search_results();
        let json = format_json(&results).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed["total_hits"], results.total_count);
        assert!(parsed["hits"].is_array());
    }
}
```

## Exit Codes
```rust
use std::process;

pub enum ExitCode {
    Success = 0,
    UserError = 1,      // Bad arguments, missing config, etc.
    SystemError = 2,    // Network issues, file system errors, etc.
    Interrupted = 130,  // Ctrl+C
}

pub fn exit_with_code(code: ExitCode) -> ! {
    process::exit(code as i32);
}

// Usage in main
#[tokio::main]
async fn main() {
    let result = run().await;
    
    let exit_code = match result {
        Ok(()) => ExitCode::Success,
        Err(e) => {
            eprintln!("Error: {}", e);
            
            // Print error chain for debugging
            let mut source = e.source();
            while let Some(err) = source {
                eprintln!("  Caused by: {}", err);
                source = err.source();
            }
            
            // Determine appropriate exit code based on error type
            if e.to_string().contains("No such file") || 
               e.to_string().contains("Invalid") {
                ExitCode::UserError
            } else {
                ExitCode::SystemError
            }
        }
    };
    
    exit_with_code(exit_code);
}
```

## Performance Considerations
- **Lazy loading**: Only load what's needed for the current command
- **Streaming**: Don't buffer large results in memory  
- **Caching**: Cache expensive computations between commands
- **Async I/O**: Use tokio for all network and file operations

## Shell Integration Best Practices
1. **Respect shell conventions** - return appropriate exit codes
2. **Support common flags** - `--help`, `--version`, `--verbose`, `--quiet`
3. **Pipe-friendly output** - detect TTY vs pipe and adjust formatting
4. **Color handling** - auto-detect color support, provide `--no-color` flag
5. **Tab completion** - generate completions for all shells
