// RustPixel UI Framework - Toast Component
// copyright zipxing@hotmail.com 2022～2025

//! Toast/Notification component - character-cell temporary notification.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Style, Color};
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id,
};
use crate::impl_widget_base;

/// Toast type for different notification styles
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastType {
    Info,
    Success,
    Warning,
    Error,
}

/// Toast widget: a temporary notification message.
pub struct Toast {
    base: BaseWidget,
    message: String,
    toast_type: ToastType,
    duration: f32,  // seconds
    elapsed: f32,
    auto_hide: bool,
}

impl Toast {
    pub fn new(message: &str) -> Self {
        let id = next_widget_id();
        Self {
            base: BaseWidget::new(id),
            message: message.to_string(),
            toast_type: ToastType::Info,
            duration: 3.0,
            elapsed: 0.0,
            auto_hide: true,
        }
    }

    pub fn with_type(mut self, toast_type: ToastType) -> Self {
        self.toast_type = toast_type;
        self
    }

    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    pub fn with_auto_hide(mut self, auto_hide: bool) -> Self {
        self.auto_hide = auto_hide;
        self
    }

    pub fn set_message(&mut self, message: &str) {
        self.message = message.to_string();
        self.elapsed = 0.0;
        self.base.state.visible = true;
        self.mark_dirty();
    }

    pub fn show(&mut self) {
        self.elapsed = 0.0;
        self.base.state.visible = true;
        self.mark_dirty();
    }

    pub fn hide(&mut self) {
        self.base.state.visible = false;
        self.mark_dirty();
    }

    fn get_style(&self) -> Style {
        match self.toast_type {
            ToastType::Info => Style::default().fg(Color::White).bg(Color::Blue),
            ToastType::Success => Style::default().fg(Color::White).bg(Color::Green),
            ToastType::Warning => Style::default().fg(Color::Black).bg(Color::Yellow),
            ToastType::Error => Style::default().fg(Color::White).bg(Color::Red),
        }
    }

    fn get_icon(&self) -> &str {
        match self.toast_type {
            ToastType::Info => "ℹ",
            ToastType::Success => "✓",
            ToastType::Warning => "⚠",
            ToastType::Error => "✗",
        }
    }
}

impl Widget for Toast {
    impl_widget_base!(Toast, base);

    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible { return Ok(()); }
        let b = self.bounds();
        if b.width == 0 || b.height == 0 { return Ok(()); }

        // Check if position is within buffer bounds
        let buffer_area = *buffer.area();
        if b.y >= buffer_area.y + buffer_area.height || b.x >= buffer_area.x + buffer_area.width {
            return Ok(());
        }

        let style = self.get_style();
        let icon = self.get_icon();
        
        // Fill background
        let max_x = (b.x + b.width).min(buffer_area.x + buffer_area.width);
        let max_y = (b.y + b.height).min(buffer_area.y + buffer_area.height);
        
        for y in b.y..max_y {
            for x in b.x..max_x {
                buffer.get_mut(x, y).set_symbol(" ").set_style(style);
            }
        }

        // Render icon and message
        if b.y < buffer_area.y + buffer_area.height && b.x < buffer_area.x + buffer_area.width {
            let text = format!("{} {}", icon, self.message);
            let max_len = b.width.min(buffer_area.width.saturating_sub(b.x - buffer_area.x)) as usize;
            let display_text = if text.len() > max_len {
                &text[..max_len]
            } else {
                &text
            };
            buffer.set_string(b.x, b.y, display_text, style);
        }

        Ok(())
    }

    fn handle_event(&mut self, event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        if !self.state().visible { return Ok(false); }

        // Click to dismiss
        if let UIEvent::Input(crate::event::Event::Mouse(mouse_event)) = event {
            if self.hit_test(mouse_event.column, mouse_event.row) {
                if let crate::event::MouseEventKind::Down(crate::event::MouseButton::Left) = mouse_event.kind {
                    self.hide();
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    fn preferred_size(&self, available: Rect) -> Rect {
        // Width based on message length + icon
        let width = (self.message.len() as u16 + 4).min(available.width);
        Rect::new(available.x, available.y, width, 1)
    }
}

