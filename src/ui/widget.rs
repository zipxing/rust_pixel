// RustPixel UI Framework - Widget System
// copyright zipxing@hotmail.com 2022～2025

//! Core widget system defining the base traits and behaviors for all UI components.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::Style;
use crate::util::Rect;
use crate::ui::{UIEvent, UIResult};
use std::any::Any;

/// Unique identifier for widgets
pub type WidgetId = u32;

/// Widget state flags
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidgetState {
    pub visible: bool,
    pub enabled: bool,
    pub focused: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub dirty: bool,  // needs redraw
}

impl Default for WidgetState {
    fn default() -> Self {
        Self {
            visible: true,
            enabled: true,
            focused: false,
            hovered: false,
            pressed: false,
            dirty: true,
        }
    }
}

/// Core trait that all UI widgets must implement
pub trait Widget: Any {
    /// Get widget's unique identifier
    fn id(&self) -> WidgetId;
    
    /// Get widget's current bounds
    fn bounds(&self) -> Rect;
    
    /// Set widget's bounds
    fn set_bounds(&mut self, bounds: Rect);
    
    /// Get widget's current state
    fn state(&self) -> &WidgetState;
    
    /// Get mutable widget state
    fn state_mut(&mut self) -> &mut WidgetState;
    
    /// Render the widget to a buffer
    fn render(&self, buffer: &mut Buffer, ctx: &Context) -> UIResult<()>;
    
    /// Handle input events
    fn handle_event(&mut self, event: &UIEvent, ctx: &mut Context) -> UIResult<bool>;
    
    /// Update widget logic (called every frame)
    fn update(&mut self, dt: f32, ctx: &mut Context) -> UIResult<()>;
    
    /// Calculate preferred size based on content
    fn preferred_size(&self, available: Rect) -> Rect;
    
    /// Check if point is inside widget bounds
    fn hit_test(&self, x: u16, y: u16) -> bool {
        let bounds = self.bounds();
        x >= bounds.x && x < bounds.x + bounds.width &&
        y >= bounds.y && y < bounds.y + bounds.height
    }
    
    /// Set widget visibility
    fn set_visible(&mut self, visible: bool) {
        self.state_mut().visible = visible;
        self.state_mut().dirty = true;
    }
    
    /// Set widget enabled state
    fn set_enabled(&mut self, enabled: bool) {
        self.state_mut().enabled = enabled;
        self.state_mut().dirty = true;
    }
    
    /// Set widget focus
    fn set_focused(&mut self, focused: bool) {
        self.state_mut().focused = focused;
        self.state_mut().dirty = true;
    }
    
    /// Mark widget as dirty (needs redraw)
    fn mark_dirty(&mut self) {
        self.state_mut().dirty = true;
    }
    
    /// Clear dirty flag
    fn clear_dirty(&mut self) {
        self.state_mut().dirty = false;
    }
    
    /// Check if widget is dirty
    fn is_dirty(&self) -> bool {
        self.state().dirty
    }
    
    /// Get widget as Any for downcasting
    fn as_any(&self) -> &dyn Any;
    
    /// Get mutable widget as Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Layout children if this widget is a container
    /// Default implementation does nothing, containers should override this
    fn layout_children(&mut self) {
        // Default: no children to layout
    }
}

/// Container widget trait for widgets that can contain children
pub trait Container: Widget {
    /// Add a child widget
    fn add_child(&mut self, child: Box<dyn Widget>);
    
    /// Remove a child widget by ID
    fn remove_child(&mut self, id: WidgetId) -> Option<Box<dyn Widget>>;
    
    /// Get child by ID
    fn get_child(&self, id: WidgetId) -> Option<&dyn Widget>;
    
    /// Get mutable child by ID
    fn get_child_mut(&mut self, id: WidgetId) -> Option<&mut dyn Widget>;
    
    /// Get all children
    fn children(&self) -> &[Box<dyn Widget>];
    
    /// Get all children mutably
    fn children_mut(&mut self) -> &mut Vec<Box<dyn Widget>>;
    
    /// Find child at point
    fn child_at_point(&self, x: u16, y: u16) -> Option<WidgetId> {
        for child in self.children().iter().rev() { // reverse for top-to-bottom hit testing
            if child.state().visible && child.hit_test(x, y) {
                return Some(child.id());
            }
        }
        None
    }
    
    /// Layout children according to layout strategy
    fn layout(&mut self);

    /// Recursively layout this container and all child containers
    /// This provides a default implementation that calls layout() and then
    /// recursively calls layout_children() on all children
    fn layout_recursive(&mut self) {
        // First layout this container's children
        self.layout();

        // Then recursively layout any child containers
        for child in self.children_mut() {
            child.layout_children();
        }
    }
}

/// Base widget implementation with common functionality
pub struct BaseWidget {
    pub id: WidgetId,
    pub bounds: Rect,
    pub state: WidgetState,
    pub style: Style,
}

impl BaseWidget {
    pub fn new(id: WidgetId) -> Self {
        Self {
            id,
            bounds: Rect::default(),
            state: WidgetState::default(),
            style: Style::default(),
        }
    }
    
    pub fn with_bounds(mut self, bounds: Rect) -> Self {
        self.bounds = bounds;
        self
    }
    
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

/// Widget ID generator (thread-safe)
use std::sync::atomic::{AtomicU32, Ordering};

static WIDGET_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

pub fn next_widget_id() -> WidgetId {
    WIDGET_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Helper macro for widget boilerplate
#[macro_export]
macro_rules! impl_widget_base {
    ($widget:ty, $base_field:ident) => {
        fn id(&self) -> WidgetId {
            self.$base_field.id
        }
        
        fn bounds(&self) -> Rect {
            self.$base_field.bounds
        }
        
        fn set_bounds(&mut self, bounds: Rect) {
            self.$base_field.bounds = bounds;
            self.$base_field.state.dirty = true;
        }
        
        fn state(&self) -> &WidgetState {
            &self.$base_field.state
        }
        
        fn state_mut(&mut self) -> &mut WidgetState {
            &mut self.$base_field.state
        }
        
        fn update(&mut self, _dt: f32, _ctx: &mut Context) -> UIResult<()> {
            Ok(())
        }
        
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    };
}