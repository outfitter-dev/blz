//! Formatting utilities

use colored::Colorize;

/// Color cycling functions for aliases
pub const ALIAS_COLORS: &[fn(&str) -> colored::ColoredString] = &[
    |s| s.green(),
    |s| s.blue(),
    |s| s.truecolor(0, 150, 136), // teal
    |s| s.magenta(),
];

/// Get a color for an alias based on its index
pub fn get_alias_color(alias: &str, index: usize) -> colored::ColoredString {
    let color_fn = ALIAS_COLORS[index % ALIAS_COLORS.len()];
    color_fn(alias)
}
