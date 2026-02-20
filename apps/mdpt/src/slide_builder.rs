use crate::chart::bar_chart::BarChart;
use crate::chart::line_chart::LineChart;
use crate::chart::mermaid::{parse_mermaid, render_mermaid};
use crate::chart::pie_chart::PieChart;
use crate::chart::{parse_chart_data, ChartRenderer};
use crate::highlight::HighlightedLine;
use crate::slide::{
    AlertType, AnimationType, ColumnAlignment, FrontMatter, LineRange, SlideContent, SlideElement,
};
use rust_pixel::render::style::{Color, Modifier, Style};
use rust_pixel::ui::*;
use rust_pixel::util::Rect;
use std::collections::HashMap;

/// Code block fill background (area outside code text)
const CODE_BG: Color = Color::Rgba(20, 24, 22, 255);

/// Code line background (syntect span bg, overrides theme default)
pub const CODE_LINE_BG: Color = Color::Rgba(20, 24, 22, 255);

/// Default code foreground for non-highlighted (dimmed) lines
pub const CODE_FG: Color = Color::Rgba(120, 125, 135, 255);

/// Default code foreground for highlighted lines
pub const CODE_FG_HL: Color = Color::Rgba(220, 225, 235, 255);

/// Deferred widgets to be added as Panel children after canvas rendering.
type DeferredWidgets = Vec<Box<dyn rust_pixel::ui::Widget>>;

