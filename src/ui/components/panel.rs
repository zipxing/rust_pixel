// RustPixel UI Framework - Panel Component
// copyright zipxing@hotmail.com 2022～2025

//! Panel component - a container widget for organizing other widgets.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Color, Style};
use crate::util::Rect;
use crate::ui::{
    Widget, Container, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    Layout, LinearLayout, LayoutConstraints,
    next_widget_id
};


/// Panel border style
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorderStyle {
    None,
    Single,
    Double,
    Rounded,
}

/// Panel component for containing other widgets
pub struct Panel {
    base: BaseWidget,
    children: Vec<Box<dyn Widget>>,
    layout: Box<dyn Layout>,
    layout_constraints: Vec<LayoutConstraints>,
    border_style: BorderStyle,
    title: Option<String>,
    /// Canvas buffer for direct character drawing (always available)
    canvas: Buffer,
}

impl Panel {
    pub fn new() -> Self {
        Self {
            base: BaseWidget::new(next_widget_id()),
            children: Vec::new(),
            layout: Box::new(LinearLayout::vertical()),
            layout_constraints: Vec::new(),
            border_style: BorderStyle::None,
            title: None,
            canvas: Buffer::empty(Rect::new(0, 0, 0, 0)),
        }
    }
    
    pub fn with_bounds(mut self, bounds: Rect) -> Self {
        self.base.bounds = bounds;
        // Auto-size canvas to bounds (render_canvas clips to content_area)
        self.canvas = Buffer::empty(Rect::new(0, 0, bounds.width, bounds.height));
        self
    }
    
    pub fn with_style(mut self, style: Style) -> Self {
        self.base.style = style;
        self
    }
    
    pub fn with_layout(mut self, layout: Box<dyn Layout>) -> Self {
        self.layout = layout;
        self
    }
    
    pub fn with_border(mut self, border_style: BorderStyle) -> Self {
        self.border_style = border_style;
        self
    }
    
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }
    
    pub fn set_title(&mut self, title: Option<String>) {
        if self.title != title {
            self.title = title;
            self.mark_dirty();
        }
    }
    
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
    
    pub fn set_border_style(&mut self, border_style: BorderStyle) {
        if self.border_style != border_style {
            self.border_style = border_style;
            self.mark_dirty();
        }
    }
    
    /// Add a child with specific layout constraints
    pub fn add_child_with_constraints(&mut self, child: Box<dyn Widget>, constraints: LayoutConstraints) {
        self.children.push(child);
        self.layout_constraints.push(constraints);
        self.mark_dirty();
    }
    
    /// Get the content area (bounds minus border and title)
    pub fn content_area(&self) -> Rect {
        let bounds = self.bounds();
        let mut content = bounds;

        // Account for border
        if self.border_style != BorderStyle::None {
            content.x += 1;
            content.y += 1;
            content.width = content.width.saturating_sub(2);
            content.height = content.height.saturating_sub(2);
        }

        // Account for title
        if self.title.is_some() {
            content.y += 1;
            content.height = content.height.saturating_sub(1);
        }

        content
    }

    // ========== Canvas methods for direct character drawing ==========

    /// Resize canvas to specified dimensions
    /// Canvas coordinates are relative to the content area
    pub fn enable_canvas(&mut self, width: u16, height: u16) {
        self.canvas = Buffer::empty(Rect::new(0, 0, width, height));
        self.mark_dirty();
    }

    /// Set a character at position (x, y) in the canvas
    /// Similar to Sprite's set_color_str but for single character
    pub fn set_char(&mut self, x: u16, y: u16, sym: &str, fg: Color, bg: Color) {
        let area = self.canvas.area();
        if x < area.width && y < area.height {
            let style = Style::default().fg(fg).bg(bg);
            self.canvas.get_mut(x, y).set_symbol(sym).set_style(style);
            self.mark_dirty();
        }
    }

    /// Set a string at position (x, y) in the canvas
    pub fn set_str(&mut self, x: u16, y: u16, s: &str, fg: Color, bg: Color) {
        let style = Style::default().fg(fg).bg(bg);
        self.canvas.set_string(x, y, s, style);
        self.mark_dirty();
    }

    /// Clear the canvas
    pub fn clear_canvas(&mut self) {
        self.canvas.reset();
        self.mark_dirty();
    }

    /// Get canvas buffer for direct manipulation
    pub fn canvas_mut(&mut self) -> &mut Buffer {
        &mut self.canvas
    }

    // ========== Sprite-compatible convenience methods ==========

    /// Set colored string at position - compatible with Sprite::set_color_str()
    pub fn set_color_str(&mut self, x: u16, y: u16, s: &str, fg: Color, bg: Color) {
        self.set_str(x, y, s, fg, bg);
    }

    /// Hide/show panel - compatible with Sprite::set_hidden()
    pub fn set_hidden(&mut self, hidden: bool) {
        self.set_visible(!hidden);
    }

    /// Check if panel is hidden - compatible with Sprite::is_hidden()
    pub fn is_hidden(&self) -> bool {
        !self.state().visible
    }

    /// Set panel position (keeps current size) - compatible with Sprite::set_pos()
    pub fn set_pos(&mut self, x: u16, y: u16) {
        let mut bounds = self.bounds();
        bounds.x = x;
        bounds.y = y;
        self.set_bounds(bounds);
    }
}

