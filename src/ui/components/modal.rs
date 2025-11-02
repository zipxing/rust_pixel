// RustPixel UI Framework - Modal Component
// copyright zipxing@hotmail.com 2022～2025

//! Modal/Dialog component - character-cell modal dialog with backdrop and focus trap.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Style, Color};
use crate::util::Rect;
use crate::ui::{
    Widget, Container, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id,
};
use crate::impl_widget_base;

/// Modal widget: a centered dialog with backdrop, title, content, and action buttons.
pub struct Modal {
    base: BaseWidget,
    title: String,
    children: Vec<Box<dyn Widget>>, // Combined content and buttons
    num_content: usize, // Number of content widgets (rest are buttons)
    backdrop_style: Style,
    dialog_style: Style,
    title_style: Style,
    min_width: u16,
    min_height: u16,
    on_close: Option<Box<dyn Fn() + 'static>>,
}

impl Modal {
    pub fn new() -> Self {
        let id = next_widget_id();
        Self {
            base: BaseWidget::new(id),
            title: String::new(),
            children: Vec::new(),
            num_content: 0,
            backdrop_style: Style::default().bg(Color::Black).fg(Color::Gray),
            dialog_style: Style::default().bg(Color::White).fg(Color::Black),
            title_style: Style::default().bg(Color::Blue).fg(Color::White),
            min_width: 40,
            min_height: 10,
            on_close: None,
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = title.to_string();
        self
    }

    pub fn with_min_size(mut self, width: u16, height: u16) -> Self {
        self.min_width = width;
        self.min_height = height;
        self
    }

    pub fn with_backdrop_style(mut self, style: Style) -> Self {
        self.backdrop_style = style;
        self
    }

    pub fn with_dialog_style(mut self, style: Style) -> Self {
        self.dialog_style = style;
        self
    }

    pub fn with_title_style(mut self, style: Style) -> Self {
        self.title_style = style;
        self
    }

    pub fn on_close<F>(mut self, callback: F) -> Self
    where
        F: Fn() + 'static,
    {
        self.on_close = Some(Box::new(callback));
        self
    }

    pub fn add_content(&mut self, widget: Box<dyn Widget>) {
        // Insert content before buttons
        self.children.insert(self.num_content, widget);
        self.num_content += 1;
        self.mark_dirty();
    }

    pub fn add_button(&mut self, widget: Box<dyn Widget>) {
        // Buttons go after content
        self.children.push(widget);
        self.mark_dirty();
    }
    
    fn content_widgets(&self) -> &[Box<dyn Widget>] {
        &self.children[..self.num_content]
    }
    
    fn content_widgets_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children[..self.num_content]
    }
    
    fn button_widgets(&self) -> &[Box<dyn Widget>] {
        &self.children[self.num_content..]
    }
    
    fn button_widgets_mut(&mut self) -> &mut [Box<dyn Widget>] {
        &mut self.children[self.num_content..]
    }

    /// Calculate the dialog rect (centered within bounds)
    fn dialog_rect(&self) -> Rect {
        let b = self.bounds();
        let width = self.min_width.min(b.width.saturating_sub(4));
        let height = self.min_height.min(b.height.saturating_sub(4));
        let x = b.x + (b.width.saturating_sub(width)) / 2;
        let y = b.y + (b.height.saturating_sub(height)) / 2;
        Rect::new(x, y, width, height)
    }

    /// Title bar area (first row)
    fn title_area(&self) -> Rect {
        let d = self.dialog_rect();
        Rect::new(d.x, d.y, d.width, if d.height > 0 { 1 } else { 0 })
    }

    /// Content area (middle rows)
    fn content_area(&self) -> Rect {
        let d = self.dialog_rect();
        if d.height <= 2 {
            return Rect::new(d.x, d.y, 0, 0);
        }
        let num_buttons = self.children.len() - self.num_content;
        let button_rows = if num_buttons > 0 { 1 } else { 0 };
        let content_height = d.height.saturating_sub(1 + button_rows);
        Rect::new(d.x, d.y + 1, d.width, content_height)
    }

    /// Button area (last row)
    fn button_area(&self) -> Rect {
        let d = self.dialog_rect();
        let num_buttons = self.children.len() - self.num_content;
        if d.height <= 1 || num_buttons == 0 {
            return Rect::new(d.x, d.y, 0, 0);
        }
        Rect::new(d.x, d.y + d.height - 1, d.width, 1)
    }

    fn layout_children(&mut self) {
        // Layout content widgets vertically
        let content_rect = self.content_area();
        if content_rect.height > 0 && self.num_content > 0 {
            let per_child = content_rect.height / self.num_content.max(1) as u16;
            for i in 0..self.num_content {
                let y = content_rect.y + (i as u16 * per_child);
                let h = per_child.min(content_rect.height.saturating_sub(i as u16 * per_child));
                self.children[i].set_bounds(Rect::new(content_rect.x, y, content_rect.width, h));
            }
        }

        // Layout buttons horizontally
        let button_rect = self.button_area();
        let num_buttons = self.children.len() - self.num_content;
        if button_rect.width > 0 && num_buttons > 0 {
            let per_button = button_rect.width / num_buttons as u16;
            for i in 0..num_buttons {
                let button_idx = self.num_content + i;
                let x = button_rect.x + (i as u16 * per_button);
                let w = per_button.min(button_rect.width.saturating_sub(i as u16 * per_button));
                self.children[button_idx].set_bounds(Rect::new(x, button_rect.y, w, 1));
            }
        }
    }