/// Image placement info for sprite-based rendering (SSF/PIX).
#[derive(Debug, Clone)]
pub struct ImagePlacement {
    pub path: String,
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
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
) -> (UIPage, Vec<ImagePlacement>) {
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
    // Collect image placements for sprite-based rendering
    let mut image_placements: Vec<ImagePlacement> = Vec::new();

    // Render visible elements
    let mut y: u16 = 1; // Start after top margin
    let mut jump_to_middle = false;
    let mut in_column_layout = false;
    let mut column_widths: Vec<u32> = Vec::new();
    let mut current_col: usize = 0;
    let mut col_x: u16 = margin;
    let mut col_top_y: u16 = 0;   // y where columns start (top-aligned)
    let mut col_max_y: u16 = 0;   // max y reached across all columns

    log::info!("[mdpt] build_slide_page: boundary={}, margin={}, content={}x{}", boundary, margin, content_width, content_height);

    // First pass: if JumpToMiddle, calculate content height
    if boundary > 0 {
        if let Some(SlideElement::JumpToMiddle) = slide.elements.first() {
            log::info!("[mdpt] build_slide_page: JumpToMiddle detected");
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
        // Skip height check for Image elements (they render as sprite overlays)
        if y >= content_height && !matches!(elem, SlideElement::Image { .. }) {
            break;
        }
        log::info!("[mdpt] build_slide_page: rendering elem {} / {:?}", ei, std::mem::discriminant(elem));

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
                // Calculate x position for this column (with 1-char gap between columns)
                let total_weight: u32 = column_widths.iter().sum();
                let num_gaps = column_widths.len().saturating_sub(1) as u16;
                let available = content_width.saturating_sub(num_gaps);
                let mut x_offset: u16 = margin;
                for i in 0..current_col {
                    if i < column_widths.len() {
                        x_offset += (available as u32 * column_widths[i] / total_weight) as u16;
                        x_offset += 1; // 1-char gap between columns
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
                log::info!("[mdpt] Title: level={} text='{}' y={}", level, &text.chars().take(20).collect::<String>(), y);
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
                log::info!("[mdpt] Title: creating Label bounds=({},{},{},{})", x_start, y, w, 1);
                let mut label = Label::new(text)
                    .with_style(style)
                    .with_align(align);
                label.set_bounds(bounds);
                deferred_widgets.push(Box::new(label));
                log::info!("[mdpt] Title: Label done");
                y += 1;

                // Render author below title on first slide
                if *level == 1 && slide_idx == 0 && !front_matter.author.is_empty() {
                    let author_style = Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::ITALIC)
                        .scale(0.98, 0.98);
                    let mut author_label = Label::new(&front_matter.author)
                        .with_style(author_style)
                        .with_align(align);
                    author_label.set_bounds(Rect::new(x_start, y, w, 1));
                    deferred_widgets.push(Box::new(author_label));
                    y += 1;
                }

                // Add spacing after titles (level 3 also needs it to avoid
                // descender clipping from scale(1.1) on letters like "g", "y")
                if *level <= 3 {
                    y += 1;
                }
            }
            SlideElement::Paragraph { text } => {
                log::info!("[mdpt] Paragraph: text='{}' y={}", &text.chars().take(20).collect::<String>(), y);
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };

                let style = Style::default().fg(Color::White);
                log::info!("[mdpt] Paragraph: calling wrap_text w={}", w);
                let line_count = rust_pixel::ui::text_util::wrap_text(text, w).len() as u16;
                log::info!("[mdpt] Paragraph: wrap_text done, line_count={}", line_count);
                let h = line_count.min(content_height.saturating_sub(y));
                let mut label = Label::new(text)
                    .with_style(style)
                    .with_wrap(true);
                label.set_bounds(Rect::new(x_start, y, w, h));
                deferred_widgets.push(Box::new(label));
                log::info!("[mdpt] Paragraph: Label done");
                y += line_count;
                y += 1; // paragraph spacing
            }
            SlideElement::CodeBlock {
                language,
                code,
                line_numbers,
                no_background,
                highlight_groups,
            } => {
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };

                let has_bg = !no_background;
                let bg_style = if has_bg {
                    Style::default()
                        .fg(Color::Gray)
                        .bg(CODE_BG)
                } else {
                    Style::default().fg(Color::Gray)
                };

                // Compute active highlight group based on step
                let active_group_idx = if highlight_groups.len() > 1 {
                    let pauses_after = highlight_groups.len() - 1;
                    let consumed = boundary.saturating_sub(ei + 1).min(pauses_after);
                    consumed
                } else {
                    0
                };
                let active_ranges: Option<&Vec<LineRange>> =
                    if !highlight_groups.is_empty() {
                        highlight_groups.get(active_group_idx)
                    } else {
                        None
                    };

                // Language label
                if !language.is_empty() {
                    let lang_label = format!(" {} ", language);
                    let lang_style = if has_bg {
                        Style::default()
                            .fg(Color::Cyan)
                            .bg(CODE_BG)
                    } else {
                        Style::default().fg(Color::Cyan)
                    };
                    buf.set_string(x_start, y, &lang_label, lang_style);
                    if has_bg {
                        let fill = " ".repeat((w as usize).saturating_sub(lang_label.len()));
                        buf.set_string(
                            x_start + lang_label.len() as u16,
                            y,
                            &fill,
                            bg_style,
                        );
                    }
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

                        let line_num_1based = li + 1;
                        let is_highlighted =
                            is_line_in_ranges(line_num_1based, active_ranges);

                        // Fill line background
                        if has_bg {
                            let bg_fill = " ".repeat(w as usize);
                            buf.set_string(x_start, y, &bg_fill, bg_style);
                        }

                        let mut cx = x_start;

                        // Line numbers
                        if *line_numbers {
                            let num_str = format!(
                                "{:>width$} ",
                                line_num_1based,
                                width = (line_num_width - 2) as usize
                            );
                            let num_style = if has_bg {
                                Style::default()
                                    .fg(Color::DarkGray)
                                    .bg(CODE_BG)
                            } else {
                                Style::default().fg(Color::DarkGray)
                            };
                            buf.set_string(cx, y, &num_str, num_style);
                            cx += line_num_width;
                        }

                        // Render highlighted spans (unicode-safe truncation)
                        for span in &hl_line.spans {
                            if cx >= x_start + w {
                                break;
                            }
                            let remaining = (x_start + w - cx) as usize;
                            let text = rust_pixel::ui::text_util::truncate_to_width(
                                &span.text, remaining,
                            );
                            let text_w =
                                rust_pixel::ui::text_util::display_width(&text);
                            let style = if is_highlighted {
                                span.style
                            } else {
                                // Non-highlighted lines use CODE_FG
                                span.style.fg(CODE_FG)
                            };
                            buf.set_string(cx, y, &text, style);
                            cx += text_w as u16;
                        }

                        y += 1;
                    }
                } else {
                    // Fallback: render code without highlighting
                    let hl_style = if has_bg {
                        Style::default().fg(CODE_FG_HL).bg(CODE_BG)
                    } else {
                        Style::default().fg(CODE_FG_HL)
                    };
                    let dim_style = if has_bg {
                        Style::default().fg(CODE_FG).bg(CODE_BG)
                    } else {
                        Style::default().fg(CODE_FG)
                    };
                    for (li, line) in code.lines().enumerate() {
                        if y >= content_height {
                            break;
                        }
                        let is_highlighted =
                            is_line_in_ranges(li + 1, active_ranges);
                        if has_bg {
                            let bg_fill = " ".repeat(w as usize);
                            buf.set_string(x_start, y, &bg_fill, bg_style);
                        }
                        let truncated = rust_pixel::ui::text_util::truncate_to_width(
                            line,
                            w as usize,
                        );
                        let style = if is_highlighted {
                            hl_style
                        } else {
                            dim_style
                        };
                        buf.set_string(x_start, y, &truncated, style);
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
                log::info!("[mdpt] AnimatedText: text='{}' animation={:?} y={}", &text.chars().take(20).collect::<String>(), animation, y);
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };

                // Draw emoji bullet on canvas (shared style with PresentList)
                log::info!("[mdpt] AnimatedText: calling set_string for marker");
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
                            .with_spotlight(hl, 12, 0.35)
                    }
                    AnimationType::Wave => {
                        let s = Style::default().fg(Color::Rgba(255, 200, 80, 255)).bg(Color::Reset);
                        Label::new(text)
                            .with_style(s)
                            .with_wave(0.2, 8.0, 0.3)
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
            SlideElement::Image { path, alt: _, pos } => {
                // Use explicit position if provided, otherwise use document flow
                let (img_x, img_y) = if let Some((px, py)) = pos {
                    (*px, *py)
                } else {
                    let (x_start, _w) = if in_column_layout {
                        (col_x, col_width(&column_widths, current_col, content_width))
                    } else {
                        (margin, content_width)
                    };
                    (x_start, y)
                };
                image_placements.push(ImagePlacement {
                    path: path.clone(),
                    x: img_x,
                    y: img_y,
                    w: content_width,
                    h: content_height.saturating_sub(img_y),
                });
                // Don't advance y for sprite-based images (they overlay)
            }
            SlideElement::Spacer(n) => {
                y += n;
            }
            SlideElement::BlockQuote { text, alert_type } => {
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };

                // Alert header
                if let Some(at) = alert_type {
                    let (icon, label_text, color) = match at {
                        AlertType::Note => ("👉", "NOTE", Color::Blue),
                        AlertType::Tip => ("💡", "TIP", Color::Green),
                        AlertType::Important => ("❗", "IMPORTANT", Color::Magenta),
                        AlertType::Warning => ("⚠", "WARNING", Color::Yellow),
                        AlertType::Caution => ("❤️", "CAUTION", Color::Red),
                    };
                    let bar_style = Style::default().fg(color);
                    let icon_style = if matches!(at, AlertType::Caution) {
                        Style::default().fg(color).scale(0.75, 0.75)
                    } else {
                        Style::default().fg(color)
                    };
                    buf.set_string(x_start, y, "│ ", bar_style);
                    buf.set_string(x_start + 2, y, icon, icon_style);
                    // emoji = 2 cells + 1 space
                    let after_icon = x_start + 2 + 2 + 1;
                    buf.set_string(after_icon, y, label_text, bar_style);
                    y += 1;
                }

                // Quote text lines with vertical bar
                let bar_color = if let Some(at) = alert_type {
                    match at {
                        AlertType::Note => Color::Blue,
                        AlertType::Tip => Color::Green,
                        AlertType::Important => Color::Magenta,
                        AlertType::Warning => Color::Yellow,
                        AlertType::Caution => Color::Red,
                    }
                } else {
                    Color::DarkGray
                };
                let bar_style = Style::default().fg(bar_color);
                let text_style = Style::default().fg(Color::Gray);
                let quote_w = w.saturating_sub(2); // "│ " takes 2 cells
                let lines = rust_pixel::ui::text_util::wrap_text(text, quote_w);
                for line in &lines {
                    if y >= content_height {
                        break;
                    }
                    buf.set_string(x_start, y, "│ ", bar_style);
                    buf.set_string(x_start + 2, y, line, text_style);
                    y += 1;
                }
                y += 1; // spacing after block quote
            }
            SlideElement::Chart { chart_type, content } => {
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };

                let data = parse_chart_data(content);
                let chart_w = data.width.unwrap_or(w);
                // Default height: 20 for pie charts (need more vertical space), 15 for others
                let default_h = if chart_type == "piechart" { 20 } else { 15 };
                let chart_h = data.height.unwrap_or(default_h).min(content_height.saturating_sub(y));

                match chart_type.as_str() {
                    "linechart" => {
                        let chart = LineChart::new(data);
                        chart.render(buf, x_start, y, chart_w, chart_h);
                    }
                    "barchart" => {
                        let chart = BarChart::new(data);
                        chart.render(buf, x_start, y, chart_w, chart_h);
                    }
                    "piechart" => {
                        let chart = PieChart::new(data);
                        chart.render(buf, x_start, y, chart_w, chart_h);
                    }
                    _ => {}
                }
                y += chart_h + 1;
            }
            SlideElement::Mermaid { content } => {
                let (x_start, w) = if in_column_layout {
                    (col_x, col_width(&column_widths, current_col, content_width))
                } else {
                    (margin, content_width)
                };

                if let Some(graph) = parse_mermaid(content) {
                    // Use all remaining available space for mermaid chart
                    let chart_h = content_height.saturating_sub(y);
                    render_mermaid(&graph, buf, x_start, y, w, chart_h);
                    y += chart_h + 1;
                } else {
                    // Fallback: render as plain code block
                    let style = Style::default().fg(Color::Gray);
                    for line in content.lines() {
                        if y >= content_height {
                            break;
                        }
                        let truncated = rust_pixel::ui::text_util::truncate_to_width(line, w as usize);
                        buf.set_string(x_start, y, &truncated, style);
                        y += 1;
                    }
                    y += 1;
                }
            }
        }
    }

    // Add deferred widgets as Panel children (rendered on top of canvas)
    log::info!("[mdpt] build_slide_page: adding {} deferred widgets", deferred_widgets.len());
    for widget in deferred_widgets {
        panel.add_child(widget);
    }

    log::info!("[mdpt] build_slide_page: creating UIPage {}x{}", width, height);
    let mut page = UIPage::new(width, height);
    log::info!("[mdpt] build_slide_page: setting root widget");
    page.set_root_widget(Box::new(panel));
    log::info!("[mdpt] build_slide_page: calling page.start()");
    page.start();
    log::info!("[mdpt] build_slide_page: done");
    (page, image_placements)
}

