//! Context line argument groups for CLI commands.
//!
//! This module provides reusable context arguments for commands that
//! retrieve content and can include surrounding context lines.
//!
//! # Design
//!
//! Follows grep-style conventions:
//! - `-C NUM`: Symmetric context (same lines before and after)
//! - `-A NUM`: After context (lines after match)
//! - `-B NUM`: Before context (lines before match)
//! - `--context all`: Full section/block expansion
//!
//! # Examples
//!
//! ```bash
//! blz find "useEffect" -C 5           # 5 lines before and after
//! blz find "useEffect" -A 3 -B 2      # 2 lines before, 3 after
//! blz find "useEffect" --context all  # Full section
//! ```

use clap::Args;
use serde::{Deserialize, Serialize};

/// Context mode for result expansion.
///
/// Represents how much surrounding context to include when retrieving
/// content matches.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ContextMode {
    /// Symmetric context (same number of lines before and after).
    Symmetric(usize),
    /// Asymmetric context (different lines before and after).
    Asymmetric {
        /// Lines of context before the match.
        before: usize,
        /// Lines of context after the match.
        after: usize,
    },
    /// Full section/block expansion.
    All,
}

impl ContextMode {
    /// Get the before and after context line counts.
    ///
    /// Returns `(before, after)` tuple. For `All` mode, returns `None`.
    #[must_use]
    pub const fn lines(&self) -> Option<(usize, usize)> {
        match self {
            Self::Symmetric(n) => Some((*n, *n)),
            Self::Asymmetric { before, after } => Some((*before, *after)),
            Self::All => None,
        }
    }

    /// Check if this is the All (full section) mode.
    #[must_use]
    pub const fn is_all(&self) -> bool {
        matches!(self, Self::All)
    }

    /// Merge two context modes, taking the maximum value for each direction.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            // All takes precedence over everything
            (Self::All, _) | (_, Self::All) => Self::All,
            // Extract line counts and compute maximum for each direction
            (a, b) => {
                let (a_before, a_after) = a.lines().unwrap_or((0, 0));
                let (b_before, b_after) = b.lines().unwrap_or((0, 0));
                let before = a_before.max(b_before);
                let after = a_after.max(b_after);
                if before == after {
                    Self::Symmetric(before)
                } else {
                    Self::Asymmetric { before, after }
                }
            },
        }
    }
}

impl std::str::FromStr for ContextMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("all") {
            Ok(Self::All)
        } else {
            s.parse::<usize>()
                .map(Self::Symmetric)
                .map_err(|_| format!("Invalid context value: '{s}'. Expected a number or 'all'"))
        }
    }
}

impl std::fmt::Display for ContextMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Symmetric(n) => write!(f, "{n}"),
            Self::Asymmetric { before, after } => write!(f, "B{before}:A{after}"),
            Self::All => write!(f, "all"),
        }
    }
}

/// Shared context arguments for commands that retrieve content.
///
/// This group provides grep-style context line options for commands
/// that return content with surrounding lines (search, get, find, etc.).
///
/// # Usage
///
/// Flatten into command structs:
///
/// ```ignore
/// #[derive(Args)]
/// struct GetArgs {
///     #[command(flatten)]
///     context: ContextArgs,
///     // ... other args
/// }
/// ```
///
/// Then resolve to a `ContextMode`:
///
/// ```ignore
/// let mode = args.context.resolve();
/// ```
#[derive(Args, Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextArgs {
    /// Lines of context before and after each match (grep-style).
    ///
    /// Use a number for symmetric context, or "all" for full section.
    ///
    /// Examples:
    ///   -C 5          # 5 lines before and after
    ///   --context 10  # 10 lines before and after
    ///   --context all # Full section
    #[arg(
        short = 'C',
        long = "context",
        value_name = "LINES",
        display_order = 30
    )]
    pub context: Option<ContextMode>,

    /// Lines of context after each match.
    ///
    /// Can be combined with -B for asymmetric context.
    ///
    /// Examples:
    ///   -A 5              # 5 lines after match
    ///   --after-context 3 # 3 lines after match
    #[arg(
        short = 'A',
        long = "after-context",
        value_name = "LINES",
        display_order = 31
    )]
    pub after_context: Option<usize>,

    /// Lines of context before each match.
    ///
    /// Can be combined with -A for asymmetric context.
    ///
    /// Examples:
    ///   -B 5               # 5 lines before match
    ///   --before-context 3 # 3 lines before match
    #[arg(
        short = 'B',
        long = "before-context",
        value_name = "LINES",
        display_order = 32
    )]
    pub before_context: Option<usize>,
}

impl ContextArgs {
    /// Create context args with symmetric context.
    #[must_use]
    pub const fn symmetric(lines: usize) -> Self {
        Self {
            context: Some(ContextMode::Symmetric(lines)),
            after_context: None,
            before_context: None,
        }
    }

