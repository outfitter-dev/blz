# blz-cli Agent Guide

This crate provides the command-line interface for the blz search tool. It focuses on user experience, argument parsing, output formatting, and shell integration.

## Architecture Overview

- **User-Facing**: All interactions happen through this CLI
- **Error Messages**: Must be helpful and actionable for end users  
- **Output Formats**: Support JSON, plain text, and colored terminal output
- **Shell Integration**: Completions, proper exit codes, pipe-friendly output

## Key Modules

- **`cli.rs`**: Argument parsing with clap, command structure
- **`commands/`**: Individual command implementations (search, add, remove, etc.)
- **`output/`**: Formatting and display logic for different output modes
- **`utils/`**: Common utilities for validation, parsing, and formatting

## User Experience Patterns

### Error Messages

```rust
// ✅ GOOD: Helpful, actionable error messages
impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CliError::SourceNotFound { alias } => {
                write!(f, "Source '{}' not found. Run `blz list` to see available sources.", alias)
            }
            CliError::InvalidQuery { query, reason } => {
                write!(f, "Invalid search query '{}':\n  {}\n  Try a simpler query or check the syntax.", query, reason)
            }
        }
    }
}

// ❌ BAD: Unhelpful error messages
impl fmt::Display for BadCliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error: {}", self.inner) // Not actionable!
    }
}
```

### Progress Indication

```rust
use indicatif::{ProgressBar, ProgressStyle};

// ✅ GOOD: Show progress for long operations
pub async fn fetch_with_progress(url: &str) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")?);
    pb.set_message(format!("Fetching {}...", url));

    let result = fetch_source(url).await;
    
    match result {
        Ok(_) => pb.finish_with_message("✅ Fetch complete"),
        Err(_) => pb.finish_with_message("❌ Fetch failed"),
    }
    
    result
}
```

### Output Formatting

```rust
// Support multiple output formats
#[derive(Debug, Clone)]
pub enum OutputFormat {
    Text,
    Json,
    Compact,
}

impl SearchResults {
    pub fn format(&self, format: OutputFormat, use_colors: bool) -> String {
        match format {
            OutputFormat::Json => serde_json::to_string_pretty(self).unwrap(),
            OutputFormat::Text => self.format_text(use_colors),
            OutputFormat::Compact => self.format_compact(),
        }
    }
    
    fn format_text(&self, use_colors: bool) -> String {
        let mut output = String::new();
        
        if use_colors && atty::is(atty::Stream::Stdout) {
            // Use colored output for terminals
            for hit in &self.hits {
                output.push_str(&format!(
                    "{}: {}\n",
                    hit.source.bright_blue(),
                    hit.title.bright_white()
                ));
            }
        } else {
            // Plain text for pipes/files
            for hit in &self.hits {
                output.push_str(&format!("{}: {}\n", hit.source, hit.title));
            }
        }
        
        output
    }
}
```

## Command Implementation Patterns

### Command Structure

```rust
// ✅ GOOD: Well-structured command with validation
#[derive(Debug, clap::Args)]
pub struct SearchArgs {
    /// Search query
    query: String,
    
    /// Maximum number of results
    #[arg(short, long, default_value = "10")]
    limit: u16,
    
    /// Output format
    #[arg(short, long, default_value = "text")]
    format: OutputFormat,
    
    /// Specific source to search
    #[arg(long)]
    source: Option<String>,
}

impl SearchArgs {
    pub fn validate(&self) -> Result<(), CliError> {
        if self.query.trim().is_empty() {
            return Err(CliError::EmptyQuery);
        }
        
        if self.limit == 0 || self.limit > 1000 {
            return Err(CliError::InvalidLimit { 
                limit: self.limit,
                max: 1000,
            });
        }
        
        Ok(())
    }
}

pub async fn search_command(args: SearchArgs) -> Result<()> {
    args.validate()?;
    
    let results = if let Some(source) = &args.source {
        search_single_source(&args.query, source, args.limit).await?
    } else {
        search_all_sources(&args.query, args.limit).await?
    };
    
    println!("{}", results.format(args.format, true));
    Ok(())
}
```

### Input Validation

```rust
// ✅ GOOD: Comprehensive input validation
pub fn validate_alias(alias: &str) -> Result<(), CliError> {
    if alias.is_empty() {
        return Err(CliError::EmptyAlias);
    }
    
    if alias.len() > 50 {
        return Err(CliError::AliasTooLong { 
            alias: alias.to_string(),
            max_length: 50,
        });
    }
    
    // Check for invalid characters
    if !alias.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(CliError::InvalidAlias {
            alias: alias.to_string(),
            reason: "Only letters, numbers, hyphens, and underscores allowed".to_string(),
        });
    }
    
    Ok(())
}

pub fn validate_url(url: &str) -> Result<(), CliError> {
    let parsed = url::Url::parse(url)
        .map_err(|e| CliError::InvalidUrl { 
            url: url.to_string(),
            reason: e.to_string(),
        })?;
    
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(CliError::UnsupportedScheme {
            scheme: parsed.scheme().to_string(),
        });
    }
    
    Ok(())
}
```

## Shell Integration

### Completions

```rust
// build.rs - Generate shell completions at build time
use clap::CommandFactory;
use clap_complete::{generate_to, Shell};
use std::env;
use std::io::Error;

fn main() -> Result<(), Error> {
    let outdir = match env::var_os("OUT_DIR") {
        None => return Ok(()),
        Some(outdir) => outdir,
    };

    let mut cmd = crate::cli::Cli::command();
    
    // Generate completions for multiple shells
    let shells = [Shell::Bash, Shell::Zsh, Shell::Fish, Shell::PowerShell];
    
    for shell in shells {
        generate_to(shell, &mut cmd, "blz", &outdir)?;
    }

    Ok(())
}
```

