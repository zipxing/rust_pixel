// RustPixel UI Framework - Application Framework
// copyright zipxing@hotmail.com 2022～2025

//! Application framework for building UI applications.

use crate::context::Context;
use crate::render::Buffer;

use crate::util::Rect;
use crate::ui::{
    Widget, Container, UIEvent, UIResult, UIError, EventDispatcher, ThemeManager,
    AppEvent, WidgetId, Panel
};
use crate::event::Event as InputEvent;
use std::time::{Duration, Instant};

/// Main UI application structure
pub struct UIApp {
    root_widget: Option<Box<dyn Widget>>,
    event_dispatcher: EventDispatcher,
    theme_manager: ThemeManager,
    buffer: Buffer,
    running: bool,
    frame_time: Duration,
    last_frame: Instant,
}

impl UIApp {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            root_widget: None,
            event_dispatcher: EventDispatcher::new(),
            theme_manager: ThemeManager::new(),
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
            running: false,
            frame_time: Duration::from_millis(16), // ~60 FPS
            last_frame: Instant::now(),
        }
    }
    
    pub fn set_root_widget(&mut self, widget: Box<dyn Widget>) {
        self.root_widget = Some(widget);
        self.layout();
    }
    
    pub fn set_theme(&mut self, theme_name: &str) -> UIResult<()> {
        self.theme_manager.set_theme(theme_name)
            .map_err(|e| UIError::ThemeError(e))?;
        
        // Emit theme changed event
        self.event_dispatcher.emit_event(AppEvent::ThemeChanged(theme_name.to_string()).into());
        
        Ok(())
    }
    
    pub fn set_frame_rate(&mut self, fps: u32) {
        self.frame_time = Duration::from_millis(1000 / fps as u64);
    }
    
    pub fn handle_input_event(&mut self, input_event: InputEvent) {
        // Update hover state based on mouse position
        if let InputEvent::Mouse(mouse_event) = &input_event {
            if let Some(ref root) = self.root_widget {
                let hovered_widget = self.find_widget_at_point(root.as_ref(), mouse_event.column, mouse_event.row);
                self.event_dispatcher.set_hover(hovered_widget);
            }
        }
        
        // Process input through event dispatcher
        self.event_dispatcher.process_input(input_event);
    }
    
    pub fn update(&mut self, dt: f32) -> UIResult<()> {
        // Create a temporary context for updates
        let mut ctx = Context::new("ui_app", ".");
        
        // Process events
        let events = self.event_dispatcher.drain_events();
        for event in events {
            match &event {
                UIEvent::App(AppEvent::Quit) => {
                    self.running = false;
                }
                _ => {
                    // Forward event to root widget
                    if let Some(ref mut root) = self.root_widget {
                        root.handle_event(&event, &mut ctx)?;
                    }
                }
            }
        }
        
        // Update root widget
        if let Some(ref mut root) = self.root_widget {
            root.update(dt, &mut ctx)?;
        }
        
        Ok(())
    }
    
    pub fn render(&mut self) -> UIResult<()> {
        // Clear buffer
        self.clear_buffer();
        
        // Create rendering context
        let ctx = Context::new("ui_app", ".");
        
        // Render root widget
        if let Some(ref root) = self.root_widget {
            root.render(&mut self.buffer, &ctx)?;
        }
        
        Ok(())
    }
    
    pub fn layout(&mut self) {
        if let Some(ref mut root) = self.root_widget {
            // Set root widget bounds to full buffer area
            root.set_bounds(*self.buffer.area());
            
            // Try to cast to Container and call layout
            if let Some(container) = root.as_any_mut().downcast_mut::<Panel>() {
                container.layout();
            }
        }
    }
    
    pub fn resize(&mut self, width: u16, height: u16) {
        self.buffer = Buffer::empty(Rect::new(0, 0, width, height));
        self.layout();
    }
    
    pub fn quit(&mut self) {
        self.event_dispatcher.emit_event(AppEvent::Quit.into());
    }
    
    pub fn is_running(&self) -> bool {
        self.running
    }
    
    pub fn start(&mut self) {
        self.running = true;
        self.last_frame = Instant::now();
    }
    
    pub fn should_render(&self) -> bool {
        self.last_frame.elapsed() >= self.frame_time
    }
    
    pub fn frame_complete(&mut self) {
        self.last_frame = Instant::now();
    }
    
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }
    
    pub fn event_dispatcher(&mut self) -> &mut EventDispatcher {
        &mut self.event_dispatcher
    }
    
    pub fn theme_manager(&self) -> &ThemeManager {
        &self.theme_manager
    }
    
    pub fn theme_manager_mut(&mut self) -> &mut ThemeManager {
        &mut self.theme_manager
    }
    
    /// Simple main loop for testing (in a real app, you'd integrate with rust_pixel's main loop)
    pub fn run_simple(&mut self) -> UIResult<()> {
        self.start();
        
        while self.running {
            let dt = self.last_frame.elapsed().as_secs_f32();
            
            // Update
            self.update(dt)?;
            
            // Render if needed
            if self.should_render() {
                self.render()?;
                self.frame_complete();
            }
            
            // Simple delay to prevent busy waiting
            std::thread::sleep(Duration::from_millis(1));
        }
        
        Ok(())
    }
    
    fn clear_buffer(&mut self) {
        let bounds = *self.buffer.area();
        for y in bounds.y..bounds.y + bounds.height {
            for x in bounds.x..bounds.x + bounds.width {
                self.buffer.get_mut(x, y).reset();
            }
        }
    }
    
    fn find_widget_at_point(&self, widget: &dyn Widget, x: u16, y: u16) -> Option<WidgetId> {
        if !widget.state().visible || !widget.hit_test(x, y) {
            return None;
        }
        
        // For now, just return the widget's ID
        // TODO: Add proper container support when trait object issues are resolved
        Some(widget.id())
    }
}

/// Builder for UI applications
pub struct UIAppBuilder {
    width: u16,
    height: u16,
    title: Option<String>,
    theme: Option<String>,
    frame_rate: Option<u32>,
}

impl UIAppBuilder {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            title: None,
            theme: None,
            frame_rate: None,
        }
    }
    
    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }
    
    pub fn with_theme(mut self, theme: &str) -> Self {
        self.theme = Some(theme.to_string());
        self
    }
    
    pub fn with_frame_rate(mut self, fps: u32) -> Self {
        self.frame_rate = Some(fps);
        self
    }
    
    pub fn build(self) -> UIResult<UIApp> {
        let mut app = UIApp::new(self.width, self.height);
        
        if let Some(theme) = self.theme {
            app.set_theme(&theme)?;
        }
        
        if let Some(fps) = self.frame_rate {
            app.set_frame_rate(fps);
        }
        
        Ok(app)
    }
}