impl Widget for Panel {
    fn id(&self) -> WidgetId { self.base.id }
    fn bounds(&self) -> Rect { self.base.bounds }
    fn set_bounds(&mut self, bounds: Rect) {
        self.base.bounds = bounds;
        self.base.state.dirty = true;
    }
    fn state(&self) -> &WidgetState { &self.base.state }
    fn state_mut(&mut self) -> &mut WidgetState { &mut self.base.state }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn update(&mut self, dt: f32, ctx: &mut Context) -> UIResult<()> {
        for child in &mut self.children {
            child.update(dt, ctx)?;
        }
        Ok(())
    }

    fn layout_children(&mut self) {
        // Panel is a container, so delegate to Container's layout_recursive
        Container::layout_recursive(self);
    }

    fn render(&self, buffer: &mut Buffer, ctx: &Context) -> UIResult<()> {
        if !self.state().visible {
            return Ok(());
        }

        let bounds = self.bounds();
        if bounds.width == 0 || bounds.height == 0 {
            return Ok(());
        }

        // Use base style for now
        let style = self.base.style;

        // Clear background
        self.render_background(buffer, style)?;

        // Render border
        if self.border_style != BorderStyle::None {
            self.render_border(buffer, style)?;
        }

        // Render title
        if let Some(ref title) = self.title {
            self.render_title(buffer, title, style)?;
        }

        // Render canvas content
        if self.canvas.area().width > 0 && self.canvas.area().height > 0 {
            self.render_canvas(buffer, &self.canvas)?;
        }

        // Render children
        for child in &self.children {
            child.render(buffer, ctx)?;
        }

        Ok(())
    }
    
    fn handle_event(&mut self, event: &UIEvent, ctx: &mut Context) -> UIResult<bool> {
        if !self.state().enabled {
            return Ok(false);
        }
        
        // First let children handle the event
        for child in &mut self.children {
            if child.handle_event(event, ctx)? {
                return Ok(true);
            }
        }
        
        // Panel doesn't handle events by default
        Ok(false)
    }
    

    
    fn preferred_size(&self, available: Rect) -> Rect {
        // Panel prefers to use all available space
        available
    }
}

impl Container for Panel {
    fn add_child(&mut self, child: Box<dyn Widget>) {
        self.layout_constraints.push(LayoutConstraints::default());
        self.children.push(child);
        self.mark_dirty();
    }
    
    fn remove_child(&mut self, id: WidgetId) -> Option<Box<dyn Widget>> {
        if let Some(index) = self.children.iter().position(|child| child.id() == id) {
            self.layout_constraints.remove(index);
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
        let content_area = self.content_area();

        // Ensure we have constraints for all children
        while self.layout_constraints.len() < self.children.len() {
            self.layout_constraints.push(LayoutConstraints::default());
        }

        // Apply layout to position and size children
        self.layout.layout(&mut self.children, content_area, &self.layout_constraints);

        // Mark all children as dirty to trigger re-render
        for child in &mut self.children {
            child.mark_dirty();
        }
    }
}

impl Panel {
    /// Check if (x, y) is within buffer bounds
    #[inline]
    fn in_buffer(buf: &Buffer, x: u16, y: u16) -> bool {
        let a = buf.area();
        x >= a.x && x < a.x + a.width && y >= a.y && y < a.y + a.height
    }

