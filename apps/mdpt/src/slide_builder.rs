use crate::highlight::HighlightedLine;
use crate::slide::{AnimationType, ColumnAlignment, FrontMatter, ListItem, SlideContent, SlideElement};
use rust_pixel::render::style::{Color, Modifier, Style};
use rust_pixel::render::Buffer;
use rust_pixel::ui::*;
use rust_pixel::util::Rect;
use std::collections::HashMap;

/// Deferred Label widget to be added as Panel child after canvas rendering.
struct DeferredLabel {
    label: Label,
}

/// Build a UIPage for a given slide at a given step.
///
/// Only elements up to the step boundary are rendered (pause support).
pub fn build_slide_page(
    slide: &SlideContent,
    slide_idx: usize,
    step: usize,
    highlight_cache: &HashMap<(usize, usize), Vec<HighlightedLine>>,
    front_matter: &FrontMatter,
    width: u16,
    height: u16,
) -> UIPage {
    let margin = front_matter.margin;
    let content_width = width.saturating_sub(margin * 2);
    let content_height = height.saturating_sub(2); // 1 top margin + 1 bottom for status

    // Determine which elements to show based on step (pause boundaries)
    let boundary = slide.step_boundary(step);

    // Build the panel with canvas for direct rendering
    // FreeLayout ensures child Labels keep their manually set bounds
    let mut panel = Panel::new()
        .with_bounds(Rect::new(0, 0, width, height))
        .with_border(BorderStyle::None)
        .with_layout(Box::new(FreeLayout));
    panel.enable_canvas(width, height);

    let buf = panel.canvas_mut();

    // Collect deferred Label widgets for animated text
    let mut deferred_labels: Vec<DeferredLabel> = Vec::new();

    // Render visible elements
    let mut y: u16 = 1; // Start after top margin
    let mut jump_to_middle = false;
    let mut in_column_layout = false;
    let mut column_widths: Vec<u32> = Vec::new();
    let mut current_col: usize = 0;
    let mut col_x: u16 = margin;

    // First pass: if JumpToMiddle, calculate content height
    if boundary > 0 {
        if let Some(SlideElement::JumpToMiddle) = slide.elements.first() {
            // Calculate total content height of remaining elements
            let total_h = estimate_content_height(&slide.elements[1..boundary], content_width);
            let start_y = content_height.saturating_sub(total_h) / 2;
            y = start_y.max(1);
            jump_to_middle = true;
        }
    }

    for (ei, elem) in slide.elements.iter().enumerate() {
        if ei >= boundary {
            break;
        }
        if y >= content_height {
            break;
        }

        match elem {
            SlideElement::JumpToMiddle => {
                // Already handled above
            }
            SlideElement::Pause => {
                // Not rendered; boundary logic handles this
            }
            SlideElement::ColumnLayout { widths } => {
                in_column_layout = true;
                column_widths = widths.clone();
                current_col = 0;
                col_x = margin;
            }
            SlideElement::Column(idx) => {
                current_col = *idx;
                // Calculate x position for this column
                let total_weight: u32 = column_widths.iter().sum();
                let mut x_offset: u16 = margin;
                for i in 0..current_col {
                    if i < column_widths.len() {
                        x_offset += (content_width as u32 * column_widths[i] / total_weight) as u16;
                    }
                }
                col_x = x_offset;
                // Reset y to after the column layout header if this is not the first column
                if *idx > 0 {
                    // Don't reset y; columns share the same vertical space
                    // We'd need to track per-column y, but for simplicity let's use the same y
                }
            }
            SlideElement::ResetLayout => {
                in_column_layout = false;
                column_widths.clear();
                col_x = margin;
            }
            SlideElement::Title { level, text } => {
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };

                let style = title_style(*level);
                if *level == 1 && !jump_to_middle {
                    // Center h1 titles
                    let text_len = text.len() as u16;
                    let centered_x = x_start + w.saturating_sub(text_len) / 2;
                    buf.set_string(centered_x, y, text, style);
                } else {
                    buf.set_string(x_start, y, text, style);
                }
                y += 1;
                // Add spacing after titles
                if *level <= 2 {
                    y += 1;
                }
            }
            SlideElement::Paragraph { text } => {
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };

                let style = Style::default().fg(Color::White);
                let lines = wrap_text(text, w as usize);
                for line in &lines {
                    if y >= content_height {
                        break;
                    }
                    buf.set_string(x_start, y, line, style);
                    y += 1;
                }
                y += 1; // paragraph spacing
            }
            SlideElement::CodeBlock { language, code, line_numbers } => {
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };

                // Draw code block background
                let bg_style = Style::default()
                    .fg(Color::Gray)
                    .bg(Color::Rgba(40, 44, 52, 255));

                // Language label
                if !language.is_empty() {
                    let lang_label = format!(" {} ", language);
                    let lang_style = Style::default()
                        .fg(Color::Cyan)
                        .bg(Color::Rgba(40, 44, 52, 255));
                    buf.set_string(x_start, y, &lang_label, lang_style);
                    // Fill rest of line with bg
                    let fill = " ".repeat((w as usize).saturating_sub(lang_label.len()));
                    buf.set_string(x_start + lang_label.len() as u16, y, &fill, bg_style);
                    y += 1;
                }

                // Check if we have highlighted lines
                if let Some(hl_lines) = highlight_cache.get(&(slide_idx, ei)) {
                    let line_num_width = if *line_numbers {
                        format!("{}", hl_lines.len()).len() as u16 + 2
                    } else {
                        0
                    };

                    for (li, hl_line) in hl_lines.iter().enumerate() {
                        if y >= content_height {
                            break;
                        }

                        // Fill line background
                        let bg_fill = " ".repeat(w as usize);
                        buf.set_string(x_start, y, &bg_fill, bg_style);

                        let mut cx = x_start;

                        // Line numbers
                        if *line_numbers {
                            let num_str = format!("{:>width$} ", li + 1, width = (line_num_width - 2) as usize);
                            let num_style = Style::default()
                                .fg(Color::DarkGray)
                                .bg(Color::Rgba(40, 44, 52, 255));
                            buf.set_string(cx, y, &num_str, num_style);
                            cx += line_num_width;
                        }

                        // Render highlighted spans
                        for span in &hl_line.spans {
                            if cx >= x_start + w {
                                break;
                            }
                            let remaining = (x_start + w - cx) as usize;
                            let text = if span.text.len() > remaining {
                                &span.text[..remaining]
                            } else {
                                &span.text
                            };
                            buf.set_string(cx, y, text, span.style);
                            cx += text.len() as u16;
                        }

                        y += 1;
                    }
                } else {
                    // Fallback: render code without highlighting
                    let plain_style = Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgba(40, 44, 52, 255));
                    for line in code.lines() {
                        if y >= content_height {
                            break;
                        }
                        let bg_fill = " ".repeat(w as usize);
                        buf.set_string(x_start, y, &bg_fill, bg_style);
                        let truncated = if line.len() > w as usize {
                            &line[..w as usize]
                        } else {
                            line
                        };
                        buf.set_string(x_start, y, truncated, plain_style);
                        y += 1;
                    }
                }
                y += 1; // spacing after code block
            }
            SlideElement::List { items } => {
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };

                for item in items {
                    if y >= content_height {
                        break;
                    }
                    render_list_item(buf, x_start, y, w, item);
                    y += 1;
                }
                y += 1; // spacing after list
            }
            SlideElement::Table { headers, rows, alignments } => {
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };

                y = render_table(buf, x_start, y, w, headers, rows, alignments);
                y += 1; // spacing after table
            }
            SlideElement::Divider => {
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };
                let divider = "─".repeat(w as usize);
                let style = Style::default().fg(Color::DarkGray);
                buf.set_string(x_start, y, &divider, style);
                y += 1;
            }
            SlideElement::AnimatedText { text, animation } => {
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };

                // Draw emoji bullet on canvas (same as list items)
                let marker_style = Style::default().fg(Color::Cyan).scale(0.5, 0.5);
                buf.set_string(x_start, y, "🟢", marker_style);
                // Label starts after emoji(2) + space(1) = 3 cells
                let label_x = x_start + 3;
                let label_w = w.saturating_sub(3);
                let bounds = Rect::new(label_x, y, label_w, 1);
                let mut label = match animation {
                    AnimationType::Spotlight => {
                        let s = Style::default().fg(Color::Rgba(200, 200, 200, 255)).bg(Color::Reset);
                        let hl = Style::default().fg(Color::Rgba(80, 200, 255, 255)).bg(Color::Reset);
                        Label::new(text)
                            .with_style(s)
                            .with_spotlight(hl, 12, 0.55)
                    }
                    AnimationType::Wave => {
                        let s = Style::default().fg(Color::Rgba(255, 200, 80, 255)).bg(Color::Reset);
                        Label::new(text)
                            .with_style(s)
                            .with_wave(0.4, 6.0, 0.15)
                    }
                    AnimationType::FadeIn => {
                        let s = Style::default().fg(Color::Rgba(100, 255, 150, 255)).bg(Color::Reset);
                        Label::new(text)
                            .with_style(s)
                            .with_fade_in(8, true)
                    }
                    AnimationType::Typewriter => {
                        let s = Style::default().fg(Color::Rgba(255, 150, 200, 255)).bg(Color::Reset);
                        Label::new(text)
                            .with_style(s)
                            .with_typewriter(6, true, true)
                    }
                };
                label.set_bounds(bounds);
                deferred_labels.push(DeferredLabel { label });
                y += 1;
            }
            SlideElement::Image { path, alt } => {
                let (x_start, _w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };
                // Placeholder for image support
                let img_text = format!("[Image: {} ({})]", alt, path);
                let style = Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::DIM);
                buf.set_string(x_start, y, &img_text, style);
                y += 1;
            }
        }
    }

    // Add deferred Label widgets as Panel children (rendered on top of canvas)
    for dl in deferred_labels {
        panel.add_child(Box::new(dl.label));
    }

    let mut page = UIPage::new(width, height);
    page.set_root_widget(Box::new(panel));
    page.start();
    page
}

