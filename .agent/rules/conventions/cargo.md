# Cargo and Build System Conventions

## Workspace Configuration

### Root Workspace Setup

**Cargo.toml (Workspace Root)**
```toml
[workspace]
members = [
    "crates/*",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Outfitter Team <team@outfitter.dev>"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/outfitter-dev/cache"
homepage = "https://github.com/outfitter-dev/cache"
documentation = "https://docs.rs/cache"
readme = "README.md"
rust-version = "1.70.0"
keywords = ["search", "cache", "tantivy", "full-text", "index"]
categories = ["text-processing", "database-implementations", "caching"]

[workspace.dependencies]
# Core dependencies
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
clap = { version = "4.0", features = ["derive"] }

# Search engine
tantivy = "0.21"

# Async utilities
futures = "0.3"
tokio-util = { version = "0.7", features = ["full"] }

# Serialization and configuration
toml = "0.8"
serde_json = "1.0"

# Error handling and logging
log = "0.4"
env_logger = "0.10"

# Testing dependencies  
criterion = { version = "0.5", features = ["html_reports"] }
proptest = "1.0"
rstest = "0.18"
tempfile = "3.0"
tokio-test = "0.4"

# Development tools
once_cell = "1.19"

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
rust_2018_idioms = "deny"
unused_lifetimes = "deny"
unused_qualifications = "deny"
trivial_casts = "deny"
trivial_numeric_casts = "deny"

[workspace.lints.clippy]
all = "deny"
pedantic = "deny" 
cargo = "deny"
nursery = "deny"

# Allow some pedantic lints that are too restrictive
module_name_repetitions = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"
must_use_candidate = "allow"
too_many_lines = "allow"

# Cargo-specific lints
multiple_crate_versions = "deny"
wildcard_dependencies = "deny"

[profile.dev]
debug = true
opt-level = 0
overflow-checks = true
panic = "unwind"

[profile.test]
debug = true
opt-level = 1  # Slight optimization for faster tests

[profile.release]
debug = false
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
strip = true

[profile.bench]
debug = false
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"

# Development profile with some optimizations
[profile.dev-opt]
inherits = "dev"
opt-level = 1
debug = true

# Production profile with debug info for better error reporting
[profile.release-debug]
inherits = "release"
debug = true
strip = false
```

### Crate-Level Configuration

**cache-core/Cargo.toml**
```toml
[package]
name = "cache-core"
description = "Core search cache functionality using Tantivy"
workspace = true

[dependencies]
# Use workspace dependencies for consistency
tokio = { workspace = true }
serde = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
tantivy = { workspace = true }
futures = { workspace = true }
once_cell = { workspace = true }

# Crate-specific dependencies with explicit versioning
dashmap = "5.5"
lru = "0.12"

[dev-dependencies]
tokio-test = { workspace = true }
tempfile = { workspace = true }
rstest = { workspace = true }
proptest = { workspace = true }
criterion = { workspace = true }

[features]
default = ["search", "cache"]

# Feature flags for optional functionality
search = []
cache = []
metrics = ["prometheus"]
compression = ["zstd"]

# Optional dependencies enabled by features
[dependencies.prometheus]
version = "0.13"
optional = true

[dependencies.zstd]
version = "0.12"
optional = true

# Documentation settings
[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]

[[bench]]
name = "search_performance"
harness = false

[[example]]
name = "basic_search"
required-features = ["search"]
```

**cache-cli/Cargo.toml**
```toml
[package]
name = "cache-cli"
description = "Command-line interface for search cache"
workspace = true

[[bin]]
name = "cache"
path = "src/main.rs"

[dependencies]
cache-core = { path = "../cache-core" }

# CLI-specific dependencies
clap = { workspace = true }
anyhow = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
serde = { workspace = true }
toml = { workspace = true }

# Shell integration
rustyline = "13.0"
dirs = "5.0"

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
tempfile = { workspace = true }

[features]
default = ["shell-integration"]
shell-integration = ["rustyline"]
```

## Build Scripts and Code Generation

### Build Script Best Practices

