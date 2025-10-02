//! Clipboard support using OSC 52 escape sequences
//!
//! OSC 52 is a terminal escape sequence that allows applications to write to the system
//! clipboard. This works even over SSH connections and in terminal multiplexers like tmux.
//!
//! The format is: `\x1b]52;c;<base64_content>\x1b\\` or `\x1b]52;c;<base64_content>\x07`
//!
//! References:
//! - https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h3-Operating-System-Commands
//! - https://github.com/ojroques/vim-oscyank

use base64::{Engine, engine::general_purpose::STANDARD};
use std::io::{self, Write};

/// Copy text to the clipboard using OSC 52 escape sequence
///
/// # Arguments
/// * `text` - The text to copy to the clipboard
///
/// # Returns
/// Returns Ok(()) if the escape sequence was successfully written to stdout,
/// or an io::Error if writing failed.
///
/// # Examples
/// ```no_run
/// use blz_cli::utils::clipboard::copy_to_clipboard;
///
/// copy_to_clipboard("Hello, clipboard!")?;
/// # Ok::<(), std::io::Error>(())
/// ```
pub fn copy_to_clipboard(text: &str) -> io::Result<()> {
    let encoded = STANDARD.encode(text);

    // Use the BEL terminator (\x07) for better compatibility
    // Some terminals don't support the ST terminator (\x1b\\)
    let osc52 = format!("\x1b]52;c;{encoded}\x07");

    // Write directly to stderr to avoid interfering with normal output
    // that might be piped or redirected
    io::stderr().write_all(osc52.as_bytes())?;
    io::stderr().flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_copy() {
        // Just verify it doesn't panic
        // We can't actually test clipboard functionality in unit tests
        let result = copy_to_clipboard("test content");
        // In test environment, this might fail if stderr is not available,
        // but the function should at least execute without panicking
        let _ = result;
    }

    #[test]
    fn test_empty_string() {
        let result = copy_to_clipboard("");
        let _ = result;
    }

    #[test]
    fn test_multiline_content() {
        let content = "line 1\nline 2\nline 3";
        let result = copy_to_clipboard(content);
        let _ = result;
    }

    #[test]
    fn test_special_characters() {
        let content = "Special: !@#$%^&*()_+-=[]{}|;':\",./<>?";
        let result = copy_to_clipboard(content);
        let _ = result;
    }
}
