// RustPixel UI Framework - Table Component
// copyright zipxing@hotmail.com 2022～2025

//! Table component for displaying multi-column data with selection support.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::Style;
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id
};
use crate::impl_widget_base;
use crate::event::{Event as InputEvent, KeyCode, MouseEventKind, MouseButton};
use unicode_width::UnicodeWidthStr;

/// Column text alignment
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnAlign {
    Left,
    Center,
    Right,
}

/// Column definition
#[derive(Debug, Clone)]
pub struct Column {
    pub title: String,
    pub width: u16,
    pub align: ColumnAlign,
}

impl Column {
    pub fn new(title: &str, width: u16) -> Self {
        Self {
            title: title.to_string(),
            width,
            align: ColumnAlign::Left,
        }
    }

    pub fn align(mut self, align: ColumnAlign) -> Self {
        self.align = align;
        self
    }
}

/// A single cell in a table row
#[derive(Debug, Clone)]
pub struct TableCell {
    pub text: String,
    pub style: Option<Style>,
}

impl TableCell {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            style: None,
        }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

/// A row of cells
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub enabled: bool,
}

impl TableRow {
    pub fn new(cells: Vec<TableCell>) -> Self {
        Self {
            cells,
            enabled: true,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Table widget for multi-column data display with row selection
pub struct Table {
    base: BaseWidget,
    columns: Vec<Column>,
    rows: Vec<TableRow>,
    selected_index: Option<usize>,
    scroll_offset: usize,
    show_header: bool,
    header_style: Style,
    selected_style: Style,
    on_selection_changed: Option<Box<dyn FnMut(Option<usize>) + Send>>,
    on_row_activated: Option<Box<dyn FnMut(usize) + Send>>,
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

impl Table {
    pub fn new() -> Self {
        Self {
            base: BaseWidget::new(next_widget_id()),
            columns: Vec::new(),
            rows: Vec::new(),
            selected_index: None,
            scroll_offset: 0,
            show_header: true,
            header_style: Style::default(),
            selected_style: Style::default(),
            on_selection_changed: None,
            on_row_activated: None,
        }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.base.style = style;
        self
    }

    pub fn with_columns(mut self, columns: Vec<Column>) -> Self {
        self.columns = columns;
        self
    }

    pub fn with_header(mut self, show: bool) -> Self {
        self.show_header = show;
        self
    }

    pub fn with_header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }

    pub fn with_selected_style(mut self, style: Style) -> Self {
        self.selected_style = style;
        self
    }

    pub fn on_selection_changed<F>(mut self, callback: F) -> Self
    where
        F: FnMut(Option<usize>) + Send + 'static,
    {
        self.on_selection_changed = Some(Box::new(callback));
        self
    }

    pub fn on_row_activated<F>(mut self, callback: F) -> Self
    where
        F: FnMut(usize) + Send + 'static,
    {
        self.on_row_activated = Some(Box::new(callback));
        self
    }

    // --- Mutation API ---

    pub fn set_rows(&mut self, rows: Vec<TableRow>) {
        self.rows = rows;
        // Clamp selection
        if let Some(idx) = self.selected_index {
            if idx >= self.rows.len() {
                self.selected_index = if self.rows.is_empty() { None } else { Some(self.rows.len() - 1) };
            }
        }
        self.mark_dirty();
    }

    pub fn add_row(&mut self, row: TableRow) {
        self.rows.push(row);
        self.mark_dirty();
    }

    pub fn clear_rows(&mut self) {
        self.rows.clear();
        self.selected_index = None;
        self.scroll_offset = 0;
        self.mark_dirty();
    }

    pub fn select(&mut self, index: Option<usize>) {
        if let Some(idx) = index {
            if idx < self.rows.len() {
                self.selected_index = Some(idx);
                self.scroll_to(idx);
            }
        } else {
            self.selected_index = None;
        }
        self.mark_dirty();
        self.notify_selection_changed();
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn rows(&self) -> &[TableRow] {
        &self.rows
    }

    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    // --- Internal ---

    fn header_offset(&self) -> u16 {
        if self.show_header { 1 } else { 0 }
    }

    fn data_rows_visible(&self) -> usize {
        let bounds = self.bounds();
        bounds.height.saturating_sub(self.header_offset()) as usize
    }

    fn scroll_to(&mut self, index: usize) {
        let visible = self.data_rows_visible();
        if visible == 0 {
            return;
        }
        if index < self.scroll_offset {
            self.scroll_offset = index;
        } else if index >= self.scroll_offset + visible {
            self.scroll_offset = index.saturating_sub(visible - 1);
        }
    }

    fn visible_range(&self) -> (usize, usize) {
        let visible = self.data_rows_visible();
        let start = self.scroll_offset;
        let end = (start + visible).min(self.rows.len());
        (start, end)
    }

    fn notify_selection_changed(&mut self) {
        if let Some(ref mut cb) = self.on_selection_changed {
            cb(self.selected_index);
        }
    }

    fn notify_row_activated(&mut self, index: usize) {
        if let Some(ref mut cb) = self.on_row_activated {
            cb(index);
        }
    }

    /// Render aligned text into buffer within a fixed-width column region
    fn render_cell_text(
        buffer: &mut Buffer,
        x: u16,
        y: u16,
        width: u16,
        text: &str,
        align: ColumnAlign,
        style: Style,
    ) {
        let ba = *buffer.area();
        if y < ba.y || y >= ba.y + ba.height {
            return;
        }

        let text_w = text.width();
        let col_w = width as usize;

        // Truncate if needed
        let display = if text_w > col_w {
            let mut truncated = String::new();
            let mut w = 0;
            for ch in text.chars() {
                let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                if w + cw > col_w {
                    break;
                }
                truncated.push(ch);
                w += cw;
            }
            truncated
        } else {
            text.to_string()
        };

        let display_w = display.width();
        let pad = col_w.saturating_sub(display_w);
        let (pad_left, _pad_right) = match align {
            ColumnAlign::Left => (0, pad),
            ColumnAlign::Right => (pad, 0),
            ColumnAlign::Center => (pad / 2, pad - pad / 2),
        };

        // Fill the entire column width with the style (background)
        for cx in 0..col_w {
            let abs_x = x + cx as u16;
            if abs_x >= ba.x && abs_x < ba.x + ba.width {
                buffer.get_mut(abs_x, y).set_symbol(" ").set_style(style);
            }
        }

        // Write the text at the aligned position
        let text_x = x + pad_left as u16;
        if text_x < ba.x + ba.width {
            buffer.set_string(text_x, y, &display, style);
        }
    }
}

impl Widget for Table {
    impl_widget_base!(Table, base);

    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible {
            return Ok(());
        }

        let bounds = self.bounds();
        if bounds.width == 0 || bounds.height == 0 {
            return Ok(());
        }

        let base_style = self.base.style;

        // Clear background
        let ba = *buffer.area();
        for y in bounds.y..bounds.y + bounds.height {
            for x in bounds.x..bounds.x + bounds.width {
                if x >= ba.x && x < ba.x + ba.width && y >= ba.y && y < ba.y + ba.height {
                    buffer.get_mut(x, y).set_symbol(" ").set_style(base_style);
                }
            }
        }

        // Render header
        if self.show_header {
            let mut col_x = bounds.x;
            for col in &self.columns {
                if col_x >= bounds.x + bounds.width {
                    break;
                }
                let w = col.width.min(bounds.x + bounds.width - col_x);
                Self::render_cell_text(
                    buffer, col_x, bounds.y, w,
                    &col.title, col.align, self.header_style,
                );
                col_x += col.width;
            }
        }

        // Render data rows
        let (start, end) = self.visible_range();
        let data_y0 = bounds.y + self.header_offset();

        for (display_idx, row_idx) in (start..end).enumerate() {
            let y = data_y0 + display_idx as u16;
            if y >= bounds.y + bounds.height {
                break;
            }

            let row = &self.rows[row_idx];
            let is_selected = self.selected_index == Some(row_idx);
            let row_base = if is_selected { self.selected_style } else { base_style };

            let mut col_x = bounds.x;
            for (ci, col) in self.columns.iter().enumerate() {
                if col_x >= bounds.x + bounds.width {
                    break;
                }
                let w = col.width.min(bounds.x + bounds.width - col_x);

                let (text, cell_style) = if let Some(cell) = row.cells.get(ci) {
                    let merged = if let Some(cs) = cell.style {
                        // Merge: cell fg/bg overrides row base, but keep row bg if cell has no bg
                        let mut s = row_base;
                        if cs.fg.is_some() {
                            s.fg = cs.fg;
                        }
                        if cs.bg.is_some() {
                            s.bg = cs.bg;
                        }
                        s
                    } else {
                        row_base
                    };
                    (cell.text.as_str(), merged)
                } else {
                    ("", row_base)
                };

                Self::render_cell_text(buffer, col_x, y, w, text, col.align, cell_style);
                col_x += col.width;
            }

            // Fill remaining width with row style
            let ba = *buffer.area();
            while col_x < bounds.x + bounds.width {
                if col_x >= ba.x && col_x < ba.x + ba.width && y >= ba.y && y < ba.y + ba.height {
                    buffer.get_mut(col_x, y).set_symbol(" ").set_style(row_base);
                }
                col_x += 1;
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        if !self.state().enabled {
            return Ok(false);
        }

        match event {
            UIEvent::Input(InputEvent::Key(key_event)) => {
                match key_event.code {
                    KeyCode::Up => {
                        if let Some(idx) = self.selected_index {
                            if idx > 0 {
                                let new = idx - 1;
                                self.selected_index = Some(new);
                                self.scroll_to(new);
                                self.mark_dirty();
                                self.notify_selection_changed();
                                return Ok(true);
                            }
                        } else if !self.rows.is_empty() {
                            self.selected_index = Some(0);
                            self.scroll_to(0);
                            self.mark_dirty();
                            self.notify_selection_changed();
                            return Ok(true);
                        }
                    }
                    KeyCode::Down => {
                        if let Some(idx) = self.selected_index {
                            if idx + 1 < self.rows.len() {
                                let new = idx + 1;
                                self.selected_index = Some(new);
                                self.scroll_to(new);
                                self.mark_dirty();
                                self.notify_selection_changed();
                                return Ok(true);
                            }
                        } else if !self.rows.is_empty() {
                            self.selected_index = Some(0);
                            self.scroll_to(0);
                            self.mark_dirty();
                            self.notify_selection_changed();
                            return Ok(true);
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = self.selected_index {
                            self.notify_row_activated(idx);
                            return Ok(true);
                        }
                    }
                    _ => {}
                }
            }
            UIEvent::Input(InputEvent::Mouse(mouse_event)) => {
                if self.hit_test(mouse_event.column, mouse_event.row) {
                    if let MouseEventKind::Down(MouseButton::Left) = mouse_event.kind {
                        let bounds = self.bounds();
                        let click_y = mouse_event.row.saturating_sub(bounds.y + self.header_offset());
                        let (start, _) = self.visible_range();
                        let row_idx = start + click_y as usize;

                        if row_idx < self.rows.len() && self.rows[row_idx].enabled {
                            self.selected_index = Some(row_idx);
                            self.mark_dirty();
                            self.notify_selection_changed();
                            return Ok(true);
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(false)
    }

    fn preferred_size(&self, available: Rect) -> Rect {
        let total_col_width: u16 = self.columns.iter().map(|c| c.width).sum();
        let width = total_col_width.min(available.width);
        let height = (self.rows.len() as u16 + self.header_offset()).min(available.height);
        Rect::new(available.x, available.y, width, height)
    }
}
