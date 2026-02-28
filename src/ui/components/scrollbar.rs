// RustPixel UI Framework - ScrollBar Component
// copyright zipxing@hotmail.com 2022～2025

//! ScrollBar component for scrollable content.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::Style;
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id
};
use crate::impl_widget_base;
use crate::event::{Event as InputEvent, MouseEventKind, MouseButton};

/// Scrollbar orientation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollbarOrientation {
    Vertical,
    Horizontal,
}

/// ScrollBar component
pub struct ScrollBar {
    base: BaseWidget,
    orientation: ScrollbarOrientation,
    value: f32,      // Current scroll position (0.0 to 1.0)
    page_size: f32,  // Size of visible content relative to total (0.0 to 1.0)
    step: f32,       // Step size for arrow clicks
    dragging: bool,
    drag_offset: u16,
    on_value_changed: Option<Box<dyn FnMut(f32) + Send>>,
}

impl ScrollBar {
    pub fn new(orientation: ScrollbarOrientation) -> Self {
        Self {
            base: BaseWidget::new(next_widget_id()),
            orientation,
            value: 0.0,
            page_size: 0.1,
            step: 0.1,
            dragging: false,
            drag_offset: 0,
            on_value_changed: None,
        }
    }
    
    pub fn vertical() -> Self {
        Self::new(ScrollbarOrientation::Vertical)
    }
    
    pub fn horizontal() -> Self {
        Self::new(ScrollbarOrientation::Horizontal)
    }
    
    pub fn with_style(mut self, style: Style) -> Self {
        self.base.style = style;
        self
    }
    
    pub fn with_value(mut self, value: f32) -> Self {
        self.value = value.clamp(0.0, 1.0);
        self
    }
    
    pub fn with_page_size(mut self, page_size: f32) -> Self {
        self.page_size = page_size.clamp(0.0, 1.0);
        self
    }
    
    pub fn with_step(mut self, step: f32) -> Self {
        self.step = step.clamp(0.0, 1.0);
        self
    }
    
    pub fn on_value_changed<F>(mut self, callback: F) -> Self
    where
        F: FnMut(f32) + Send + 'static,
    {
        self.on_value_changed = Some(Box::new(callback));
        self
    }
    
    pub fn set_value(&mut self, value: f32) {
        let new_value = value.clamp(0.0, 1.0 - self.page_size);
        if (self.value - new_value).abs() > f32::EPSILON {
            self.value = new_value;
            self.mark_dirty();
            self.notify_value_changed();
        }
    }
    
    pub fn value(&self) -> f32 {
        self.value
    }
    
    pub fn set_page_size(&mut self, page_size: f32) {
        let new_page_size = page_size.clamp(0.0, 1.0);
        if (self.page_size - new_page_size).abs() > f32::EPSILON {
            self.page_size = new_page_size;
            self.value = self.value.min(1.0 - self.page_size);
            self.mark_dirty();
        }
    }
    
    pub fn page_size(&self) -> f32 {
        self.page_size
    }
    
    pub fn scroll_up(&mut self) {
        self.set_value(self.value - self.step);
    }
    
    pub fn scroll_down(&mut self) {
        self.set_value(self.value + self.step);
    }
    
    pub fn scroll_page_up(&mut self) {
        self.set_value(self.value - self.page_size);
    }
    
    pub fn scroll_page_down(&mut self) {
        self.set_value(self.value + self.page_size);
    }
    
    fn notify_value_changed(&mut self) {
        if let Some(ref mut callback) = self.on_value_changed {
            callback(self.value);
        }
    }
    
    fn get_thumb_bounds(&self) -> (u16, u16) {
        let bounds = self.bounds();
        
        match self.orientation {
            ScrollbarOrientation::Vertical => {
                let track_height = bounds.height.saturating_sub(2); // Account for arrows
                let thumb_size = ((self.page_size * track_height as f32) as u16).max(1);
                let thumb_pos = (self.value * (track_height - thumb_size) as f32) as u16;
                (bounds.y + 1 + thumb_pos, thumb_size)
            }
            ScrollbarOrientation::Horizontal => {
                let track_width = bounds.width.saturating_sub(2); // Account for arrows
                let thumb_size = ((self.page_size * track_width as f32) as u16).max(1);
                let thumb_pos = (self.value * (track_width - thumb_size) as f32) as u16;
                (bounds.x + 1 + thumb_pos, thumb_size)
            }
        }
    }
    
    fn position_to_value(&self, pos: u16) -> f32 {
        let bounds = self.bounds();
        
        match self.orientation {
            ScrollbarOrientation::Vertical => {
                let track_height = bounds.height.saturating_sub(2) as f32;
                let relative_pos = (pos.saturating_sub(bounds.y + 1)) as f32;
                (relative_pos / track_height).clamp(0.0, 1.0 - self.page_size)
            }
            ScrollbarOrientation::Horizontal => {
                let track_width = bounds.width.saturating_sub(2) as f32;
                let relative_pos = (pos.saturating_sub(bounds.x + 1)) as f32;
                (relative_pos / track_width).clamp(0.0, 1.0 - self.page_size)
            }
        }
    }
}

