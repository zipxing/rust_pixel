// RustPixel UI Framework - ToggleSwitch Component
// copyright zipxing@hotmail.com 2022～2025

//! ToggleSwitch component - character-cell toggle switch with label.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Style, Color};
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id,
};
use crate::impl_widget_base;

/// ToggleSwitch widget: a toggleable switch with optional label.
pub struct ToggleSwitch {
    base: BaseWidget,
    on: bool,
    label: String,
    off_style: Style,
    on_style: Style,
    on_change: Option<Box<dyn Fn(bool) + 'static>>,
}

impl ToggleSwitch {
    pub fn new(label: &str) -> Self {
        let id = next_widget_id();
        Self {
            base: BaseWidget::new(id),
            on: false,
            label: label.to_string(),
            off_style: Style::default().fg(Color::Gray).bg(Color::Black),
            on_style: Style::default().fg(Color::Green).bg(Color::Black),
            on_change: None,
        }
    }

    pub fn with_on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    pub fn with_off_style(mut self, style: Style) -> Self {
        self.off_style = style;
        self
    }

    pub fn with_on_style(mut self, style: Style) -> Self {
        self.on_style = style;
        self
    }

    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(bool) + 'static,
    {
        self.on_change = Some(Box::new(callback));
        self
    }

    pub fn set_on(&mut self, on: bool) {
        if self.on != on {
            self.on = on;
            self.mark_dirty();
            if let Some(ref callback) = self.on_change {
                callback(on);
            }
        }
    }

    pub fn is_on(&self) -> bool {
        self.on
    }

    pub fn toggle(&mut self) {
        self.set_on(!self.on);
    }
}

impl Widget for ToggleSwitch {
    impl_widget_base!(ToggleSwitch, base);

    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible { return Ok(()); }
        let b = self.bounds();
        if b.width == 0 || b.height == 0 { return Ok(()); }

        // Check if position is within buffer bounds
        let buffer_area = *buffer.area();
        if b.y >= buffer_area.y + buffer_area.height || b.x >= buffer_area.x + buffer_area.width {
            return Ok(());
        }

        // Render switch symbol: [OFF] or [ON ]
        let switch_symbol = if self.on { "[ON ]" } else { "[OFF]" };
        let switch_style = if self.on { self.on_style } else { self.off_style };
        
        // Ensure we don't exceed buffer width
        if b.x + 5 < buffer_area.x + buffer_area.width {
            buffer.set_string(b.x, b.y, switch_symbol, switch_style);
        }

        // Render label if there's space
        if b.width > 6 && !self.label.is_empty() {
            let label_x = b.x + 6;
            if label_x < buffer_area.x + buffer_area.width {
                let max_len = (b.width.saturating_sub(6)).min(buffer_area.width.saturating_sub(label_x - buffer_area.x)) as usize;
                let label_text = if self.label.len() > max_len {
                    &self.label[..max_len]
                } else {
                    &self.label
                };
                buffer.set_string(label_x, b.y, label_text, self.off_style);
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
        let width = (6 + self.label.len() as u16).min(available.width);
        Rect::new(available.x, available.y, width, 1)
    }
}