/// Calculate column width from weights.
fn col_width(weights: &[u32], col_idx: usize, total_width: u16) -> u16 {
    let total_weight: u32 = weights.iter().sum();
    if total_weight == 0 || col_idx >= weights.len() {
        return total_width;
    }
    (total_width as u32 * weights[col_idx] / total_weight) as u16
}

/// Estimate content height for vertical centering.
fn estimate_content_height(elements: &[SlideElement], width: u16) -> u16 {
    let mut h: u16 = 0;
    for elem in elements {
        match elem {
            SlideElement::Title { level, .. } => {
                h += 1;
                if *level <= 2 {
                    h += 1;
                }
            }
            SlideElement::Paragraph { text } => {
                let lines = wrap_text(text, width as usize);
                h += lines.len() as u16 + 1;
            }
            SlideElement::CodeBlock { code, .. } => {
                h += code.lines().count() as u16 + 2; // +1 for lang label, +1 spacing
            }
            SlideElement::List { items } => {
                h += items.len() as u16 + 1;
            }
            SlideElement::Table { rows, .. } => {
                h += rows.len() as u16 + 3; // header + separator + rows + spacing
            }
            SlideElement::Divider => h += 1,
            SlideElement::Image { .. } => h += 1,
            SlideElement::AnimatedText { .. } => h += 1,
            SlideElement::Pause | SlideElement::JumpToMiddle => {}
            SlideElement::ColumnLayout { .. } | SlideElement::Column(_) | SlideElement::ResetLayout => {}
        }
    }
    h
}

