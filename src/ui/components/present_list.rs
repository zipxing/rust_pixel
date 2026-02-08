// RustPixel UI Framework - PresentList Component
// copyright zipxing@hotmail.com 2022～2025

//! Lightweight read-only list widget for presentation/display purposes.
//!
//! Supports nested items with depth-based indentation, emoji bullets for
//! unordered lists, and numbered prefixes for ordered lists. No interaction
//! (no selection, scrolling, or keyboard handling).

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Color, Style};
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id,
};
use crate::impl_widget_base;

/// A single item in a PresentList.
#[derive(Debug, Clone)]
pub struct PresentListItem {
    pub text: String,
    pub depth: u8,
    pub ordered: bool,
    pub index: usize,
}

impl PresentListItem {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            depth: 0,
            ordered: false,
            index: 1,
        }
    }

    pub fn with_depth(mut self, depth: u8) -> Self {
        self.depth = depth;
        self
    }

    pub fn with_ordered(mut self, ordered: bool, index: usize) -> Self {
        self.ordered = ordered;
        self.index = index;
        self
    }
}

/// Emoji markers used for unordered list bullets at different depths.
pub const DEFAULT_MARKERS: [&str; 3] = ["🟢", "🔵", "🟡"];

/// Default marker style (half-scale emoji).
pub fn default_marker_style() -> Style {
    Style::default().fg(Color::Cyan).scale(0.5, 0.5)
}

/// Lightweight read-only list widget with emoji bullets and nested indentation.
pub struct PresentList {
    base: BaseWidget,
    items: Vec<PresentListItem>,
    prefix_style: Style,
    text_style: Style,
    marker_style: Style,
    markers: [String; 3],
}

impl PresentList {
    pub fn new() -> Self {
        Self {
            base: BaseWidget::new(next_widget_id()),
            items: Vec::new(),
            prefix_style: Style::default().fg(Color::Cyan),
            text_style: Style::default().fg(Color::White),
            marker_style: default_marker_style(),
            markers: DEFAULT_MARKERS.map(|s| s.to_string()),
        }
    }

    pub fn with_items(mut self, items: Vec<PresentListItem>) -> Self {
        self.items = items;
        self
    }

    pub fn with_prefix_style(mut self, style: Style) -> Self {
        self.prefix_style = style;
        self
    }

    pub fn with_text_style(mut self, style: Style) -> Self {
        self.text_style = style;
        self
    }

    pub fn with_marker_style(mut self, style: Style) -> Self {
        self.marker_style = style;
        self
    }

    pub fn with_markers(mut self, markers: [String; 3]) -> Self {
        self.markers = markers;
        self
    }

    pub fn set_items(&mut self, items: Vec<PresentListItem>) {
        self.items = items;
        self.mark_dirty();
    }

    pub fn items(&self) -> &[PresentListItem] {
        &self.items
    }

    fn render_item(&self, buf: &mut Buffer, x: u16, y: u16, item: &PresentListItem) {
        let indent_width = item.depth as u16 * 2;
        let indent = "  ".repeat(item.depth as usize);

        if item.ordered {
            let bullet = format!("{}{}. ", indent, item.index);
            let w = bullet.len() as u16;
            buf.set_string(x, y, &bullet, self.prefix_style);
            buf.set_string(x + w, y, &item.text, self.text_style);
        } else {
            let marker_idx = (item.depth as usize).min(self.markers.len() - 1);
            let marker = &self.markers[marker_idx];
            buf.set_string(x, y, &indent, self.prefix_style);
            buf.set_string(x + indent_width, y, marker, self.marker_style);
            // emoji(2 cells) + space(1 cell) = 3
            buf.set_string(x + indent_width + 3, y, &item.text, self.text_style);
        }
    }
}

impl Widget for PresentList {
    impl_widget_base!(PresentList, base);

    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible {
            return Ok(());
        }
        let bounds = self.bounds();
        if bounds.width == 0 || bounds.height == 0 {
            return Ok(());
        }

        let buffer_area = *buffer.area();
        if bounds.y >= buffer_area.y + buffer_area.height
            || bounds.x >= buffer_area.x + buffer_area.width
        {
            return Ok(());
        }

        for (i, item) in self.items.iter().enumerate() {
            let y = bounds.y + i as u16;
            if y >= bounds.y + bounds.height || y >= buffer_area.y + buffer_area.height {
                break;
            }
            self.render_item(buffer, bounds.x, y, item);
        }

        Ok(())
    }

    fn handle_event(&mut self, _event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        Ok(false)
    }

    fn preferred_size(&self, available: Rect) -> Rect {
        let height = (self.items.len() as u16).min(available.height);
        Rect::new(available.x, available.y, available.width, height)
    }
}
