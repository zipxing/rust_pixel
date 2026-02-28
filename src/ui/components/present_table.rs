// RustPixel UI Framework - PresentTable Component
// copyright zipxing@hotmail.com 2022～2025

//! Lightweight read-only table widget for presentation/display purposes.
//!
//! Renders a simple table with header row, separator line, and data rows.
//! Supports column alignment (left/center/right). No interaction (no selection,
//! scrolling, or keyboard handling).

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Color, Modifier, Style};
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id,
};
use crate::ui::components::table::ColumnAlign;
use crate::impl_widget_base;
use unicode_width::UnicodeWidthStr;

/// Lightweight read-only table widget with header, separator, and aligned columns.
pub struct PresentTable {
    base: BaseWidget,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    alignments: Vec<ColumnAlign>,
    header_style: Style,
    separator_style: Style,
    cell_style: Style,
}

impl Default for PresentTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PresentTable {
    pub fn new() -> Self {
        Self {
            base: BaseWidget::new(next_widget_id()),
            headers: Vec::new(),
            rows: Vec::new(),
            alignments: Vec::new(),
            header_style: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            separator_style: Style::default().fg(Color::DarkGray),
            cell_style: Style::default().fg(Color::White),
        }
    }

    pub fn with_headers(mut self, headers: Vec<String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.rows = rows;
        self
    }

    pub fn with_alignments(mut self, alignments: Vec<ColumnAlign>) -> Self {
        self.alignments = alignments;
        self
    }

    pub fn with_header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }

    pub fn with_separator_style(mut self, style: Style) -> Self {
        self.separator_style = style;
        self
    }

    pub fn with_cell_style(mut self, style: Style) -> Self {
        self.cell_style = style;
        self
    }

    pub fn set_data(
        &mut self,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        alignments: Vec<ColumnAlign>,
    ) {
        self.headers = headers;
        self.rows = rows;
        self.alignments = alignments;
        self.mark_dirty();
    }

    fn col_align(&self, idx: usize) -> ColumnAlign {
        self.alignments.get(idx).copied().unwrap_or(ColumnAlign::Left)
    }
}

/// Align and truncate text within a fixed column width.
fn align_text(text: &str, width: usize, align: ColumnAlign) -> String {
    let text_w = text.width();
    let truncated = if text_w > width.saturating_sub(1) {
        // Truncate by character to respect unicode width
        let mut s = String::new();
        let mut w = 0;
        for ch in text.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if w + cw >= width {
                break;
            }
            s.push(ch);
            w += cw;
        }
        s
    } else {
        text.to_string()
    };

    let trunc_w = truncated.width();
    let pad = width.saturating_sub(trunc_w);

    match align {
        ColumnAlign::Left => format!("{}{}", truncated, " ".repeat(pad)),
        ColumnAlign::Center => {
            let left = pad / 2;
            let right = pad - left;
            format!("{}{}{}", " ".repeat(left), truncated, " ".repeat(right))
        }
        ColumnAlign::Right => format!("{}{}", " ".repeat(pad), truncated),
    }
}

impl Widget for PresentTable {
    impl_widget_base!(PresentTable, base);

    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible {
            return Ok(());
        }
        let bounds = self.bounds();
        if bounds.width == 0 || bounds.height == 0 || self.headers.is_empty() {
            return Ok(());
        }

        let buffer_area = *buffer.area();
        if bounds.y >= buffer_area.y + buffer_area.height
            || bounds.x >= buffer_area.x + buffer_area.width
        {
            return Ok(());
        }

        let num_cols = self.headers.len();
        let col_width = (bounds.width as usize / num_cols).max(3);
        let mut y = bounds.y;

        // Header row
        let mut cx = bounds.x;
        for (i, header) in self.headers.iter().enumerate() {
            let text = align_text(header, col_width, self.col_align(i));
            buf_set_clipped(buffer, cx, y, &text, self.header_style, &buffer_area);
            cx += col_width as u16;
        }
        y += 1;

        if y >= bounds.y + bounds.height || y >= buffer_area.y + buffer_area.height {
            return Ok(());
        }

        // Separator
        let separator = "─".repeat(col_width.saturating_sub(1));
        cx = bounds.x;
        for _ in 0..num_cols {
            buf_set_clipped(buffer, cx, y, &separator, self.separator_style, &buffer_area);
            cx += col_width as u16;
        }
        y += 1;

        // Data rows
        for row in &self.rows {
            if y >= bounds.y + bounds.height || y >= buffer_area.y + buffer_area.height {
                break;
            }
            cx = bounds.x;
            for (i, cell) in row.iter().enumerate() {
                let text = align_text(cell, col_width, self.col_align(i));
                buf_set_clipped(buffer, cx, y, &text, self.cell_style, &buffer_area);
                cx += col_width as u16;
            }
            y += 1;
        }

        Ok(())
    }

    fn handle_event(&mut self, _event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        Ok(false)
    }

    fn preferred_size(&self, available: Rect) -> Rect {
        // header + separator + rows
        let height = if self.headers.is_empty() {
            0
        } else {
            (2 + self.rows.len() as u16).min(available.height)
        };
        Rect::new(available.x, available.y, available.width, height)
    }
}

/// Write string to buffer only if within buffer area.
fn buf_set_clipped(
    buffer: &mut Buffer,
    x: u16,
    y: u16,
    text: &str,
    style: Style,
    buffer_area: &Rect,
) {
    if y >= buffer_area.y && y < buffer_area.y + buffer_area.height
        && x < buffer_area.x + buffer_area.width
    {
        buffer.set_string(x, y, text, style);
    }
}
