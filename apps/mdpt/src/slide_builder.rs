use crate::highlight::HighlightedLine;
use crate::slide::{AnimationType, ColumnAlignment, FrontMatter, SlideContent, SlideElement};
use rust_pixel::render::style::{Color, Modifier, Style};
use rust_pixel::ui::*;
use rust_pixel::util::Rect;
use std::collections::HashMap;

/// Deferred widgets to be added as Panel children after canvas rendering.
type DeferredWidgets = Vec<Box<dyn rust_pixel::ui::Widget>>;

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

    // Collect deferred widgets (Labels, PresentList, PresentTable) added as Panel children
    let mut deferred_widgets: DeferredWidgets = Vec::new();

    // Render visible elements
    let mut y: u16 = 1; // Start after top margin
    let mut jump_to_middle = false;
    let mut in_column_layout = false;
    let mut column_widths: Vec<u32> = Vec::new();
    let mut current_col: usize = 0;
    let mut col_x: u16 = margin;
    let mut col_top_y: u16 = 0;   // y where columns start (top-aligned)
    let mut col_max_y: u16 = 0;   // max y reached across all columns

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
                col_top_y = y;
                col_max_y = y;
            }
            SlideElement::Column(idx) => {
                // Track the tallest column before switching
                if y > col_max_y {
                    col_max_y = y;
                }
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
                // Reset y to column top so all columns are top-aligned
                y = col_top_y;
            }
            SlideElement::ResetLayout => {
                // Advance y past the tallest column
                if y > col_max_y {
                    col_max_y = y;
                }
                y = col_max_y;
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
                let align = if *level == 1 && !jump_to_middle {
                    TextAlign::Center
                } else {
                    TextAlign::Left
                };
                let bounds = Rect::new(x_start, y, w, 1);
                let mut label = Label::new(text)
                    .with_style(style)
                    .with_align(align);
                label.set_bounds(bounds);
                deferred_widgets.push(Box::new(label));
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
                let line_count = rust_pixel::ui::text_util::wrap_text(text, w).len() as u16;
                let h = line_count.min(content_height.saturating_sub(y));
                let mut label = Label::new(text)
                    .with_style(style)
                    .with_wrap(true);
                label.set_bounds(Rect::new(x_start, y, w, h));
                deferred_widgets.push(Box::new(label));
                y += line_count;
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

                        // Render highlighted spans (unicode-safe truncation)
                        for span in &hl_line.spans {
                            if cx >= x_start + w {
                                break;
                            }
                            let remaining = (x_start + w - cx) as usize;
                            let text = rust_pixel::ui::text_util::truncate_to_width(&span.text, remaining);
                            let text_w = rust_pixel::ui::text_util::display_width(&text);
                            buf.set_string(cx, y, &text, span.style);
                            cx += text_w as u16;
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
                        let truncated = rust_pixel::ui::text_util::truncate_to_width(line, w as usize);
                        buf.set_string(x_start, y, &truncated, plain_style);
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

                let list_items: Vec<PresentListItem> = items.iter().map(|item| {
                    PresentListItem::new(&item.text)
                        .with_depth(item.depth)
                        .with_ordered(item.ordered, item.index)
                }).collect();
                let item_count = list_items.len() as u16;
                let h = item_count.min(content_height.saturating_sub(y));
                let mut pl = PresentList::new().with_items(list_items);
                pl.set_bounds(Rect::new(x_start, y, w, h));
                deferred_widgets.push(Box::new(pl));
                y += item_count;
                y += 1; // spacing after list
            }
            SlideElement::Table { headers, rows, alignments } => {
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };

                let col_aligns: Vec<ColumnAlign> = alignments.iter().map(|a| match a {
                    ColumnAlignment::Left | ColumnAlignment::None => ColumnAlign::Left,
                    ColumnAlignment::Center => ColumnAlign::Center,
                    ColumnAlignment::Right => ColumnAlign::Right,
                }).collect();
                let table_h = 2 + rows.len() as u16; // header + separator + rows
                let h = table_h.min(content_height.saturating_sub(y));
                let mut pt = PresentTable::new()
                    .with_headers(headers.clone())
                    .with_rows(rows.clone())
                    .with_alignments(col_aligns);
                pt.set_bounds(Rect::new(x_start, y, w, h));
                deferred_widgets.push(Box::new(pt));
                y += table_h;
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

                // Draw emoji bullet on canvas (shared style with PresentList)
                buf.set_string(x_start, y, DEFAULT_MARKERS[0], default_marker_style());
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
                deferred_widgets.push(Box::new(label));
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
            SlideElement::Spacer(n) => {
                y += n;
            }
        }
    }

    // Add deferred widgets as Panel children (rendered on top of canvas)
    for widget in deferred_widgets {
        panel.add_child(widget);
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
                let lines = rust_pixel::ui::text_util::wrap_text(text, width);
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
            SlideElement::Spacer(n) => h += n,
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
            .add_modifier(Modifier::BOLD),
        2 => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        3 => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        _ => Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    }
}

