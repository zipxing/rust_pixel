use crate::slide::*;
use comrak::{
    Arena, Options,
    nodes::{AstNode, ListType, NodeCodeBlock, NodeList, NodeValue},
    parse_document,
};
use serde::Deserialize;

/// Parse animation name string to AnimationType.
fn parse_animation_type(name: &str) -> Option<AnimationType> {
    match name.to_lowercase().as_str() {
        "spotlight" => Some(AnimationType::Spotlight),
        "wave" => Some(AnimationType::Wave),
        "fadein" | "fade_in" => Some(AnimationType::FadeIn),
        "typewriter" => Some(AnimationType::Typewriter),
        _ => None,
    }
}

/// Parse a markdown string into a Presentation.
pub fn parse_markdown(contents: &str) -> Presentation {
    let arena = Arena::new();
    let mut options = Options::default();
    options.extension.front_matter_delimiter = Some("---".into());
    options.extension.table = true;
    options.extension.strikethrough = true;

    let root = parse_document(&arena, contents, &options);

    let mut front_matter = FrontMatter::default();
    let mut slides = Vec::new();
    let mut current_slide = SlideContent::new();
    let mut pending_anim: Option<AnimationType> = None;

    for node in root.children() {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::FrontMatter(fm_content) => {
                front_matter = parse_front_matter(fm_content);
            }
            NodeValue::ThematicBreak => {
                // --- splits slides (like presenterm's end_slide_shorthand)
                slides.push(current_slide);
                current_slide = SlideContent::new();
                pending_anim = None;
            }
            NodeValue::HtmlBlock(block) => {
                // Check if it's a comment command
                if let Some(cmd) = parse_html_comment(&block.literal) {
                    match cmd {
                        CommentCommand::EndSlide => {
                            slides.push(current_slide);
                            current_slide = SlideContent::new();
                            pending_anim = None;
                        }
                        CommentCommand::Pause => {
                            current_slide.elements.push(SlideElement::Pause);
                        }
                        CommentCommand::JumpToMiddle => {
                            current_slide.elements.push(SlideElement::JumpToMiddle);
                        }
                        CommentCommand::ColumnLayout(widths) => {
                            current_slide
                                .elements
                                .push(SlideElement::ColumnLayout { widths });
                        }
                        CommentCommand::Column(idx) => {
                            current_slide.elements.push(SlideElement::Column(idx));
                        }
                        CommentCommand::ResetLayout => {
                            current_slide.elements.push(SlideElement::ResetLayout);
                        }
                        CommentCommand::Anim(name) => {
                            pending_anim = parse_animation_type(&name);
                        }
                        CommentCommand::Spacer(n) => {
                            current_slide.elements.push(SlideElement::Spacer(n));
                        }
                        CommentCommand::Divider => {
                            current_slide.elements.push(SlideElement::Divider);
                        }
                    }
                }
                // Non-comment HTML blocks are ignored
            }
            NodeValue::Heading(heading) => {
                let text = collect_text(node);
                current_slide.elements.push(SlideElement::Title {
                    level: heading.level,
                    text,
                });
            }
            NodeValue::Paragraph => {
                // Check if paragraph contains an image
                if let Some(image) = extract_image(node) {
                    current_slide.elements.push(image);
                } else {
                    let text = collect_text(node);
                    if !text.is_empty() {
                        if let Some(anim) = pending_anim.take() {
                            current_slide.elements.push(SlideElement::AnimatedText {
                                text,
                                animation: anim,
                            });
                        } else {
                            current_slide
                                .elements
                                .push(SlideElement::Paragraph { text });
                        }
                    }
                }
            }
            NodeValue::CodeBlock(block) => {
                let element = parse_code_block(block);
                // Insert extra Pause elements for dynamic highlight groups
                let extra_pauses = if let SlideElement::CodeBlock {
                    ref highlight_groups, ..
                } = element
                {
                    if highlight_groups.len() > 1 {
                        highlight_groups.len() - 1
                    } else {
                        0
                    }
                } else {
                    0
                };
                current_slide.elements.push(element);
                for _ in 0..extra_pauses {
                    current_slide.elements.push(SlideElement::Pause);
                }
            }
            NodeValue::List(_) => {
                let items = parse_list(node, 0);
                current_slide.elements.push(SlideElement::List { items });
            }
            NodeValue::Table(_) => {
                let table = parse_table(node);
                current_slide.elements.push(table);
            }
            NodeValue::BlockQuote => {
                let (alert_type, text) = parse_block_quote(node);
                if !text.is_empty() {
                    current_slide.elements.push(SlideElement::BlockQuote {
                        text,
                        alert_type,
                    });
                }
            }
            _ => {
                // Skip unsupported elements
            }
        }
    }

    // Don't forget the last slide
    if !current_slide.elements.is_empty() {
        slides.push(current_slide);
    }

    // Ensure at least one slide
    if slides.is_empty() {
        slides.push(SlideContent::new());
    }

    Presentation {
        front_matter,
        slides,
    }
}

