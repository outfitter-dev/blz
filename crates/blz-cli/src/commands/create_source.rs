//! Registry create-source command implementation

use anyhow::{Context, Result, bail};
use blz_core::PerformanceMetrics;
use chrono::Utc;
use colored::Colorize;
use inquire::{Confirm, MultiSelect, Select, Text};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tokio::process::Command;

use crate::commands::{AddRequest, DescriptorInput, add_source};
use crate::utils::validation::{normalize_alias, validate_alias};

/// TOML source file structure
#[derive(Debug, Serialize, Deserialize)]
struct SourceToml {
    id: String,
    name: String,
    description: String,
    url: String,
    category: String,
    tags: Vec<String>,
    #[serde(rename = "registeredAt")]
    registered_at: String,
    #[serde(rename = "verifiedAt")]
    verified_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    aliases: Option<AliasesSection>,
    analysis: AnalysisSection,
}

/// Aliases section for npm/github packages
#[derive(Debug, Serialize, Deserialize)]
struct AliasesSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    npm: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    github: Option<Vec<String>>,
}

/// Analysis section metadata
#[derive(Debug, Serialize, Deserialize)]
struct AnalysisSection {
    #[serde(rename = "contentType")]
    content_type: String,
    #[serde(rename = "lineCount")]
    line_count: usize,
    #[serde(rename = "charCount")]
    char_count: usize,
    #[serde(rename = "headerCount")]
    header_count: usize,
    sections: usize,
    #[serde(rename = "fileSize")]
    file_size: String,
    #[serde(rename = "analyzedAt")]
    analyzed_at: String,
}

/// Execute the registry create-source command
///
/// Analyzes a source URL using dry-run, prompts for metadata,
/// creates a TOML file in registry/sources/, and rebuilds the registry.
/// Optionally adds the source to your local index if --add flag is set.
#[allow(clippy::too_many_arguments)]
pub async fn execute(
    name: &str,
    url: &str,
    description: Option<String>,
    category: Option<String>,
    tags: Vec<String>,
    npm_packages: Vec<String>,
    github_repos: Vec<String>,
    add_to_index: bool,
    yes: bool,
    quiet: bool,
    metrics: PerformanceMetrics,
) -> Result<()> {
    let normalized_alias = normalize_alias(name);
    let safe_name = sanitize_id(&normalized_alias)?;
    validate_alias(&safe_name)?;
    if !quiet && safe_name != name {
        println!("[info] Normalized alias to '{}'", safe_name.green());
    }

    // Step 1: Analyze source using dry-run
    if !quiet {
        println!("Analyzing source...");
    }

    let analysis = analyze_source(&safe_name, url, metrics.clone()).await?;

    // Step 2: Display analysis
    if !quiet {
        display_analysis(&analysis);
    }

    // Step 3: Validate content type
    if analysis.analysis.content_type == "index" && !yes {
        println!(
            "\n{} This source appears to be a navigation index only ({} lines).",
            "⚠".yellow(),
            analysis.analysis.line_count
        );
        let proceed = Confirm::new("Add it to the registry anyway?")
            .with_default(false)
            .prompt()?;

        if !proceed {
            println!("Skipping source.");
            return Ok(());
        }
    }

    // Step 4: Prompt for metadata (if not provided)
    let metadata = if yes {
        SourceMetadata {
            description: description.unwrap_or_else(|| "No description provided".to_string()),
            category: category.unwrap_or_else(|| "library".to_string()),
            tags: if tags.is_empty() {
                vec!["uncategorized".to_string()]
            } else {
                tags
            },
            npm_packages,
            github_repos,
        }
    } else {
        prompt_metadata(description, category, tags, npm_packages, github_repos)?
    };

    // Step 5: Create TOML file
    create_source_toml(&safe_name, name, &analysis, &metadata)?;

    if !quiet {
        println!(
            "{} Created registry source: {}",
            "✓".green(),
            format!("registry/sources/{safe_name}.toml").bright_black()
        );
    }

    // Step 6: Rebuild registry
    rebuild_registry(quiet).await?;

    if !quiet {
        println!("{} Added {} to registry", "✓".green(), safe_name.green());
    }

    // Step 7: Optionally add to local index
    if add_to_index {
        if !quiet {
            println!("\nAdding {} to local index...", safe_name.green());
        }

        let descriptor = DescriptorInput::from_cli_inputs(
            &[],
            Some(&safe_name),
            Some(&metadata.description),
            Some(&metadata.category),
            &metadata.tags,
        );

        let request = AddRequest::new(
            safe_name.clone(),
            analysis.final_url.clone(),
            descriptor,
            false,
            quiet,
            metrics,
            false, // no_language_filter
        );

        add_source(request).await?;

        if !quiet {
            println!("{} Source added to local index", "✓".green());
        }
    }

    Ok(())
}

