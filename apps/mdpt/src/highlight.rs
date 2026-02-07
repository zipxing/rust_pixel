use rust_pixel::render::style::{Color, Modifier, Style};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

/// A styled text span using RustPixel's Style system.
/// Can be rendered directly via `buffer.set_str(x, y, &span.text, span.style)`.
#[derive(Debug, Clone)]
pub struct StyledSpan {
    pub text: String,
    pub style: Style,
}

/// A highlighted line of code, composed of styled spans.
#[derive(Debug, Clone)]
pub struct HighlightedLine {
    pub spans: Vec<StyledSpan>,
}

/// Code highlighter wrapping syntect.
pub struct CodeHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl CodeHighlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Highlight a code block, returning one HighlightedLine per source line.
    ///
    /// `language` is the language identifier (e.g. "rust", "python", "js").
    /// `theme_name` is the syntect theme name (e.g. "base16-ocean.dark").
    /// Falls back to plain text if language or theme is not found.
    pub fn highlight(&self, code: &str, language: &str, theme_name: &str) -> Vec<HighlightedLine> {
        let syntax = self
            .syntax_set
            .find_syntax_by_token(language)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = self
            .theme_set
            .themes
            .get(theme_name)
            .unwrap_or_else(|| {
                self.theme_set
                    .themes
                    .values()
                    .next()
                    .expect("no themes available")
            });

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut lines = Vec::new();

        for line in code.lines() {
            let ranges = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            let spans: Vec<StyledSpan> = ranges
                .into_iter()
                .map(|(st, text)| {
                    let style = syntect_style_to_pixel(st);
                    StyledSpan {
                        text: text.to_string(),
                        style,
                    }
                })
                .collect();

            lines.push(HighlightedLine { spans });
        }

        lines
    }

    /// List available theme names.
    pub fn theme_names(&self) -> Vec<&str> {
        self.theme_set.themes.keys().map(|s| s.as_str()).collect()
    }
}

/// Convert a syntect Style to a RustPixel Style.
/// Maps foreground, background colors and font_style (bold/italic/underline).
fn syntect_style_to_pixel(st: syntect::highlighting::Style) -> Style {
    let mut style = Style::default()
        .fg(Color::Rgba(st.foreground.r, st.foreground.g, st.foreground.b, st.foreground.a))
        .bg(Color::Rgba(st.background.r, st.background.g, st.background.b, st.background.a));

    let mut modifier = Modifier::empty();
    if st.font_style.contains(FontStyle::BOLD) {
        modifier.insert(Modifier::BOLD);
    }
    if st.font_style.contains(FontStyle::ITALIC) {
        modifier.insert(Modifier::ITALIC);
    }
    if st.font_style.contains(FontStyle::UNDERLINE) {
        modifier.insert(Modifier::UNDERLINED);
    }
    if !modifier.is_empty() {
        style = style.add_modifier(modifier);
    }

    style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust() {
        let hl = CodeHighlighter::new();
        let code = "fn main() {\n    println!(\"hello\");\n}\n";
        let lines = hl.highlight(code, "rust", "base16-ocean.dark");
        assert_eq!(lines.len(), 3);
        for line in &lines {
            assert!(!line.spans.is_empty());
            // Each span should have a Style with fg and bg set
            for span in &line.spans {
                assert!(span.style.fg.is_some());
                assert!(span.style.bg.is_some());
            }
        }
    }

    #[test]
    fn test_highlight_unknown_language() {
        let hl = CodeHighlighter::new();
        let code = "hello world\n";
        let lines = hl.highlight(code, "nonexistent_language_xyz", "base16-ocean.dark");
        assert_eq!(lines.len(), 1);
        assert!(!lines[0].spans.is_empty());
    }

    #[test]
    fn test_highlight_unknown_theme_fallback() {
        let hl = CodeHighlighter::new();
        let code = "let x = 1;\n";
        let lines = hl.highlight(code, "rust", "nonexistent_theme_xyz");
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_available_themes() {
        let hl = CodeHighlighter::new();
        let themes = hl.theme_names();
        assert!(!themes.is_empty());
        assert!(themes.contains(&"base16-ocean.dark"));
    }

    #[test]
    fn test_style_has_modifiers() {
        let hl = CodeHighlighter::new();
        // Markdown bold syntax should produce bold modifier in some themes
        let code = "**bold text**\n";
        let lines = hl.highlight(code, "md", "base16-ocean.dark");
        assert!(!lines.is_empty());
        // Just verify it doesn't panic and produces spans
        assert!(!lines[0].spans.is_empty());
    }
}