/// Parse YAML front matter content into FrontMatter struct.
fn parse_front_matter(contents: &str) -> FrontMatter {
    // Strip --- delimiters (comrak may include them)
    let contents = contents.strip_prefix("---\n").unwrap_or(contents);
    let contents = contents.strip_prefix("---\r\n").unwrap_or(contents);
    let contents = contents.strip_suffix("---\n").unwrap_or(contents);
    let contents = contents.strip_suffix("---\r\n").unwrap_or(contents);
    let contents = contents.strip_suffix("---\n\n").unwrap_or(contents);
    let contents = contents.strip_suffix("---\r\n\r\n").unwrap_or(contents);

    serde_yaml::from_str(contents).unwrap_or_default()
}

/// Parse an HTML block as a comment command.
/// Returns None if it's not a recognized comment command.
fn parse_html_comment(literal: &str) -> Option<CommentCommand> {
    let block = literal.trim();
    let start_tag = "<!--";
    let end_tag = "-->";
    if !block.starts_with(start_tag) || !block.ends_with(end_tag) {
        return None;
    }
    let inner = &block[start_tag.len()..block.len() - end_tag.len()];
    let inner = inner.trim();

    // Parse using serde_yaml (same approach as presenterm)
    #[derive(Deserialize)]
    struct CommandWrapper(#[serde(with = "serde_yaml::with::singleton_map")] CommentCommand);

    serde_yaml::from_str::<CommandWrapper>(inner)
        .map(|w| w.0)
        .ok()
}

/// Comment commands parsed from HTML comments (presenterm-compatible).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CommentCommand {
    EndSlide,
    Pause,
    JumpToMiddle,
    #[serde(rename = "column_layout")]
    ColumnLayout(Vec<u32>),
    Column(usize),
    ResetLayout,
    /// Text animation: spotlight, wave, fadein, typewriter
    Anim(String),
    /// Vertical spacer
    Spacer(u16),
    /// In-page horizontal divider line
    Divider,
}

/// Recursively collect all text content from a node and its children.
fn collect_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();
    collect_text_recursive(node, &mut text);
    text
}

fn collect_text_recursive<'a>(node: &'a AstNode<'a>, out: &mut String) {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Text(t) => {
            out.push_str(t);
        }
        NodeValue::Code(code) => {
            out.push('`');
            out.push_str(&code.literal);
            out.push('`');
        }
        NodeValue::SoftBreak => {
            out.push(' ');
        }
        NodeValue::LineBreak => {
            out.push('\n');
        }
        _ => {}
    }
    for child in node.children() {
        collect_text_recursive(child, out);
    }
}

/// Check if a paragraph node contains an image and extract it.
fn extract_image<'a>(node: &'a AstNode<'a>) -> Option<SlideElement> {
    for child in node.children() {
        let data = child.data.borrow();
        if let NodeValue::Image(link) = &data.value {
            let alt = collect_text(child);
            return Some(SlideElement::Image {
                path: link.url.clone(),
                alt,
            });
        }
    }
    None
}

/// Parse a code block, extracting language, +line_numbers, +no_background, and {highlight} groups.
fn parse_code_block(block: &NodeCodeBlock) -> SlideElement {
    let info = block.info.trim().to_string();
    let mut language = String::new();
    let mut line_numbers = false;
    let mut no_background = false;
    let mut highlight_groups: Vec<Vec<LineRange>> = Vec::new();

    for part in info.split_whitespace() {
        if part == "+line_numbers" {
            line_numbers = true;
        } else if part == "+no_background" {
            no_background = true;
        } else if part.starts_with('{') && part.ends_with('}') {
            highlight_groups = parse_highlight_groups(&part[1..part.len() - 1]);
        } else if language.is_empty() {
            // Language may have highlight spec attached, e.g. "rust{1-4|6-10}"
            if let Some(brace_pos) = part.find('{') {
                language = part[..brace_pos].to_string();
                if part.ends_with('}') {
                    highlight_groups =
                        parse_highlight_groups(&part[brace_pos + 1..part.len() - 1]);
                }
            } else {
                language = part.to_string();
            }
        }
    }

    SlideElement::CodeBlock {
        language,
        code: block.literal.clone(),
        line_numbers,
        no_background,
        highlight_groups,
    }
}

