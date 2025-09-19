//! Terminal display width helpers.
//!
//! Provides ANSI-aware width calculation for rendered content so layout
//! padding stays aligned without requiring the full Boxy crate.

/// Compute the display width of a string after stripping ANSI escapes.
pub fn display_width(text: &str) -> usize {
    let clean = strip_ansi_escapes::strip(text);
    let clean_str = String::from_utf8_lossy(&clean);
    unicode_width::UnicodeWidthStr::width(&*clean_str)
}
