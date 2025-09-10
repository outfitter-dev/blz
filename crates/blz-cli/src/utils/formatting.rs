//! Formatting utilities

use colored::Colorize;

/// ANSI-only color cycling functions for aliases (exclude red)
/// Order: blue → cyan → green → yellow → magenta
pub const ALIAS_COLORS: &[fn(&str) -> colored::ColoredString] = &[
    |s| s.blue(),
    |s| s.cyan(),
    |s| s.green(),
    |s| s.yellow(),
    |s| s.magenta(),
];

/// Get a color for an alias based on its index
pub fn get_alias_color(alias: &str, index: usize) -> colored::ColoredString {
    let color_fn = ALIAS_COLORS[index % ALIAS_COLORS.len()];
    color_fn(alias)
}
