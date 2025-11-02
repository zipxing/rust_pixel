// RustPixel UI Framework - Panel Component
// copyright zipxing@hotmail.com 2022～2025

//! Panel component - a container widget for organizing other widgets.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::Style;
use crate::util::Rect;
use crate::ui::{
    Widget, Container, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    Layout, LinearLayout, LayoutConstraints,
    next_widget_id
};
use crate::impl_widget_base;

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
        }
    }
    
    pub fn with_bounds(mut self, bounds: Rect) -> Self {
        self.base.bounds = bounds;
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
}

impl Widget for Panel {
    impl_widget_base!(Panel, base);
    
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
        
        // Apply layout
        self.layout.layout(&mut self.children, content_area, &self.layout_constraints);
        
        // Recursively layout child containers
        for child in &mut self.children {
            child.mark_dirty();
            
            // Try to downcast to known container types and call their layout
            if let Some(child_panel) = child.as_any_mut().downcast_mut::<Panel>() {
                child_panel.layout();
            } else if let Some(child_tabs) = child.as_any_mut().downcast_mut::<crate::ui::Tabs>() {
                child_tabs.layout();
            }
        }
    }
}

impl Panel {
    fn render_background(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        
        // Fill background
        for y in bounds.y..bounds.y + bounds.height {
            for x in bounds.x..bounds.x + bounds.width {
                let cell = buffer.get_mut(x, y);
                if cell.symbol == " " || cell.symbol.is_empty() {
                    cell.set_symbol(" ").set_style(style);
                }
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
        
        // Top and bottom borders
        for x in (bounds.x + 1)..(bounds.x + bounds.width - 1) {
            buffer.get_mut(x, bounds.y).set_symbol(horizontal).set_style(border_style);
            buffer.get_mut(x, bounds.y + bounds.height - 1).set_symbol(horizontal).set_style(border_style);
        }
        
        // Left and right borders
        for y in (bounds.y + 1)..(bounds.y + bounds.height - 1) {
            buffer.get_mut(bounds.x, y).set_symbol(vertical).set_style(border_style);
            buffer.get_mut(bounds.x + bounds.width - 1, y).set_symbol(vertical).set_style(border_style);
        }
        
        // Corners
        buffer.get_mut(bounds.x, bounds.y).set_symbol(top_left).set_style(border_style);
        buffer.get_mut(bounds.x + bounds.width - 1, bounds.y).set_symbol(top_right).set_style(border_style);
        buffer.get_mut(bounds.x, bounds.y + bounds.height - 1).set_symbol(bottom_left).set_style(border_style);
        buffer.get_mut(bounds.x + bounds.width - 1, bounds.y + bounds.height - 1).set_symbol(bottom_right).set_style(border_style);
        
        Ok(())
    }
    
    fn render_title(&self, buffer: &mut Buffer, title: &str, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        
        if title.is_empty() || bounds.width < 4 {
            return Ok(());
        }
        
        let title_y = if self.border_style != BorderStyle::None {
            bounds.y
        } else {
            bounds.y
        };
        
        let available_width = if self.border_style != BorderStyle::None {
            bounds.width.saturating_sub(4) // Account for border and padding
        } else {
            bounds.width
        };
        
        let title_x = if self.border_style != BorderStyle::None {
            bounds.x + 2 // Start after border and padding
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
}