**build.rs**
```rust
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Re-run if build script changes
    println!("cargo:rerun-if-changed=build.rs");
    
    // Re-run if version files change
    println!("cargo:rerun-if-changed=version.txt");
    
    // Generate version information
    generate_version_info();
    
    // Set build-time configuration
    set_build_config();
    
    // Platform-specific configuration
    configure_for_platform();
}

fn generate_version_info() {
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let git_hash = get_git_hash().unwrap_or_else(|| "unknown".to_string());
    let build_time = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    
    let version_info = format!(
        r#"
        pub const VERSION: &str = "{}";
        pub const GIT_HASH: &str = "{}";
        pub const BUILD_TIME: &str = "{}";
        "#,
        version, git_hash, build_time
    );
    
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("version.rs");
    fs::write(dest_path, version_info).unwrap();
}

fn get_git_hash() -> Option<String> {
    use std::process::Command;
    
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    
    if output.status.success() {
        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}

fn set_build_config() {
    // Enable specific features based on target
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-cfg=has_epoll");
    }
    
    // Set optimization flags for release builds
    if env::var("PROFILE").unwrap() == "release" {
        println!("cargo:rustc-env=OPTIMIZATION_LEVEL=3");
    }
}

fn configure_for_platform() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    
    match target_os.as_str() {
        "windows" => {
            println!("cargo:rustc-link-lib=shell32");
        }
        "macos" => {
            println!("cargo:rustc-link-lib=framework=CoreServices");
        }
        "linux" => {
            println!("cargo:rustc-link-lib=pthread");
        }
        _ => {}
    }
}
```

**Using Generated Code**
```rust
// src/version.rs
include!(concat!(env!("OUT_DIR"), "/version.rs"));

pub fn version_info() -> String {
    format!("{} ({})", VERSION, GIT_HASH)
}

pub fn full_version_info() -> String {
    format!("{} ({}) built on {}", VERSION, GIT_HASH, BUILD_TIME)
}
```

## Dependency Management

### Dependency Guidelines

**Dependency Selection Criteria**
```toml
# ✅ Prefer well-maintained crates with clear semver
serde = "1.0"              # Stable, widely used
tokio = "1.0"              # Async runtime standard
thiserror = "1.0"          # Error handling best practice

# ✅ Pin security-critical dependencies
ring = "=0.17.7"           # Cryptography - exact version
rustls = "=0.21.10"        # TLS - exact version

# ✅ Use minimal feature sets
tokio = { version = "1.0", features = ["rt", "net", "fs"] }
serde = { version = "1.0", features = ["derive"] }

# ❌ Avoid wildcards and overly broad ranges
regex = "*"                # Too broad
uuid = ">=0.8"            # Could break

# ❌ Avoid dev dependencies with many features
criterion = { version = "0.5", features = ["html_reports"] } # Specific features only
```

**Dependency Auditing**
```toml
# Cargo.deny.toml - Policy enforcement
[licenses]
allow = [
    "MIT",
    "Apache-2.0", 
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-DFS-2016",
]

deny = [
    "GPL-2.0",
    "GPL-3.0", 
    "AGPL-1.0",
    "AGPL-3.0",
    "LGPL-2.0",
    "LGPL-2.1",
    "LGPL-3.0",
]

[bans]
multiple-versions = "deny"
wildcards = "deny"

# Deny specific problematic crates
deny = [
    { name = "openssl", use-instead = "rustls" },
    { name = "chrono", use-instead = "time" }, # Only if you don't need timezone support
]

[advisories]
vulnerability = "deny"
unmaintained = "deny"
unsound = "deny"
notice = "warn"
ignore = [
    # Temporarily ignore specific advisories with justification
    # "RUSTSEC-2021-0073", # time 0.2 - waiting for upstream update
]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

### Version Management Strategy

**Semantic Versioning Guidelines**
```toml
# Version bumping rules for cache project:
# MAJOR: Breaking API changes, Rust edition changes
# MINOR: New features, non-breaking API additions
# PATCH: Bug fixes, performance improvements, documentation

[package]
version = "0.1.0"  # Pre-1.0: breaking changes allowed in minor versions
# version = "1.2.3" # Post-1.0: strict semantic versioning