    fn render_backdrop(&self, buffer: &mut Buffer) {
        let b = self.bounds();
        for y in b.y..b.y + b.height {
            for x in b.x..b.x + b.width {
                buffer.get_mut(x, y).set_symbol("░").set_style(self.backdrop_style);
            }
        }
    }

    fn render_dialog(&self, buffer: &mut Buffer) {
        let d = self.dialog_rect();
        
        // Fill dialog background
        for y in d.y..d.y + d.height {
            for x in d.x..d.x + d.width {
                buffer.get_mut(x, y).set_symbol(" ").set_style(self.dialog_style);
            }
        }

        // Draw border
        self.render_border(buffer, d);

        // Render title
        let title_rect = self.title_area();
        if title_rect.height > 0 {
            for x in title_rect.x..title_rect.x + title_rect.width {
                buffer.get_mut(x, title_rect.y).set_symbol(" ").set_style(self.title_style);
            }
            let title_text = if self.title.len() > title_rect.width as usize - 2 {
                &self.title[..title_rect.width as usize - 2]
            } else {
                &self.title
            };
            buffer.set_string(title_rect.x + 1, title_rect.y, title_text, self.title_style);
        }
    }

    fn render_border(&self, buffer: &mut Buffer, rect: Rect) {
        let style = self.dialog_style;
        // Top and bottom
        for x in rect.x..rect.x + rect.width {
            buffer.get_mut(x, rect.y).set_symbol("─").set_style(style);
            buffer.get_mut(x, rect.y + rect.height - 1).set_symbol("─").set_style(style);
        }
        // Left and right
        for y in rect.y..rect.y + rect.height {
            buffer.get_mut(rect.x, y).set_symbol("│").set_style(style);
            buffer.get_mut(rect.x + rect.width - 1, y).set_symbol("│").set_style(style);
        }
        // Corners
        buffer.get_mut(rect.x, rect.y).set_symbol("┌").set_style(style);
        buffer.get_mut(rect.x + rect.width - 1, rect.y).set_symbol("┐").set_style(style);
        buffer.get_mut(rect.x, rect.y + rect.height - 1).set_symbol("└").set_style(style);
        buffer.get_mut(rect.x + rect.width - 1, rect.y + rect.height - 1).set_symbol("┘").set_style(style);
    }
}

impl Widget for Modal {
    impl_widget_base!(Modal, base);

    fn render(&self, buffer: &mut Buffer, ctx: &Context) -> UIResult<()> {
        if !self.state().visible { return Ok(()); }
        let b = self.bounds();
        if b.width == 0 || b.height == 0 { return Ok(()); }

        // Render backdrop
        self.render_backdrop(buffer);

        // Render dialog box
        self.render_dialog(buffer);

        // Render all children (content + buttons)
        for child in &self.children {
            child.render(buffer, ctx)?;
        }

        Ok(())
    }

    fn handle_event(&mut self, event: &UIEvent, ctx: &mut Context) -> UIResult<bool> {
        if !self.state().visible { return Ok(false); }

        // Handle ESC to close
        if let UIEvent::Input(crate::event::Event::Key(key)) = event {
            if key.code == crate::event::KeyCode::Esc {
                if let Some(ref callback) = self.on_close {
                    callback();
                }
                return Ok(true);
            }
        }

        // Forward events to all children (buttons first, then content)
        for child in self.children.iter_mut().rev() {
            if child.handle_event(event, ctx)? {
                return Ok(true);
            }
        }

        // Modal consumes all events (focus trap)
        Ok(true)
    }

    fn preferred_size(&self, available: Rect) -> Rect {
        // Modal prefers to fill the entire available space (for backdrop)
        available
    }
}

impl Container for Modal {
    fn add_child(&mut self, child: Box<dyn Widget>) {
        // By default, add as content
        self.add_content(child);
    }
    
    fn remove_child(&mut self, id: WidgetId) -> Option<Box<dyn Widget>> {
        if let Some(index) = self.children.iter().position(|child| child.id() == id) {
            if index < self.num_content {
                self.num_content -= 1;
            }
            self.mark_dirty();
            Some(self.children.remove(index))
        } else {
            None
        }
    }
    
    fn get_child(&self, id: WidgetId) -> Option<&dyn Widget> {
        self.children.iter().find(|child| child.id() == id).map(|c| c.as_ref())
    }
    
    fn get_child_mut(&mut self, id: WidgetId) -> Option<&mut dyn Widget> {
        self.children.iter_mut().find(|child| child.id() == id).map(|c| c.as_mut())
    }
    
    fn children(&self) -> &[Box<dyn Widget>] {
        &self.children
    }
    
    fn children_mut(&mut self) -> &mut Vec<Box<dyn Widget>> {
        &mut self.children
    }
    
    fn layout(&mut self) {
        self.layout_children();
    }
}

