// RustPixel UI Framework - Text Utilities
// copyright zipxing@hotmail.com 2022～2025

//! Public text utility functions for UI components.

use unicode_width::UnicodeWidthStr;
use unicode_width::UnicodeWidthChar;

/// Word-wrap text to fit within `max_width` cells (unicode-aware).
///
/// Features:
/// - Respects `\n` paragraph breaks
/// - Uses `unicode-width` for correct CJK/emoji width calculation
/// - Breaks long words that exceed `max_width`
/// - Returns `vec![]` if `max_width` is 0
///
/// # Examples
/// ```
/// use rust_pixel::ui::text_util::wrap_text;
/// let lines = wrap_text("hello world", 5);
/// assert_eq!(lines, vec!["hello", "world"]);
/// ```
pub fn wrap_text(text: &str, max_width: u16) -> Vec<String> {
    if max_width == 0 {
        return vec![];
    }
    let max_w = max_width as usize;

    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current_line = String::new();
        let mut current_width: usize = 0;

        for word in paragraph.split_whitespace() {
            let word_width = word.width();

            if current_line.is_empty() {
                if word_width > max_w {
                    // Break long words by character
                    let mut w = 0;
                    let mut part = String::new();
                    for ch in word.chars() {
                        let cw = ch.width().unwrap_or(0);
                        if w + cw > max_w && !part.is_empty() {
                            lines.push(part);
                            part = String::new();
                            w = 0;
                        }
                        part.push(ch);
                        w += cw;
                    }
                    current_line = part;
                    current_width = w;
                } else {
                    current_line = word.to_string();
                    current_width = word_width;
                }
            } else if current_width + 1 + word_width > max_w {
                lines.push(current_line);
                current_line = word.to_string();
                current_width = word_width;
            } else {
                current_line.push(' ');
                current_line.push_str(word);
                current_width += 1 + word_width;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// Return the display width of a string in cells (unicode-aware).
///
/// CJK characters and some emoji count as 2 cells; ASCII counts as 1.
pub fn display_width(text: &str) -> usize {
    text.width()
}

/// Truncate a string to fit within `max_width` display cells (unicode-safe).
///
/// Unlike byte slicing (`&s[..n]`), this respects multi-byte characters and
/// double-width CJK/emoji glyphs. Returns an owned String.
pub fn truncate_to_width(text: &str, max_width: usize) -> String {
    let text_w = text.width();
    if text_w <= max_width {
        return text.to_string();
    }
    let mut s = String::new();
    let mut w = 0;
    for ch in text.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > max_width {
            break;
        }
        s.push(ch);
        w += cw;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_wrap() {
        let lines = wrap_text("hello world", 5);
        assert_eq!(lines, vec!["hello", "world"]);
    }

    #[test]
    fn test_no_wrap_needed() {
        let lines = wrap_text("short", 20);
        assert_eq!(lines, vec!["short"]);
    }

    #[test]
    fn test_paragraph_breaks() {
        let lines = wrap_text("line1\nline2\nline3", 20);
        assert_eq!(lines, vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn test_empty_paragraph() {
        let lines = wrap_text("before\n\nafter", 20);
        assert_eq!(lines, vec!["before", "", "after"]);
    }

    #[test]
    fn test_long_word_break() {
        let lines = wrap_text("abcdefghij", 4);
        assert_eq!(lines, vec!["abcd", "efgh", "ij"]);
    }

    #[test]
    fn test_zero_width() {
        let lines = wrap_text("hello", 0);
        assert!(lines.is_empty());
    }

    #[test]
    fn test_empty_text() {
        let lines = wrap_text("", 10);
        assert_eq!(lines, vec![""]);
    }

    #[test]
    fn test_cjk_width() {
        // CJK characters are 2 cells wide
        let lines = wrap_text("你好世界", 5);
        // "你好" = 4 cells, "世" would make 6 > 5, so wrap
        assert_eq!(lines, vec!["你好", "世界"]);
    }

    #[test]
    fn test_mixed_content() {
        let lines = wrap_text("hello world\n\nrust is great", 12);
        assert_eq!(lines, vec!["hello world", "", "rust is", "great"]);
    }
}
