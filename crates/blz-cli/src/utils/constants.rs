//! # Constants and Static Values
//!
//! This module defines constants used throughout the CLI to ensure consistency
//! and prevent naming conflicts between user-defined aliases and built-in commands.

/// Reserved keywords that cannot be used as aliases
///
/// This array contains all command names, subcommands, and special identifiers
/// that are reserved by the CLI. Attempting to use any of these as an alias
/// will result in a validation error.
///
/// # Categories
///
/// The reserved keywords are organized into several categories:
///
/// ## Commands
/// Primary CLI commands that users can invoke:
/// - `add`, `search`, `get`, `list`, `sources`, `update`, `remove`, `rm`, `delete`
/// - `help`, `version`, `completions`, `diff`, `lookup`, `plugin`
///
/// ## Meta Operations
/// Configuration and system-level operations (may be added in future versions):
/// - `config`, `settings`, `serve`, `server`, `start`, `stop`, `status`
///
/// ## Data Operations
/// Operations for data management (may be added in future versions):
/// - `sync`, `export`, `import`, `backup`, `restore`, `clean`, `purge`
///
/// ## Special Identifiers
/// Keywords with special meaning in various contexts:
/// - `all`, `none`, `default`, `local`, `global`, `cache`, `self`
///
/// # Usage
///
/// This constant is used by the [`validate_alias`](crate::utils::validate_alias)
/// function to ensure user-provided aliases don't conflict with CLI functionality:
///
/// ```rust,ignore
/// use crate::utils::{validate_alias, RESERVED_KEYWORDS};
///
/// // This will fail validation
/// assert!(validate_alias("add").is_err());
/// assert!(validate_alias("search").is_err());
///
/// // These are allowed
/// assert!(validate_alias("react").is_ok());
/// assert!(validate_alias("nextjs").is_ok());
/// assert!(RESERVED_KEYWORDS.contains(&"add"));
/// ```
///
/// # Design Rationale
///
/// Reserved keywords prevent:
/// - **Command Confusion**: Users can't accidentally shadow CLI commands
/// - **Future Compatibility**: Space is reserved for planned features
/// - **Consistent Interface**: Predictable behavior as the CLI evolves
/// - **Error Prevention**: Clear validation errors rather than mysterious failures
///
/// # Maintenance
///
/// When adding new CLI commands or features:
/// 1. Add the new keyword to this array
/// 2. Update the documentation categories above
/// 3. Ensure tests cover the new reserved word
/// 4. Consider backward compatibility impact
///
/// # Case Sensitivity
///
/// Validation is case-insensitive, so "ADD", "Add", and "add" are all rejected.
/// This prevents confusion and ensures consistent behavior across different
/// user input styles.
pub const RESERVED_KEYWORDS: &[&str] = &[
    // Commands
    "add",
    "search",
    "get",
    "list",
    "sources",
    "update",
    "remove",
    "rm",
    "delete",
    "help",
    "version",
    "completions",
    "diff",
    "lookup",
    "plugin",
    // Meta
    "config",
    "settings",
    "serve",
    "server",
    "start",
    "stop",
    "status",
    // Operations
    "sync",
    "export",
    "import",
    "backup",
    "restore",
    "clean",
    "purge",
    // Special
    "all",
    "none",
    "default",
    "local",
    "global",
    "cache",
    "self",
];
