// RustPixel UI Framework - Application Framework
// copyright zipxing@hotmail.com 2022～2025

//! Application framework for building UI applications.
//!
//! ## UIPage
//!
//! `UIPage` 代表一个独立的 UI 页面，内置 Buffer。多个 UIPage 可以通过
//! `BufferTransition` 实现页面转场效果。
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐
//! │   UIPage A  │     │   UIPage B  │
//! │  (widgets)  │     │  (widgets)  │
//! │  [buffer]   │     │  [buffer]   │
//! └──────┬──────┘     └──────┬──────┘
//!        │                   │
//!        └─────────┬─────────┘
//!                  ▼
//!        BufferTransition (可选)
//!                  ▼
//!            tui_buffer
//! ```

use crate::context::Context;
use crate::render::Buffer;

use crate::util::Rect;
use crate::ui::{
    Widget, WidgetId, UIEvent, UIResult, UIError, EventDispatcher, ThemeManager,
    AppEvent
};
use crate::event::Event as InputEvent;
use std::time::{Duration, Instant};

/// UI 页面结构 (支持多页面转场)
///
/// 每个 UIPage 包含:
/// - 一个 widget 树 (root_widget)
/// - 一个内置 Buffer (用于渲染和转场)
/// - 事件分发器和主题管理器
///
/// ## 使用方式
///
/// ```ignore
/// // 创建多个页面
/// let mut page_a = UIPage::new(80, 30);
/// page_a.set_root_widget(Box::new(slide1_panel));
///
/// let mut page_b = UIPage::new(80, 30);
/// page_b.set_root_widget(Box::new(slide2_panel));
///
/// // 渲染到各自的 buffer
/// page_a.render();
/// page_b.render();
///
/// // 使用转场混合
/// let transition = WipeTransition::left();
/// transition.transition(
///     page_a.buffer(),
///     page_b.buffer(),
///     tui_buffer,
///     0.5  // 50% 进度
/// );
/// ```
pub struct UIPage {
    root_widget: Option<Box<dyn Widget>>,
    event_dispatcher: EventDispatcher,
    theme_manager: ThemeManager,
    buffer: Buffer,
    running: bool,
    frame_time: Duration,
    last_frame: Instant,
}

/// Type alias for backward compatibility
#[allow(non_camel_case_types)]
pub type UiApp = UIPage;

/// Type alias for backward compatibility
pub type UIApp = UIPage;

impl UIPage {
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
        // Create a temporary context for updates (uses default GAME_CONFIG if not set)
        let mut ctx = Context::new();

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

        // Create rendering context (uses default GAME_CONFIG if not set)
        let ctx = Context::new();

        // Render root widget to internal buffer
        if let Some(ref root) = self.root_widget {
            root.render(&mut self.buffer, &ctx)?;
        }

        Ok(())
    }

    /// Render UI directly into the provided buffer (zero-copy)
    /// This is the recommended way to integrate UI rendering with the main game loop
    pub fn render_into(&mut self, target_buffer: &mut Buffer) -> UIResult<()> {
        // Clear target buffer using optimized method
        target_buffer.reset();

        // Create rendering context (uses default GAME_CONFIG if not set)
        let ctx = Context::new();

        // Render root widget directly to target buffer
        if let Some(ref root) = self.root_widget {
            root.render(target_buffer, &ctx)?;
        }

        Ok(())
    }

    pub fn layout(&mut self) {
        if let Some(ref mut root) = self.root_widget {
            // Set root widget bounds to buffer area
            let area = *self.buffer.area();
            root.set_bounds(area);

            // Recursively layout all containers using the new polymorphic mechanism
            root.layout_children();
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

    /// Get immutable reference to internal buffer
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Get mutable reference to internal buffer (for BufferTransition output)
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
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
        // Use the optimized clear_area method
        self.buffer.reset();
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

/// Builder for UI pages
pub struct UIPageBuilder {
    width: u16,
    height: u16,
    title: Option<String>,
    theme: Option<String>,
    frame_rate: Option<u32>,
}

/// Type alias for backward compatibility
pub type UIAppBuilder = UIPageBuilder;

impl UIPageBuilder {
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

    pub fn build(self) -> UIResult<UIPage> {
        let mut page = UIPage::new(self.width, self.height);

        if let Some(theme) = self.theme {
            page.set_theme(&theme)?;
        }

        if let Some(fps) = self.frame_rate {
            page.set_frame_rate(fps);
        }

        Ok(page)
    }
}