### Exit Codes

```rust
// ✅ GOOD: Meaningful exit codes
pub fn exit_with_code(result: Result<(), CliError>) -> ! {
    match result {
        Ok(()) => std::process::exit(0),
        Err(error) => {
            eprintln!("{}", error);
            
            let exit_code = match error {
                CliError::SourceNotFound { .. } => 2,
                CliError::InvalidQuery { .. } => 3,
                CliError::NetworkError { .. } => 4,
                CliError::IoError { .. } => 5,
                _ => 1, // Generic error
            };
            
            std::process::exit(exit_code);
        }
    }
}
```

## Testing Patterns

### Command Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use assert_cmd::Command;
    use predicates::prelude::*;
    use tempfile::TempDir;

    #[test]
    fn test_search_command() {
        let mut cmd = Command::cargo_bin("blz").unwrap();
        
        cmd.arg("search")
            .arg("rust")
            .assert()
            .success()
            .stdout(predicate::str::contains("Results"));
    }
    
    #[test]
    fn test_invalid_arguments() {
        let mut cmd = Command::cargo_bin("blz").unwrap();
        
        cmd.arg("search")
            .arg("")  // Empty query
            .assert()
            .failure()
            .code(3)  // Invalid query exit code
            .stderr(predicate::str::contains("empty"));
    }
    
    #[test]
    fn test_json_output() {
        let mut cmd = Command::cargo_bin("blz").unwrap();
        
        cmd.arg("search")
            .arg("--format")
            .arg("json")
            .arg("test")
            .assert()
            .success()
            .stdout(predicate::str::is_json());
    }
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_add_and_search_workflow() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("BLZ_CONFIG_DIR", temp_dir.path());
    
    // Add a source
    let add_result = add_command(AddArgs {
        alias: "test".to_string(),
        url: "https://example.com/llms.txt".to_string(),
    }).await;
    assert!(add_result.is_ok());
    
    // Search the source
    let search_result = search_command(SearchArgs {
        query: "example".to_string(),
        limit: 10,
        format: OutputFormat::Text,
        source: Some("test".to_string()),
    }).await;
    assert!(search_result.is_ok());
}
```

## Common Agent Tasks

### Adding New Commands

1. **Add command struct** in `commands/mod.rs`
2. **Implement command logic** in `commands/new_command.rs`
3. **Add to CLI parser** in `cli.rs`
4. **Write tests** in the command module
5. **Update help text and documentation**

### Improving Error Messages

1. **Identify user pain points** from error reports
2. **Add context and suggestions** to error types
3. **Test error scenarios** in unit tests
4. **Verify help text is actionable**

### Output Format Changes

1. **Update output types** in `output/mod.rs`
2. **Add formatting methods**
3. **Test with different terminals**
4. **Ensure JSON compatibility**

## Common Gotchas

### Terminal Detection

```rust
// ✅ GOOD: Detect terminal capabilities
use atty::{is, Stream};

pub fn should_use_colors() -> bool {
    // Respect NO_COLOR environment variable
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    
    // Check if stdout is a terminal
    is(Stream::Stdout) && supports_color::on(supports_color::Stream::Stdout).is_some()
}
```

### Argument Parsing Edge Cases

```rust
// ✅ GOOD: Handle edge cases in parsing
#[derive(Debug, clap::Args)]
pub struct SearchArgs {
    /// Search query (handles quotes and special characters)
    #[arg(value_parser = parse_query)]
    query: String,
}

fn parse_query(query: &str) -> Result<String, String> {
    if query.trim().is_empty() {
        return Err("Query cannot be empty".to_string());
    }
    
    // Handle shell quoting
    let trimmed = query.trim_matches(|c| c == '"' || c == '\'');
    
    if trimmed.len() > 1000 {
        return Err("Query too long (max 1000 characters)".to_string());
    }
    
    Ok(trimmed.to_string())
}
```

### Config File Locations

```rust
// ✅ GOOD: Cross-platform config directories
use directories::ProjectDirs;

pub fn get_config_dir() -> Result<PathBuf, CliError> {
    // Check environment variable first
    if let Ok(custom_dir) = std::env::var("BLZ_CONFIG_DIR") {
        return Ok(PathBuf::from(custom_dir));
    }
    
    // Use platform-appropriate config directory
    let proj_dirs = ProjectDirs::from("com", "outfitter", "blz")
        .ok_or(CliError::NoConfigDir)?;
    
    Ok(proj_dirs.config_dir().to_path_buf())
}
```

## Development Workflow

### Testing Commands
```bash
# Test all CLI functionality
cargo test -p blz-cli

# Test with different shells
cargo build --release
./target/release/blz --help

# Test completions
./target/release/blz completions bash > /tmp/blz-completions
source /tmp/blz-completions
```

### Manual Testing
```bash
# Test different output formats
blz search "rust" --format json
blz search "rust" --format compact

# Test error cases
blz search ""  # Empty query
blz search --source nonexistent "test"  # Missing source
```

### Building
```bash
# Development build
cargo build -p blz-cli

# Release build with optimizations
cargo build -p blz-cli --release

# Install locally for testing
cargo install --path crates/blz-cli --force
```

Remember: This crate is the face of the project. Every interaction should be polite, helpful, and efficient.