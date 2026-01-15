// RustPixel UI Framework - Event System
// copyright zipxing@hotmail.com 2022～2025

//! UI event system built on top of rust_pixel's input events.

use crate::event::{Event as InputEvent, KeyEvent, MouseEvent, MouseEventKind, MouseButton};
use crate::ui::WidgetId;
use std::collections::VecDeque;

/// UI-specific events extending the base input events
#[derive(Debug, Clone, PartialEq)]
pub enum UIEvent {
    /// Raw input event from rust_pixel
    Input(InputEvent),
    
    /// Widget-specific events
    Widget(WidgetEvent),
    
    /// Application events
    App(AppEvent),
}

/// Widget-specific event types
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetEvent {
    /// Widget gained focus
    FocusGained(WidgetId),
    
    /// Widget lost focus
    FocusLost(WidgetId),
    
    /// Mouse entered widget bounds
    MouseEnter(WidgetId),
    
    /// Mouse left widget bounds
    MouseLeave(WidgetId),
    
    /// Widget was clicked
    Click(WidgetId, u16, u16), // widget_id, x, y
    
    /// Widget value changed
    ValueChanged(WidgetId, WidgetValue),
    
    /// Custom widget event
    Custom(WidgetId, String, WidgetValue),
}

/// Application-level events
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// Request to quit the application
    Quit,
    
    /// Request to redraw
    Redraw,
    
    /// Timer event
    Timer(String, f32),
    
    /// Theme changed
    ThemeChanged(String),
}

/// Generic value type for widget events
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetValue {
    None,
    Bool(bool),
    Int(i32),
    Float(f32),
    String(String),
    Point(u16, u16),
}

impl From<InputEvent> for UIEvent {
    fn from(event: InputEvent) -> Self {
        UIEvent::Input(event)
    }
}

impl From<WidgetEvent> for UIEvent {
    fn from(event: WidgetEvent) -> Self {
        UIEvent::Widget(event)
    }
}

impl From<AppEvent> for UIEvent {
    fn from(event: AppEvent) -> Self {
        UIEvent::App(event)
    }
}

/// Event handler trait for widgets
pub trait EventHandler {
    fn handle_event(&mut self, event: &UIEvent) -> bool;
}

/// Event dispatcher for managing event flow
pub struct EventDispatcher {
    focused_widget: Option<WidgetId>,
    hovered_widget: Option<WidgetId>,
    event_queue: VecDeque<UIEvent>,
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self {
            focused_widget: None,
            hovered_widget: None,
            event_queue: VecDeque::new(),
        }
    }
}

impl EventDispatcher {
    pub fn new() -> Self {
        Default::default()
    }
    
    /// Set the currently focused widget
    pub fn set_focus(&mut self, widget_id: Option<WidgetId>) {
        if self.focused_widget != widget_id {
            if let Some(old_id) = self.focused_widget {
                self.emit_event(WidgetEvent::FocusLost(old_id).into());
            }
            
            self.focused_widget = widget_id;
            
            if let Some(new_id) = widget_id {
                self.emit_event(WidgetEvent::FocusGained(new_id).into());
            }
        }
    }
    
    /// Get the currently focused widget
    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.focused_widget
    }
    
    /// Set the currently hovered widget
    pub fn set_hover(&mut self, widget_id: Option<WidgetId>) {
        if self.hovered_widget != widget_id {
            if let Some(old_id) = self.hovered_widget {
                self.emit_event(WidgetEvent::MouseLeave(old_id).into());
            }
            
            self.hovered_widget = widget_id;
            
            if let Some(new_id) = widget_id {
                self.emit_event(WidgetEvent::MouseEnter(new_id).into());
            }
        }
    }
    
    /// Get the currently hovered widget
    pub fn hovered_widget(&self) -> Option<WidgetId> {
        self.hovered_widget
    }
    
    /// Emit an event
    pub fn emit_event(&mut self, event: UIEvent) {
        self.event_queue.push_back(event);
    }
    
    /// Process input event and convert to UI events
    pub fn process_input(&mut self, input_event: InputEvent) {
        // Convert input events to UI events
        match &input_event {
            InputEvent::Mouse(mouse_event) => {
                self.process_mouse_event(mouse_event);
            }
            InputEvent::Key(key_event) => {
                self.process_key_event(key_event);
            }
        }
        
        // Always emit the raw input event
        self.emit_event(UIEvent::Input(input_event));
    }
    
    fn process_mouse_event(&mut self, mouse_event: &MouseEvent) {
        match mouse_event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Click event will be generated when we know which widget was clicked
                if let Some(widget_id) = self.hovered_widget {
                    self.emit_event(WidgetEvent::Click(widget_id, mouse_event.column, mouse_event.row).into());
                }
            }
            MouseEventKind::Moved => {
                // Hover tracking will be handled by the UI system
            }
            _ => {}
        }
    }
    
    fn process_key_event(&mut self, _key_event: &KeyEvent) {
        // Key events are handled directly by focused widgets
    }
    
    /// Get next event from queue (O(1) operation)
    pub fn next_event(&mut self) -> Option<UIEvent> {
        self.event_queue.pop_front()
    }
    
    /// Get all events and clear queue
    pub fn drain_events(&mut self) -> Vec<UIEvent> {
        self.event_queue.drain(..).collect()
    }
    
    /// Check if there are pending events
    pub fn has_events(&self) -> bool {
        !self.event_queue.is_empty()
    }
    
    /// Clear all events
    pub fn clear_events(&mut self) {
        self.event_queue.clear();
    }
}

/// Event callback types
pub type ClickCallback = Box<dyn FnMut(&mut dyn std::any::Any) + Send>;
pub type ValueChangedCallback = Box<dyn FnMut(&mut dyn std::any::Any, WidgetValue) + Send>;
pub type GenericCallback = Box<dyn FnMut(&mut dyn std::any::Any, &UIEvent) + Send>;

/// Helper macro for creating event handlers
#[macro_export]
macro_rules! ui_event_handler {
    ($widget:expr, $event:pat => $body:expr) => {
        |event: &UIEvent| {
            match event {
                $event => {
                    $body;
                    true
                }
                _ => false
            }
        }
    };
}