impl Widget for ScrollBar {
    impl_widget_base!(ScrollBar, base);
    
    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible {
            return Ok(());
        }
        
        let bounds = self.bounds();
        if bounds.width == 0 || bounds.height == 0 {
            return Ok(());
        }
        
        let style = self.base.style;
        
        match self.orientation {
            ScrollbarOrientation::Vertical => self.render_vertical(buffer, style)?,
            ScrollbarOrientation::Horizontal => self.render_horizontal(buffer, style)?,
        }
        
        Ok(())
    }
    
    fn handle_event(&mut self, event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        if !self.state().enabled {
            return Ok(false);
        }
        
        if let UIEvent::Input(InputEvent::Mouse(mouse_event)) = event {
            if self.hit_test(mouse_event.column, mouse_event.row) {
                match mouse_event.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        let bounds = self.bounds();
                        
                        match self.orientation {
                            ScrollbarOrientation::Vertical => {
                                if mouse_event.row == bounds.y {
                                    // Up arrow
                                    self.scroll_up();
                                } else if mouse_event.row == bounds.y + bounds.height - 1 {
                                    // Down arrow
                                    self.scroll_down();
                                } else {
                                    // Track or thumb
                                    let (thumb_pos, thumb_size) = self.get_thumb_bounds();
                                    if mouse_event.row >= thumb_pos && mouse_event.row < thumb_pos + thumb_size {
                                        // Start dragging thumb
                                        self.dragging = true;
                                        self.drag_offset = mouse_event.row - thumb_pos;
                                    } else {
                                        // Click on track
                                        let new_value = self.position_to_value(mouse_event.row);
                                        self.set_value(new_value);
                                    }
                                }
                            }
                            ScrollbarOrientation::Horizontal => {
                                if mouse_event.column == bounds.x {
                                    // Left arrow
                                    self.scroll_up();
                                } else if mouse_event.column == bounds.x + bounds.width - 1 {
                                    // Right arrow
                                    self.scroll_down();
                                } else {
                                    // Track or thumb
                                    let (thumb_pos, thumb_size) = self.get_thumb_bounds();
                                    if mouse_event.column >= thumb_pos && mouse_event.column < thumb_pos + thumb_size {
                                        // Start dragging thumb
                                        self.dragging = true;
                                        self.drag_offset = mouse_event.column - thumb_pos;
                                    } else {
                                        // Click on track
                                        let new_value = self.position_to_value(mouse_event.column);
                                        self.set_value(new_value);
                                    }
                                }
                            }
                        }
                        return Ok(true);
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        if self.dragging {
                            self.dragging = false;
                            return Ok(true);
                        }
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        if self.dragging {
                            let drag_pos = match self.orientation {
                                ScrollbarOrientation::Vertical => mouse_event.row.saturating_sub(self.drag_offset),
                                ScrollbarOrientation::Horizontal => mouse_event.column.saturating_sub(self.drag_offset),
                            };
                            let new_value = self.position_to_value(drag_pos);
                            self.set_value(new_value);
                            return Ok(true);
                        }
                    }
                    _ => {}
                }
            }
        }
        
        Ok(false)
    }
    
    fn preferred_size(&self, available: Rect) -> Rect {
        match self.orientation {
            ScrollbarOrientation::Vertical => {
                Rect::new(available.x, available.y, 1, available.height)
            }
            ScrollbarOrientation::Horizontal => {
                Rect::new(available.x, available.y, available.width, 1)
            }
        }
    }
}

impl ScrollBar {
    fn render_vertical(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        
        // Up arrow
        buffer.get_mut(bounds.x, bounds.y).set_symbol("▲").set_style(style);
        
        // Down arrow
        if bounds.height > 1 {
            buffer.get_mut(bounds.x, bounds.y + bounds.height - 1).set_symbol("▼").set_style(style);
        }
        
        // Track
        for y in (bounds.y + 1)..(bounds.y + bounds.height - 1) {
            buffer.get_mut(bounds.x, y).set_symbol("│").set_style(style);
        }
        
        // Thumb
        let (thumb_pos, thumb_size) = self.get_thumb_bounds();
        for y in thumb_pos..(thumb_pos + thumb_size) {
            if y < bounds.y + bounds.height - 1 {
                buffer.get_mut(bounds.x, y).set_symbol("█").set_style(style);
            }
        }
        
        Ok(())
    }
    
    fn render_horizontal(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        
        // Left arrow
        buffer.get_mut(bounds.x, bounds.y).set_symbol("◀").set_style(style);
        
        // Right arrow
        if bounds.width > 1 {
            buffer.get_mut(bounds.x + bounds.width - 1, bounds.y).set_symbol("▶").set_style(style);
        }
        
        // Track
        for x in (bounds.x + 1)..(bounds.x + bounds.width - 1) {
            buffer.get_mut(x, bounds.y).set_symbol("─").set_style(style);
        }
        
        // Thumb
        let (thumb_pos, thumb_size) = self.get_thumb_bounds();
        for x in thumb_pos..(thumb_pos + thumb_size) {
            if x < bounds.x + bounds.width - 1 {
                buffer.get_mut(x, bounds.y).set_symbol("█").set_style(style);
            }
        }
        
        Ok(())
    }
}