/// Parse highlight group spec like "1-4|6-10|all" into Vec<Vec<LineRange>>.
fn parse_highlight_groups(spec: &str) -> Vec<Vec<LineRange>> {
    spec.split('|')
        .map(|group| {
            group
                .split(',')
                .filter_map(|part| {
                    let part = part.trim();
                    if part.eq_ignore_ascii_case("all") {
                        Some(LineRange::All)
                    } else if let Some(dash) = part.find('-') {
                        let start = part[..dash].trim().parse::<usize>().ok()?;
                        let end = part[dash + 1..].trim().parse::<usize>().ok()?;
                        Some(LineRange::Range(start, end))
                    } else {
                        let n = part.parse::<usize>().ok()?;
                        Some(LineRange::Single(n))
                    }
                })
                .collect()
        })
        .filter(|g: &Vec<LineRange>| !g.is_empty())
        .collect()
}

/// Parse a block quote node, detecting GitHub-style alerts.
/// Returns (alert_type, text_content).
fn parse_block_quote<'a>(node: &'a AstNode<'a>) -> (Option<AlertType>, String) {
    let mut paragraphs = Vec::new();
    for child in node.children() {
        let data = child.data.borrow();
        if matches!(&data.value, NodeValue::Paragraph) {
            drop(data);
            paragraphs.push(collect_text(child));
        }
    }

    let full_text = paragraphs.join("\n");

    // Check for GitHub-style alert: text starts with [!TYPE]
    // comrak may join lines with spaces, so check for "[!type]" prefix
    let trimmed = full_text.trim_start();
    if trimmed.starts_with("[!") {
        if let Some(close) = trimmed.find(']') {
            let alert_name = &trimmed[2..close];
            let alert_type = match alert_name.to_lowercase().as_str() {
                "note" => Some(AlertType::Note),
                "tip" => Some(AlertType::Tip),
                "important" => Some(AlertType::Important),
                "warning" => Some(AlertType::Warning),
                "caution" => Some(AlertType::Caution),
                _ => None,
            };
            if alert_type.is_some() {
                // Remaining text after the [!TYPE] marker
                let rest = trimmed[close + 1..].trim().to_string();
                return (alert_type, rest);
            }
        }
    }

    (None, full_text)
}

/// Parse a list node into flat ListItems with depth (like presenterm).
fn parse_list<'a>(node: &'a AstNode<'a>, depth: u8) -> Vec<ListItem> {
    let mut items = Vec::new();
    for child in node.children() {
        let data = child.data.borrow();
        if let NodeValue::Item(item) = &data.value {
            parse_list_item(child, item, depth, &mut items);
        }
    }
    items
}

fn parse_list_item<'a>(
    node: &'a AstNode<'a>,
    item: &NodeList,
    depth: u8,
    items: &mut Vec<ListItem>,
) {
    let ordered = matches!(item.list_type, ListType::Ordered);
    let index = item.start;

    for child in node.children() {
        let data = child.data.borrow();
        match &data.value {
            NodeValue::Paragraph => {
                let text = collect_text(child);
                items.push(ListItem {
                    text,
                    depth,
                    ordered,
                    index,
                });
            }
            NodeValue::List(_) => {
                let nested = parse_list(child, depth + 1);
                items.extend(nested);
            }
            _ => {}
        }
    }
}

/// Parse a table node.
fn parse_table<'a>(node: &'a AstNode<'a>) -> SlideElement {
    let mut headers = Vec::new();
    let mut rows = Vec::new();
    let mut alignments = Vec::new();

    // Get alignment info from the table node
    let data = node.data.borrow();
    if let NodeValue::Table(table) = &data.value {
        alignments = table
            .alignments
            .iter()
            .map(|a| match a {
                comrak::nodes::TableAlignment::Left => ColumnAlignment::Left,
                comrak::nodes::TableAlignment::Center => ColumnAlignment::Center,
                comrak::nodes::TableAlignment::Right => ColumnAlignment::Right,
                comrak::nodes::TableAlignment::None => ColumnAlignment::None,
            })
            .collect();
    }
    drop(data);

    for child in node.children() {
        let data = child.data.borrow();
        if let NodeValue::TableRow(is_header) = &data.value {
            let is_header = *is_header;
            drop(data);
            let cells = parse_table_row(child);
            if is_header {
                headers = cells;
            } else {
                rows.push(cells);
            }
        }
    }

    SlideElement::Table {
        headers,
        rows,
        alignments,
    }
}