/// Analysis result from dry-run
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceAnalysis {
    #[serde(alias = "alias")]
    name: String,
    url: String,
    final_url: String,
    analysis: ContentAnalysis,
    would_index: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentAnalysis {
    line_count: usize,
    char_count: usize,
    header_count: usize,
    sections: usize,
    file_size: String,
    content_type: String,
}

/// Metadata collected from user or command line
struct SourceMetadata {
    description: String,
    category: String,
    tags: Vec<String>,
    npm_packages: Vec<String>,
    github_repos: Vec<String>,
}

fn sanitize_id(input: &str) -> Result<String> {
    if input.is_empty() {
        bail!("Name cannot be empty.");
    }
    if input.starts_with('.') {
        bail!("Name cannot start with '.'.");
    }
    if input.contains('/') || input.contains('\\') {
        bail!("Name cannot contain path separators.");
    }
    if input.contains("..") {
        bail!("Name cannot contain '..' sequences.");
    }
    let valid = input
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if !valid {
        bail!("Invalid name. Use [a-z0-9_-] only.");
    }
    Ok(input.to_string())
}

/// Analyze source using blz add --dry-run
async fn analyze_source(
    name: &str,
    url: &str,
    _metrics: PerformanceMetrics,
) -> Result<SourceAnalysis> {
    // Run blz add --dry-run as subprocess to capture JSON output
    // Use installed blz binary instead of cargo run for production compatibility
    let output = Command::new("blz")
        .args(["add", name, url, "--dry-run", "--quiet"])
        .output()
        .await
        .context("Failed to run blz add --dry-run. Ensure 'blz' is installed in PATH.")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to analyze source:\n{stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let analysis: SourceAnalysis =
        serde_json::from_str(&stdout).context("Failed to parse dry-run analysis")?;

    Ok(analysis)
}

/// Display analysis results to user
fn display_analysis(analysis: &SourceAnalysis) {
    println!("\n{}", "Source Analysis:".bold());
    println!("  Alias:        {}", analysis.name.green());
    println!("  URL:          {}", analysis.final_url.bright_black());
    println!(
        "  Content Type: {}",
        format_content_type(&analysis.analysis.content_type)
    );
    println!("  File Size:    {}", analysis.analysis.file_size);
    println!("  Lines:        {}", analysis.analysis.line_count);
    println!("  Headers:      {}", analysis.analysis.header_count);
    println!("  Sections:     {}", analysis.analysis.sections);
    println!();
}

/// Format content type with color
fn format_content_type(content_type: &str) -> String {
    match content_type {
        "full" => content_type.green().to_string(),
        "index" => content_type.yellow().to_string(),
        "mixed" => content_type.cyan().to_string(),
        _ => content_type.to_string(),
    }
}

/// Parse comma-separated input into a vector of trimmed non-empty strings
fn parse_comma_separated(input: &str) -> Vec<String> {
    input
        .split(',')
        .filter_map(|s| {
            let trimmed = s.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect()
}

/// Prompt user for metadata
fn prompt_metadata(
    description: Option<String>,
    category: Option<String>,
    tags: Vec<String>,
    npm_packages: Vec<String>,
    github_repos: Vec<String>,
) -> Result<SourceMetadata> {
    // Description
    let description = if let Some(desc) = description {
        desc
    } else {
        Text::new("Description").prompt()?
    };

    // Category
    let available_categories = vec![
        "library",
        "framework",
        "language",
        "tool",
        "platform",
        "service",
        "runtime",
        "other",
    ];

    let category = if let Some(cat) = category {
        cat
    } else {
        let selection = Select::new("Category", available_categories.clone())
            .with_starting_cursor(0)
            .prompt()?;
        selection.to_string()
    };

    // Tags
    let tags = if tags.is_empty() {
        let suggested_tags = vec![
            "ai",
            "typescript",
            "javascript",
            "rust",
            "python",
            "react",
            "vue",
            "node",
            "frontend",
            "backend",
            "database",
            "testing",
            "documentation",
            "cli",
        ];

        let selections = MultiSelect::new(
            "Tags (space to select, enter when done)",
            suggested_tags.clone(),
        )
        .prompt()?;

        selections.iter().map(|&s| s.to_owned()).collect()
    } else {
        tags
    };

    // NPM packages
    let npm_packages = if npm_packages.is_empty() {
        let input = Text::new("NPM packages (comma-separated, or leave empty)")
            .with_default("")
            .prompt()?;

        parse_comma_separated(&input)
    } else {
        npm_packages
    };

    // GitHub repos
    let github_repos = if github_repos.is_empty() {
        let input = Text::new("GitHub repos (comma-separated, or leave empty)")
            .with_default("")
            .prompt()?;

        parse_comma_separated(&input)
    } else {
        github_repos
    };

    Ok(SourceMetadata {
        description,
        category,
        tags,
        npm_packages,
        github_repos,
    })
}

/// Create TOML file for the source
fn create_source_toml(
    id: &str,
    display_name: &str,
    analysis: &SourceAnalysis,
    metadata: &SourceMetadata,
) -> Result<()> {
    let registry_sources_dir = PathBuf::from("registry/sources");
    fs::create_dir_all(&registry_sources_dir)
        .context("Failed to create registry/sources directory")?;

    let toml_path = registry_sources_dir.join(format!("{id}.toml"));

    let now = Utc::now().to_rfc3339();

    // Build aliases section if NPM or GitHub repos provided
    let aliases = if !metadata.npm_packages.is_empty() || !metadata.github_repos.is_empty() {
        Some(AliasesSection {
            npm: if metadata.npm_packages.is_empty() {
                None
            } else {
                Some(metadata.npm_packages.clone())
            },
            github: if metadata.github_repos.is_empty() {
                None
            } else {
                Some(metadata.github_repos.clone())
            },
        })
    } else {
        None
    };

    // Build the TOML structure
    let source_toml = SourceToml {
        id: id.to_string(),
        name: display_name.to_string(),
        description: metadata.description.clone(),
        url: analysis.final_url.clone(),
        category: metadata.category.clone(),
        tags: metadata.tags.clone(),
        registered_at: now.clone(),
        verified_at: now.clone(),
        aliases,
        analysis: AnalysisSection {
            content_type: analysis.analysis.content_type.clone(),
            line_count: analysis.analysis.line_count,
            char_count: analysis.analysis.char_count,
            header_count: analysis.analysis.header_count,
            sections: analysis.analysis.sections,
            file_size: analysis.analysis.file_size.clone(),
            analyzed_at: now,
        },
    };

    // Serialize to TOML with proper escaping
    let toml_content =
        toml::to_string_pretty(&source_toml).context("Failed to serialize TOML structure")?;

    fs::write(&toml_path, toml_content).context("Failed to write TOML file")?;

    Ok(())
}

/// Rebuild the registry JSON from TOML sources
async fn rebuild_registry(quiet: bool) -> Result<()> {
    if !quiet {
        println!("Rebuilding registry...");
    }

    let output = Command::new("./registry/scripts/build.sh")
        .output()
        .await
        .context("Failed to run registry build script")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to rebuild registry:\n{stderr}");
    }

    Ok(())
}