/// Build a UIPage for the auto-generated cover slide.
///
/// Clean, minimal layout — large centered title with animation,
/// author, and subtle config info at the bottom.
pub fn build_cover_page(
    front_matter: &FrontMatter,
    width: u16,
    height: u16,
) -> UIPage {
    let margin = front_matter.margin;
    let content_width = width.saturating_sub(margin * 2);

    let mut panel = Panel::new()
        .with_bounds(Rect::new(0, 0, width, height))
        .with_border(BorderStyle::None)
        .with_layout(Box::new(FreeLayout));
    panel.enable_canvas(width, height);

    let buf = panel.canvas_mut();
    let mut deferred_widgets: Vec<Box<dyn Widget>> = Vec::new();

    let cx = margin; // content left x
    let mid_y = height / 2; // vertical center

    // ── Title (large, centered, static) ──
    let title_y = mid_y.saturating_sub(3);
    if !front_matter.title.is_empty() {
        let title_style = Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
            .scale(1.8, 1.8);
        let mut label = Label::new(&front_matter.title)
            .with_style(title_style)
            .with_align(TextAlign::Center);
        label.set_bounds(Rect::new(cx, title_y, content_width, 1));
        deferred_widgets.push(Box::new(label));
    }

    // ── Author ──
    if !front_matter.author.is_empty() {
        let author_y = title_y + 3;
        let author_style = Style::default()
            .fg(Color::Rgba(140, 200, 160, 255))
            .scale(0.95, 0.95)
            .add_modifier(Modifier::ITALIC);
        let mut label = Label::new(&front_matter.author)
            .with_style(author_style)
            .with_align(TextAlign::Center);
        label.set_bounds(Rect::new(cx, author_y, content_width, 1));
        deferred_widgets.push(Box::new(label));
    }

    // ── Thin separator ──
    let sep_y = height - 5;
    let sep_w = 30u16.min(content_width);
    let sep_x = cx + (content_width.saturating_sub(sep_w)) / 2;
    let sep_style = Style::default().fg(Color::Rgba(60, 65, 70, 255));
    buf.set_string(sep_x, sep_y, &"─".repeat(sep_w as usize), sep_style);

    // ── Config line (small, dim, single line) ──
    let info = format!(
        "{}  ·  {}  ·  {}",
        front_matter.theme, front_matter.transition, front_matter.code_theme
    );
    let info_style = Style::default().fg(Color::Rgba(90, 95, 100, 255)).scale(0.85, 0.85);
    let info_x = cx + (content_width.saturating_sub(info.len() as u16)) / 2;
    buf.set_string(info_x, sep_y + 1, &info, info_style);

    // ── Hint ──
    let hint = "Press Space to begin →";
    let hint_style = Style::default().fg(Color::Rgba(80, 85, 90, 255)).scale(0.85, 0.85);
    let hint_x = cx + (content_width.saturating_sub(hint.len() as u16)) / 2;
    buf.set_string(hint_x, sep_y + 3, hint, hint_style);

    // Add deferred widgets
    for widget in deferred_widgets {
        panel.add_child(widget);
    }

    let mut page = UIPage::new(width, height);
    page.set_root_widget(Box::new(panel));
    page.start();
    page
}