# Development versions for unreleased changes
# version = "0.2.0-dev"      # Next minor release
# version = "0.1.1-beta.1"   # Beta release
# version = "0.1.1-rc.1"     # Release candidate
```

**Changelog Integration**
```toml
# Cargo.toml
[package.metadata.changelog]
# Configure changelog generation
header = """
# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
"""

[[package.metadata.changelog.sections]]
name = "Added"
description = "New features"

[[package.metadata.changelog.sections]]
name = "Changed" 
description = "Changes in existing functionality"

[[package.metadata.changelog.sections]]
name = "Deprecated"
description = "Soon-to-be removed features"

[[package.metadata.changelog.sections]]
name = "Removed"
description = "Removed features"

[[package.metadata.changelog.sections]]
name = "Fixed"
description = "Bug fixes"

[[package.metadata.changelog.sections]]
name = "Security"
description = "Security improvements"
```

## Testing Configuration

### Test Organization

**Comprehensive Test Setup**
```toml
# Cargo.toml test configuration
[dev-dependencies]
# Unit testing
tokio-test = { workspace = true }

# Property-based testing
proptest = { workspace = true }
quickcheck = "1.0"

# Parametric testing
rstest = { workspace = true }

# Integration testing utilities
tempfile = { workspace = true }
assert_matches = "1.5"

# Performance testing
criterion = { workspace = true }
iai = "0.1"  # Instruction-level benchmarking

# Mock and test doubles
mockall = "0.11"

# Testing configuration
[[test]]
name = "integration"
path = "tests/integration/main.rs"

[[test]]
name = "stress"
path = "tests/stress/main.rs"

[[bench]]
name = "search_performance"
harness = false

[[bench]]
name = "cache_performance" 
harness = false
required-features = ["cache"]
```

**Test Runner Configuration**
```toml
# .cargo/config.toml
[target.'cfg(test)']
rustflags = [
    # Enable additional lints for tests
    "-W", "unused-crate-dependencies",
    "-W", "missing-debug-implementations",
]

[build]
# Use specific target for consistent test results
target-dir = "target"

[profile.test]
# Optimize for test execution speed while maintaining debuggability
opt-level = 1
debug = 2
```

**Coverage Configuration**
```toml
# Cargo.toml
[package.metadata.coverage]
# Configure code coverage tools
exclude-patterns = [
    "tests/*",
    "benches/*", 
    "examples/*",
    "*/generated/*",
]

minimum-coverage = 80.0
```

## Build Optimization

### Compilation Performance

**Faster Debug Builds**
```toml
# .cargo/config.toml
[profile.dev.package."*"]
# Optimize dependencies in debug mode for faster execution
opt-level = 2
debug = false

[profile.dev.package.cache-core]
# Keep our code unoptimized for debugging
opt-level = 0
debug = true

[build]
# Use more parallel jobs
jobs = 8

# Use faster linker on available platforms
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=/DEBUG:FASTLINK"]