    fn render_background(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        let ba = *buffer.area();

        // Clip to buffer area
        let x0 = bounds.x.max(ba.x);
        let y0 = bounds.y.max(ba.y);
        let x1 = (bounds.x + bounds.width).min(ba.x + ba.width);
        let y1 = (bounds.y + bounds.height).min(ba.y + ba.height);

        // Fully reset all cells first, then apply panel style.
        // cell.reset() clears fg/bg to Color::Reset, ensuring no stale styles bleed through.
        // set_style() alone won't clear colors when style has fg=None/bg=None.
        for y in y0..y1 {
            for x in x0..x1 {
                let cell = buffer.get_mut(x, y);
                cell.reset();
                cell.set_style(style);
            }
        }

        Ok(())
    }

    fn render_border(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();

        if bounds.width < 2 || bounds.height < 2 {
            return Ok(());
        }

        let (top_left, top_right, bottom_left, bottom_right, horizontal, vertical) = match self.border_style {
            BorderStyle::Single => ("┌", "┐", "└", "┘", "─", "│"),
            BorderStyle::Double => ("╔", "╗", "╚", "╝", "═", "║"),
            BorderStyle::Rounded => ("╭", "╮", "╰", "╯", "─", "│"),
            BorderStyle::None => return Ok(()),
        };

        let border_style = style;
        let bottom_y = bounds.y + bounds.height - 1;
        let right_x = bounds.x + bounds.width - 1;

        // Top and bottom borders
        for x in (bounds.x + 1)..right_x {
            if Self::in_buffer(buffer, x, bounds.y) {
                buffer.get_mut(x, bounds.y).set_symbol(horizontal).set_style(border_style);
            }
            if Self::in_buffer(buffer, x, bottom_y) {
                buffer.get_mut(x, bottom_y).set_symbol(horizontal).set_style(border_style);
            }
        }

        // Left and right borders
        for y in (bounds.y + 1)..bottom_y {
            if Self::in_buffer(buffer, bounds.x, y) {
                buffer.get_mut(bounds.x, y).set_symbol(vertical).set_style(border_style);
            }
            if Self::in_buffer(buffer, right_x, y) {
                buffer.get_mut(right_x, y).set_symbol(vertical).set_style(border_style);
            }
        }

        // Corners
        if Self::in_buffer(buffer, bounds.x, bounds.y) {
            buffer.get_mut(bounds.x, bounds.y).set_symbol(top_left).set_style(border_style);
        }
        if Self::in_buffer(buffer, right_x, bounds.y) {
            buffer.get_mut(right_x, bounds.y).set_symbol(top_right).set_style(border_style);
        }
        if Self::in_buffer(buffer, bounds.x, bottom_y) {
            buffer.get_mut(bounds.x, bottom_y).set_symbol(bottom_left).set_style(border_style);
        }
        if Self::in_buffer(buffer, right_x, bottom_y) {
            buffer.get_mut(right_x, bottom_y).set_symbol(bottom_right).set_style(border_style);
        }

        Ok(())
    }

    fn render_title(&self, buffer: &mut Buffer, title: &str, style: Style) -> UIResult<()> {
        let bounds = self.bounds();

        if title.is_empty() || bounds.width < 4 {
            return Ok(());
        }

        let title_y = bounds.y;
        if !Self::in_buffer(buffer, bounds.x, title_y) {
            return Ok(());
        }

        let available_width = if self.border_style != BorderStyle::None {
            bounds.width.saturating_sub(4)
        } else {
            bounds.width
        };

        let title_x = if self.border_style != BorderStyle::None {
            bounds.x + 2
        } else {
            bounds.x
        };

        // Truncate title if too long
        let display_title = if title.len() > available_width as usize {
            &title[..available_width as usize]
        } else {
            title
        };

        buffer.set_string(title_x, title_y, display_title, style);

        Ok(())
    }

    /// Render canvas content to the target buffer
    fn render_canvas(&self, buffer: &mut Buffer, canvas: &Buffer) -> UIResult<()> {
        let content = self.content_area();
        let canvas_area = canvas.area();

        for y in 0..canvas_area.height.min(content.height) {
            for x in 0..canvas_area.width.min(content.width) {
                let dst_x = content.x + x;
                let dst_y = content.y + y;
                if !Self::in_buffer(buffer, dst_x, dst_y) {
                    continue;
                }
                let src_cell = canvas.get(x, y);
                let has_content = !src_cell.symbol.is_empty() && src_cell.symbol != " ";
                let has_styled_bg = src_cell.bg != Color::Reset;
                // Copy cell if it has visible content or a non-default background
                if has_content || has_styled_bg {
                    let dst_cell = buffer.get_mut(dst_x, dst_y);
                    dst_cell.set_symbol(&src_cell.symbol).set_style(src_cell.style());
                }
            }
        }

        Ok(())
    }
}