fn parse_table_row<'a>(node: &'a AstNode<'a>) -> Vec<String> {
    let mut cells = Vec::new();
    for child in node.children() {
        let data = child.data.borrow();
        if matches!(&data.value, NodeValue::TableCell) {
            drop(data);
            cells.push(collect_text(child));
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_markdown() {
        let pres = parse_markdown("");
        assert_eq!(pres.slides.len(), 1);
        assert!(pres.slides[0].elements.is_empty());
    }

    #[test]
    fn test_front_matter() {
        let md = r#"---
title: Test
theme: light
transition: wipe_left
margin: 4
---

# Hello
"#;
        let pres = parse_markdown(md);
        assert_eq!(pres.front_matter.title, "Test");
        assert_eq!(pres.front_matter.theme, "light");
        assert_eq!(pres.front_matter.transition, "wipe_left");
        assert_eq!(pres.front_matter.margin, 4);
    }

    #[test]
    fn test_slide_split_by_thematic_break() {
        let md = r#"# Slide 1

---

# Slide 2

---

# Slide 3
"#;
        let pres = parse_markdown(md);
        assert_eq!(pres.slides.len(), 3);
    }

    #[test]
    fn test_slide_split_by_end_slide() {
        let md = r#"# Slide 1

<!-- end_slide -->

# Slide 2
"#;
        let pres = parse_markdown(md);
        assert_eq!(pres.slides.len(), 2);
    }

    #[test]
    fn test_pause() {
        let md = r#"Line 1

<!-- pause -->

Line 2

<!-- pause -->

Line 3
"#;
        let pres = parse_markdown(md);
        assert_eq!(pres.slides.len(), 1);
        let slide = &pres.slides[0];
        assert_eq!(slide.step_count(), 3);
        // Step 0 should show up to first pause
        assert_eq!(slide.step_boundary(0), 1); // [Paragraph("Line 1")]
    }

    #[test]
    fn test_code_block_with_line_numbers() {
        let md = r#"```rust +line_numbers
fn main() {
    println!("hello");
}
```
"#;
        let pres = parse_markdown(md);
        let slide = &pres.slides[0];
        match &slide.elements[0] {
            SlideElement::CodeBlock {
                language,
                line_numbers,
                ..
            } => {
                assert_eq!(language, "rust");
                assert!(line_numbers);
            }
            other => panic!("expected CodeBlock, got {:?}", other),
        }
    }

    #[test]
    fn test_column_layout() {
        let md = r#"<!-- column_layout: [1, 2] -->
<!-- column: 0 -->

Left content

<!-- column: 1 -->

Right content

<!-- reset_layout -->
"#;
        let pres = parse_markdown(md);
        let elems = &pres.slides[0].elements;
        assert!(matches!(
            &elems[0],
            SlideElement::ColumnLayout { widths } if widths == &[1, 2]
        ));
        assert!(matches!(&elems[1], SlideElement::Column(0)));
        assert!(matches!(&elems[3], SlideElement::Column(1)));
        assert!(matches!(
            elems.last().unwrap(),
            SlideElement::ResetLayout
        ));
    }

    #[test]
    fn test_image() {
        let md = r#"![Logo](assets/logo.pix)
"#;
        let pres = parse_markdown(md);
        match &pres.slides[0].elements[0] {
            SlideElement::Image { path, alt } => {
                assert_eq!(path, "assets/logo.pix");
                assert_eq!(alt, "Logo");
            }
            other => panic!("expected Image, got {:?}", other),
        }
    }

    #[test]
    fn test_list() {
        let md = r#"* One
  * Sub1
  * Sub2
* Two
"#;
        let pres = parse_markdown(md);
        match &pres.slides[0].elements[0] {
            SlideElement::List { items } => {
                assert_eq!(items.len(), 4);
                assert_eq!(items[0].depth, 0);
                assert_eq!(items[0].text, "One");
                assert_eq!(items[1].depth, 1);
                assert_eq!(items[2].depth, 1);
                assert_eq!(items[3].depth, 0);
            }
            other => panic!("expected List, got {:?}", other),
        }
    }

    #[test]
    fn test_table() {
        let md = r#"| Name | Value |
|------|-------|
| A    | 1     |
| B    | 2     |
"#;
        let pres = parse_markdown(md);
        match &pres.slides[0].elements[0] {
            SlideElement::Table {
                headers, rows, ..
            } => {
                assert_eq!(headers.len(), 2);
                assert_eq!(rows.len(), 2);
                assert_eq!(headers[0], "Name");
            }
            other => panic!("expected Table, got {:?}", other),
        }
    }

    #[test]
    fn test_jump_to_middle() {
        let md = r#"<!-- jump_to_middle -->

# Centered Title
"#;
        let pres = parse_markdown(md);
        assert!(matches!(
            &pres.slides[0].elements[0],
            SlideElement::JumpToMiddle
        ));
    }

    #[test]
    fn test_divider() {
        let md = r#"# Title

<!-- divider -->

Some text
"#;
        let pres = parse_markdown(md);
        let elems = &pres.slides[0].elements;
        assert!(matches!(&elems[0], SlideElement::Title { .. }));
        assert!(matches!(&elems[1], SlideElement::Divider));
        assert!(matches!(&elems[2], SlideElement::Paragraph { .. }));
    }

    #[test]
    fn test_author_front_matter() {
        let md = r#"---
title: Test
author: Alice
---

# Hello
"#;
        let pres = parse_markdown(md);
        assert_eq!(pres.front_matter.author, "Alice");
    }

    #[test]
    fn test_no_background_code_block() {
        let md = r#"```cpp +no_background +line_numbers
int x = 42;
```
"#;
        let pres = parse_markdown(md);
        match &pres.slides[0].elements[0] {
            SlideElement::CodeBlock {
                language,
                line_numbers,
                no_background,
                ..
            } => {
                assert_eq!(language, "cpp");
                assert!(line_numbers);
                assert!(no_background);
            }
            other => panic!("expected CodeBlock, got {:?}", other),
        }
    }

    #[test]
    fn test_highlight_groups() {
        let md = r#"```rust {1-4|6-10|all} +line_numbers
fn main() {}
```
"#;
        let pres = parse_markdown(md);
        match &pres.slides[0].elements[0] {
            SlideElement::CodeBlock {
                highlight_groups, ..
            } => {
                assert_eq!(highlight_groups.len(), 3);
                // Group 0: 1-4
                assert!(matches!(highlight_groups[0][0], LineRange::Range(1, 4)));
                // Group 1: 6-10
                assert!(matches!(highlight_groups[1][0], LineRange::Range(6, 10)));
                // Group 2: all
                assert!(matches!(highlight_groups[2][0], LineRange::All));
            }
            other => panic!("expected CodeBlock, got {:?}", other),
        }
        // Should also have 2 extra Pause elements
        let elems = &pres.slides[0].elements;
        assert!(matches!(&elems[1], SlideElement::Pause));
        assert!(matches!(&elems[2], SlideElement::Pause));
    }

    #[test]
    fn test_highlight_groups_attached_to_language() {
        let md = r#"```rust{1|3|5}
code
```
"#;
        let pres = parse_markdown(md);
        match &pres.slides[0].elements[0] {
            SlideElement::CodeBlock {
                language,
                highlight_groups,
                ..
            } => {
                assert_eq!(language, "rust");
                assert_eq!(highlight_groups.len(), 3);
                assert!(matches!(highlight_groups[0][0], LineRange::Single(1)));
                assert!(matches!(highlight_groups[1][0], LineRange::Single(3)));
                assert!(matches!(highlight_groups[2][0], LineRange::Single(5)));
            }
            other => panic!("expected CodeBlock, got {:?}", other),
        }
    }

    #[test]
    fn test_block_quote() {
        let md = r#"> This is a block quote
> spanning multiple lines
"#;
        let pres = parse_markdown(md);
        match &pres.slides[0].elements[0] {
            SlideElement::BlockQuote { text, alert_type } => {
                assert!(text.contains("block quote"));
                assert!(alert_type.is_none());
            }
            other => panic!("expected BlockQuote, got {:?}", other),
        }
    }

    #[test]
    fn test_github_alert_caution() {
        let md = r#"> [!caution]
> Be very careful here
"#;
        let pres = parse_markdown(md);
        match &pres.slides[0].elements[0] {
            SlideElement::BlockQuote { text, alert_type } => {
                assert!(matches!(alert_type, Some(AlertType::Caution)));
                assert!(text.contains("careful"));
            }
            other => panic!("expected BlockQuote, got {:?}", other),
        }
    }

    #[test]
    fn test_github_alert_note() {
        let md = r#"> [!note]
> This is a note
"#;
        let pres = parse_markdown(md);
        match &pres.slides[0].elements[0] {
            SlideElement::BlockQuote { text, alert_type } => {
                assert!(matches!(alert_type, Some(AlertType::Note)));
                assert!(text.contains("note"));
            }
            other => panic!("expected BlockQuote, got {:?}", other),
        }
    }
}
