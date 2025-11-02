// RustPixel UI Framework - Checkbox Component
// copyright zipxing@hotmail.com 2022～2025

//! Checkbox component - character-cell checkbox with label.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Style, Color};
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id,
};
use crate::impl_widget_base;

/// Checkbox widget: a toggleable checkbox with optional label.
pub struct Checkbox {
    base: BaseWidget,
    checked: bool,
    label: String,
    style: Style,
    checked_style: Style,
    on_change: Option<Box<dyn Fn(bool) + 'static>>,
}

impl Checkbox {
    pub fn new(label: &str) -> Self {
        let id = next_widget_id();
        Self {
            base: BaseWidget::new(id),
            checked: false,
            label: label.to_string(),
            style: Style::default().fg(Color::White).bg(Color::Black),
            checked_style: Style::default().fg(Color::Green).bg(Color::Black),
            on_change: None,
        }
    }

    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_checked_style(mut self, style: Style) -> Self {
        self.checked_style = style;
        self
    }

    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(bool) + 'static,
    {
        self.on_change = Some(Box::new(callback));
        self
    }

    pub fn set_checked(&mut self, checked: bool) {
        if self.checked != checked {
            self.checked = checked;
            self.mark_dirty();
            if let Some(ref callback) = self.on_change {
                callback(checked);
            }
        }
    }

    pub fn is_checked(&self) -> bool {
        self.checked
    }

    pub fn toggle(&mut self) {
        self.set_checked(!self.checked);
    }
}

impl Widget for Checkbox {
    impl_widget_base!(Checkbox, base);

    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible { return Ok(()); }
        let b = self.bounds();
        if b.width == 0 || b.height == 0 { return Ok(()); }

        // Check if position is within buffer bounds
        let buffer_area = *buffer.area();
        if b.y >= buffer_area.y + buffer_area.height || b.x >= buffer_area.x + buffer_area.width {
            return Ok(());
        }

        // Render checkbox symbol
        let checkbox_symbol = if self.checked { "[✓]" } else { "[ ]" };
        let checkbox_style = if self.checked { self.checked_style } else { self.style };
        
        // Ensure we don't exceed buffer width
        if b.x + 3 < buffer_area.x + buffer_area.width {
            buffer.set_string(b.x, b.y, checkbox_symbol, checkbox_style);
        }

        // Render label if there's space
        if b.width > 4 && !self.label.is_empty() {
            let label_x = b.x + 4;
            if label_x < buffer_area.x + buffer_area.width {
                let max_len = (b.width.saturating_sub(4)).min(buffer_area.width.saturating_sub(label_x - buffer_area.x)) as usize;
                let label_text = if self.label.len() > max_len {
                    &self.label[..max_len]
                } else {
                    &self.label
                };
                buffer.set_string(label_x, b.y, label_text, self.style);
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        if !self.state().visible { return Ok(false); }

        // Handle mouse click
        if let UIEvent::Input(crate::event::Event::Mouse(mouse_event)) = event {
            if self.hit_test(mouse_event.column, mouse_event.row) {
                if let crate::event::MouseEventKind::Down(crate::event::MouseButton::Left) = mouse_event.kind {
                    self.toggle();
                    return Ok(true);
                }
            }
        }

        // Handle space key
        if let UIEvent::Input(crate::event::Event::Key(key)) = event {
            if key.code == crate::event::KeyCode::Char(' ') {
                self.toggle();
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn preferred_size(&self, available: Rect) -> Rect {
        // Prefer a single row, width based on label length
        let width = (4 + self.label.len() as u16).min(available.width);
        Rect::new(available.x, available.y, width, 1)
    }
}