[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

**Conditional Compilation**
```rust
// Use cfg attributes for platform-specific or feature-specific code
#[cfg(feature = "metrics")]
pub mod metrics {
    use prometheus::{Counter, Histogram, Registry};
    
    pub struct SearchMetrics {
        search_counter: Counter,
        search_duration: Histogram,
    }
    
    impl SearchMetrics {
        pub fn new(registry: &Registry) -> Self {
            let search_counter = Counter::new("searches_total", "Total searches").unwrap();
            let search_duration = Histogram::new("search_duration_seconds", "Search duration").unwrap();
            
            registry.register(Box::new(search_counter.clone())).unwrap();
            registry.register(Box::new(search_duration.clone())).unwrap();
            
            Self { search_counter, search_duration }
        }
    }
}

#[cfg(not(feature = "metrics"))]
pub mod metrics {
    pub struct SearchMetrics;
    
    impl SearchMetrics {
        pub fn new(_registry: &()) -> Self {
            Self
        }
    }
}

// Platform-specific implementations
#[cfg(target_os = "linux")]
mod platform {
    pub fn optimize_for_platform() {
        // Linux-specific optimizations
        use std::os::unix::fs::OpenOptionsExt;
        // ...
    }
}

#[cfg(target_os = "macos")]
mod platform {
    pub fn optimize_for_platform() {
        // macOS-specific optimizations
    }
}

#[cfg(target_os = "windows")]
mod platform {
    pub fn optimize_for_platform() {
        // Windows-specific optimizations
    }
}
```

### Release Optimization

**Production Build Configuration**
```toml
[profile.release]
# Maximum optimization
opt-level = 3
lto = "fat"               # Full link-time optimization
codegen-units = 1         # Single codegen unit for best optimization
panic = "abort"           # Smaller binaries, no unwinding
strip = "symbols"         # Remove debug symbols

# Target-specific optimizations
[target.'cfg(target_arch = "x86_64")']
rustflags = [
    "-C", "target-cpu=native",        # Use all available CPU features
    "-C", "target-feature=+crt-static", # Static linking on Linux
]

[profile.release-small]
inherits = "release"
opt-level = "s"           # Optimize for size
lto = "thin"              # Lighter LTO for smaller size
```

**Binary Size Optimization**
```toml
# For minimal binary size
[profile.min-size]
inherits = "release"
opt-level = "z"           # Aggressive size optimization
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

## Documentation Generation

### Documentation Configuration

**Comprehensive Documentation Setup**
```toml
[package.metadata.docs.rs]
# Configure docs.rs build
all-features = true
rustdoc-args = [
    "--cfg", "docsrs",
    "--html-in-header", "docs/header.html",
    "--html-before-content", "docs/before-content.html",
]

# Include additional files in documentation
[package]
include = [
    "src/**/*",
    "README.md",
    "CHANGELOG.md",
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "docs/**/*",
]

# Documentation examples
[[example]]
name = "basic_usage"
doc-scrape-examples = true

[[example]]
name = "advanced_queries"
doc-scrape-examples = true
```

**Custom Documentation Tasks**
```toml
# Cargo.toml - Custom commands via cargo-make or just
[package.metadata.cargo-make.tasks.docs]
command = "cargo"
args = [
    "doc",
    "--workspace",
    "--all-features",
    "--no-deps",
    "--document-private-items"
]

[package.metadata.cargo-make.tasks.docs-open]
command = "cargo"
args = ["doc", "--open"]
dependencies = ["docs"]
```

## Continuous Integration

### CI-Friendly Configuration

**GitHub Actions Optimization**
```toml
# .cargo/config.toml
[registries.crates-io]
protocol = "sparse"      # Faster index updates

[net]
retry = 3               # Retry failed downloads

[profile.ci]
inherits = "test"
debug = 1               # Minimal debug info for CI
incremental = false     # Disable incremental compilation for CI
```

**Build Caching Strategy**
```yaml
# .github/workflows/ci.yml excerpt
- name: Cache cargo registry
  uses: actions/cache@v3
  with:
    path: |
      ~/.cargo/registry/index
      ~/.cargo/registry/cache
      ~/.cargo/git/db
      target
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      ${{ runner.os }}-cargo-
```

## Anti-Patterns

### Avoid These Configuration Patterns

**Problematic Configurations**
```toml
# ❌ Overly broad dependency versions
regex = "*"                    # Could break with any update
serde = ">= 1.0"              # Too permissive

# ❌ Unnecessary features enabled  
tokio = { version = "1.0", features = ["full"] }  # Pulls in everything

# ❌ Missing dev-dependency boundaries
criterion = "0.5"             # Should be in [dev-dependencies]

# ❌ Inconsistent optimization levels
[profile.release]
opt-level = 2                 # Not maximum optimization

# ❌ Missing workspace configuration
# Each crate duplicates common metadata

# ✅ Better alternatives
regex = "1.10"                # Specific minor version
serde = "1.0"                 # Major version with flexibility

tokio = { version = "1.0", features = ["rt", "net", "fs"] }  # Only needed features

[dev-dependencies]
criterion = "0.5"             # Proper section

[profile.release]
opt-level = 3                 # Maximum optimization
lto = "thin"                  # Link-time optimization

# Use workspace for shared configuration
```

Remember: Good Cargo configuration enables fast, reliable builds while maintaining security and code quality. Always prefer explicit configuration over defaults, and regularly audit your dependencies for security vulnerabilities.