/// Style for heading levels.
fn title_style(level: u8) -> Style {
    match level {
        1 => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
            .scale(1.0, 1.0),
        2 => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
            .scale(1.0, 1.0),
        3 => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
            .scale(1.0, 1.0),
        _ => Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
            .scale(1.0, 1.0),
    }
}

/// Simple word-wrapping.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }

        let words: Vec<&str> = paragraph.split_whitespace().collect();
        let mut current_line = String::new();

        for word in words {
            if current_line.is_empty() {
                if word.len() > max_width {
                    // Break long words
                    let mut remaining = word;
                    while remaining.len() > max_width {
                        lines.push(remaining[..max_width].to_string());
                        remaining = &remaining[max_width..];
                    }
                    current_line = remaining.to_string();
                } else {
                    current_line = word.to_string();
                }
            } else if current_line.len() + 1 + word.len() > max_width {
                lines.push(current_line);
                current_line = word.to_string();
            } else {
                current_line.push(' ');
                current_line.push_str(word);
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

/// Render a list item with emoji bullet and indentation.
fn render_list_item(buf: &mut Buffer, x: u16, y: u16, _width: u16, item: &ListItem) {
    let indent_width = item.depth as u16 * 2;
    let indent = "  ".repeat(item.depth as usize);

    let prefix_style = Style::default().fg(Color::Cyan);
    let text_style = Style::default().fg(Color::White);

    if item.ordered {
        let bullet = format!("{}{}. ", indent, item.index);
        let w = bullet.len() as u16;
        buf.set_string(x, y, &bullet, prefix_style);
        buf.set_string(x + w, y, &item.text, text_style);
    } else {
        // Emoji bullets from tui.txt (double-width: 2 cells each), scaled to 0.5
        let marker = match item.depth {
            0 => "🟢",
            1 => "🔵",
            _ => "🟡",
        };
        let marker_style = Style::default().fg(Color::Cyan).scale(0.5, 0.5);
        buf.set_string(x, y, &indent, prefix_style);
        buf.set_string(x + indent_width, y, marker, marker_style);
        // emoji(2 cells) + space(1 cell) = 3
        buf.set_string(x + indent_width + 3, y, &item.text, text_style);
    }
}

/// Render a table.
fn render_table(
    buf: &mut Buffer,
    x: u16,
    mut y: u16,
    max_width: u16,
    headers: &[String],
    rows: &[Vec<String>],
    alignments: &[ColumnAlignment],
) -> u16 {
    if headers.is_empty() {
        return y;
    }

    let num_cols = headers.len();
    // Calculate column widths
    let col_width = (max_width as usize / num_cols).max(3);

    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let separator_style = Style::default().fg(Color::DarkGray);
    let cell_style = Style::default().fg(Color::White);

    // Header row
    let mut cx = x;
    for (i, header) in headers.iter().enumerate() {
        let text = align_text(header, col_width, alignments.get(i).copied());
        buf.set_string(cx, y, &text, header_style);
        cx += col_width as u16;
    }
    y += 1;

    // Separator
    let separator = "─".repeat(col_width - 1);
    cx = x;
    for _ in 0..num_cols {
        buf.set_string(cx, y, &separator, separator_style);
        cx += col_width as u16;
    }
    y += 1;

    // Data rows
    for row in rows {
        cx = x;
        for (i, cell) in row.iter().enumerate() {
            let text = align_text(cell, col_width, alignments.get(i).copied());
            buf.set_string(cx, y, &text, cell_style);
            cx += col_width as u16;
        }
        y += 1;
    }

    y
}

/// Align text within a column width.
fn align_text(text: &str, width: usize, alignment: Option<ColumnAlignment>) -> String {
    let truncated = if text.len() > width - 1 {
        &text[..width - 1]
    } else {
        text
    };

    match alignment.unwrap_or(ColumnAlignment::Left) {
        ColumnAlignment::Left | ColumnAlignment::None => {
            format!("{:<width$}", truncated, width = width)
        }
        ColumnAlignment::Center => {
            format!("{:^width$}", truncated, width = width)
        }
        ColumnAlignment::Right => {
            format!("{:>width$}", truncated, width = width)
        }
    }
}