/// Calculate column width from weights (accounts for 1-char gap between columns).
fn col_width(weights: &[u32], col_idx: usize, total_width: u16) -> u16 {
    let total_weight: u32 = weights.iter().sum();
    if total_weight == 0 || col_idx >= weights.len() {
        return total_width;
    }
    let num_gaps = weights.len().saturating_sub(1) as u16;
    let available = total_width.saturating_sub(num_gaps);
    (available as u32 * weights[col_idx] / total_weight) as u16
}

/// Estimate content height for vertical centering.
fn estimate_content_height(elements: &[SlideElement], width: u16) -> u16 {
    let mut h: u16 = 0;
    for elem in elements {
        match elem {
            SlideElement::Title { level, .. } => {
                h += 1;
                if *level <= 3 {
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
            SlideElement::Image { .. } => {}, // sprite overlay, no vertical space
            SlideElement::AnimatedText { .. } => h += 1,
            SlideElement::Spacer(n) => h += n,
            SlideElement::BlockQuote { text, alert_type } => {
                if alert_type.is_some() {
                    h += 1; // alert header
                }
                let lines = rust_pixel::ui::text_util::wrap_text(text, width.saturating_sub(2));
                h += lines.len() as u16 + 1;
            }
            SlideElement::Chart { content, .. } => {
                let data = parse_chart_data(content);
                h += data.height.unwrap_or(15) + 1;
            }
            SlideElement::Mermaid { .. } => {
                h += 16; // default mermaid height + spacing
            }
            SlideElement::Pause | SlideElement::JumpToMiddle => {}
            SlideElement::ColumnLayout { .. } | SlideElement::Column(_) | SlideElement::ResetLayout => {}
        }
    }
    h
}

/// Check if a 1-indexed line number is within any of the given ranges.
/// Returns true if no ranges specified (show all highlighted).
fn is_line_in_ranges(line: usize, ranges: Option<&Vec<LineRange>>) -> bool {
    let ranges = match ranges {
        Some(r) if !r.is_empty() => r,
        _ => return true, // No highlight groups = all lines highlighted
    };
    for range in ranges {
        match range {
            LineRange::All => return true,
            LineRange::Single(n) => {
                if line == *n {
                    return true;
                }
            }
            LineRange::Range(start, end) => {
                if line >= *start && line <= *end {
                    return true;
                }
            }
        }
    }
    false
}

/// Style for heading levels.
/// Uses FIXED_SLOT so that scale doesn't push subsequent cells in the same row
/// (important for column layouts where left column headings share rows with right column content).
fn title_style(level: u8) -> Style {
    match level {
        1 => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD | Modifier::FIXED_SLOT)
            .scale(1.1, 1.1),
        2 => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::FIXED_SLOT)
            .scale(1.1, 1.1),
        3 => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD | Modifier::FIXED_SLOT)
            .scale(1.1, 1.1),
        _ => Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD | Modifier::FIXED_SLOT)
            .scale(1.1, 1.1),
    }
}