    /// Create context args for full section expansion.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            context: Some(ContextMode::All),
            after_context: None,
            before_context: None,
        }
    }

    /// Resolve the context arguments into a single `ContextMode`.
    ///
    /// Implements grep-style merging logic:
    /// - `-C` provides symmetric context
    /// - `-A` and `-B` can be combined for asymmetric context
    /// - If multiple flags are provided, takes maximum value for each direction
    #[must_use]
    pub fn resolve(&self) -> Option<ContextMode> {
        // Start with the primary context flag
        let mut result = self.context.clone();

        // Merge in -A and -B flags if present
        if let Some(after) = self.after_context {
            let new_mode = self
                .before_context
                .map_or(ContextMode::Asymmetric { before: 0, after }, |before| {
                    ContextMode::Asymmetric { before, after }
                });

            result = Some(match result.take() {
                Some(existing) => existing.merge(new_mode),
                None => new_mode,
            });
        } else if let Some(before) = self.before_context {
            // Only -B specified, create asymmetric mode with 0 after
            let new_mode = ContextMode::Asymmetric { before, after: 0 };
            result = Some(match result.take() {
                Some(existing) => existing.merge(new_mode),
                None => new_mode,
            });
        }

        result
    }

    /// Check if any context is requested.
    #[must_use]
    pub const fn has_context(&self) -> bool {
        self.context.is_some() || self.after_context.is_some() || self.before_context.is_some()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    mod context_mode {
        use super::*;

        #[test]
        fn test_symmetric_lines() {
            let mode = ContextMode::Symmetric(5);
            assert_eq!(mode.lines(), Some((5, 5)));
        }

        #[test]
        fn test_asymmetric_lines() {
            let mode = ContextMode::Asymmetric {
                before: 3,
                after: 7,
            };
            assert_eq!(mode.lines(), Some((3, 7)));
        }

        #[test]
        fn test_all_lines() {
            let mode = ContextMode::All;
            assert_eq!(mode.lines(), None);
        }

        #[test]
        fn test_is_all() {
            assert!(ContextMode::All.is_all());
            assert!(!ContextMode::Symmetric(5).is_all());
            assert!(
                !ContextMode::Asymmetric {
                    before: 1,
                    after: 1
                }
                .is_all()
            );
        }

        #[test]
        fn test_merge_all_precedence() {
            let sym = ContextMode::Symmetric(5);
            let all = ContextMode::All;

            assert!(matches!(sym.clone().merge(all.clone()), ContextMode::All));
            assert!(matches!(all.merge(sym), ContextMode::All));
        }

        #[test]
        fn test_merge_symmetric() {
            let a = ContextMode::Symmetric(3);
            let b = ContextMode::Symmetric(5);

            assert_eq!(a.merge(b), ContextMode::Symmetric(5));
        }

        #[test]
        fn test_merge_asymmetric() {
            let a = ContextMode::Asymmetric {
                before: 3,
                after: 2,
            };
            let b = ContextMode::Asymmetric {
                before: 1,
                after: 5,
            };

            assert_eq!(
                a.merge(b),
                ContextMode::Asymmetric {
                    before: 3,
                    after: 5
                }
            );
        }

        #[test]
        fn test_parse_number() {
            assert_eq!(
                "5".parse::<ContextMode>().unwrap(),
                ContextMode::Symmetric(5)
            );
            assert_eq!(
                "0".parse::<ContextMode>().unwrap(),
                ContextMode::Symmetric(0)
            );
        }

        #[test]
        fn test_parse_all() {
            assert_eq!("all".parse::<ContextMode>().unwrap(), ContextMode::All);
            assert_eq!("ALL".parse::<ContextMode>().unwrap(), ContextMode::All);
            assert_eq!("All".parse::<ContextMode>().unwrap(), ContextMode::All);
        }

        #[test]
        fn test_parse_invalid() {
            assert!("abc".parse::<ContextMode>().is_err());
            assert!("-1".parse::<ContextMode>().is_err());
        }

        #[test]
        fn test_display() {
            assert_eq!(ContextMode::Symmetric(5).to_string(), "5");
            assert_eq!(
                ContextMode::Asymmetric {
                    before: 2,
                    after: 3
                }
                .to_string(),
                "B2:A3"
            );
            assert_eq!(ContextMode::All.to_string(), "all");
        }
    }

    mod context_args {
        use super::*;

        #[test]
        fn test_default() {
            let args = ContextArgs::default();
            assert_eq!(args.context, None);
            assert_eq!(args.after_context, None);
            assert_eq!(args.before_context, None);
            assert!(!args.has_context());
        }

        #[test]
        fn test_symmetric() {
            let args = ContextArgs::symmetric(5);
            assert_eq!(args.resolve(), Some(ContextMode::Symmetric(5)));
            assert!(args.has_context());
        }

        #[test]
        fn test_all() {
            let args = ContextArgs::all();
            assert_eq!(args.resolve(), Some(ContextMode::All));
            assert!(args.has_context());
        }

        #[test]
        fn test_resolve_only_after() {
            let args = ContextArgs {
                context: None,
                after_context: Some(5),
                before_context: None,
            };
            assert_eq!(
                args.resolve(),
                Some(ContextMode::Asymmetric {
                    before: 0,
                    after: 5
                })
            );
        }

        #[test]
        fn test_resolve_only_before() {
            let args = ContextArgs {
                context: None,
                after_context: None,
                before_context: Some(5),
            };
            assert_eq!(
                args.resolve(),
                Some(ContextMode::Asymmetric {
                    before: 5,
                    after: 0
                })
            );
        }

        #[test]
        fn test_resolve_both_before_after() {
            let args = ContextArgs {
                context: None,
                after_context: Some(3),
                before_context: Some(5),
            };
            assert_eq!(
                args.resolve(),
                Some(ContextMode::Asymmetric {
                    before: 5,
                    after: 3
                })
            );
        }

        #[test]
        fn test_resolve_merge_with_context() {
            let args = ContextArgs {
                context: Some(ContextMode::Symmetric(2)),
                after_context: Some(5),
                before_context: None,
            };
            // Should merge: symmetric(2) + asymmetric(0, 5) = asymmetric(2, 5)
            assert_eq!(
                args.resolve(),
                Some(ContextMode::Asymmetric {
                    before: 2,
                    after: 5
                })
            );
        }
    }
}
