// RustPixel UI Framework - Slider Component
// copyright zipxing@hotmail.com 2022～2025

//! Slider component - character-cell slider for value selection.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Style, Color};
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id,
};
use crate::impl_widget_base;

/// Slider widget: a horizontal slider for selecting a value in a range.
pub struct Slider {
    base: BaseWidget,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    track_style: Style,
    thumb_style: Style,
    on_change: Option<Box<dyn Fn(f32) + 'static>>,
}

impl Slider {
    pub fn new(min: f32, max: f32) -> Self {
        let id = next_widget_id();
        Self {
            base: BaseWidget::new(id),
            value: min,
            min,
            max,
            step: (max - min) / 100.0,
            track_style: Style::default().fg(Color::Gray).bg(Color::Black),
            thumb_style: Style::default().fg(Color::White).bg(Color::Blue),
            on_change: None,
        }
    }

    pub fn with_value(mut self, value: f32) -> Self {
        self.value = value.clamp(self.min, self.max);
        self
    }

    pub fn with_step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn with_track_style(mut self, style: Style) -> Self {
        self.track_style = style;
        self
    }

    pub fn with_thumb_style(mut self, style: Style) -> Self {
        self.thumb_style = style;
        self
    }

    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(f32) + 'static,
    {
        self.on_change = Some(Box::new(callback));
        self
    }

    pub fn set_value(&mut self, value: f32) {
        let clamped = value.clamp(self.min, self.max);
        if (self.value - clamped).abs() > 0.001 {
            self.value = clamped;
            self.mark_dirty();
            if let Some(ref callback) = self.on_change {
                callback(self.value);
            }
        }
    }

    pub fn get_value(&self) -> f32 {
        self.value
    }

    fn value_to_position(&self, width: u16) -> u16 {
        if self.max <= self.min {
            return 0;
        }
        let normalized = (self.value - self.min) / (self.max - self.min);
        ((width as f32 - 1.0) * normalized) as u16
    }

    fn position_to_value(&self, pos: u16, width: u16) -> f32 {
        if width <= 1 {
            return self.min;
        }
        let normalized = pos as f32 / (width as f32 - 1.0);
        self.min + normalized * (self.max - self.min)
    }
}

impl Widget for Slider {
    impl_widget_base!(Slider, base);

    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible { return Ok(()); }
        let b = self.bounds();
        if b.width == 0 || b.height == 0 { return Ok(()); }

        // Check if position is within buffer bounds
        let buffer_area = *buffer.area();
        if b.y >= buffer_area.y + buffer_area.height || b.x >= buffer_area.x + buffer_area.width {
            return Ok(());
        }

        // Clamp bounds to buffer area
        let max_x = (b.x + b.width).min(buffer_area.x + buffer_area.width);
        let max_y = (b.y + b.height).min(buffer_area.y + buffer_area.height);

        // Render track
        for x in b.x..max_x {
            for y in b.y..max_y {
                buffer.get_mut(x, y)
                    .set_symbol("─")
                    .set_style(self.track_style);
            }
        }

        // Render thumb
        let thumb_pos = b.x + self.value_to_position(b.width);
        if thumb_pos < max_x {
            for y in b.y..max_y {
                buffer.get_mut(thumb_pos, y)
                    .set_symbol("█")
                    .set_style(self.thumb_style);
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        if !self.state().visible { return Ok(false); }

        // Handle mouse click to set position
        if let UIEvent::Input(crate::event::Event::Mouse(mouse_event)) = event {
            if self.hit_test(mouse_event.column, mouse_event.row) {
                if let crate::event::MouseEventKind::Down(crate::event::MouseButton::Left) = mouse_event.kind {
                    let b = self.bounds();
                    let pos = mouse_event.column.saturating_sub(b.x);
                    let new_value = self.position_to_value(pos, b.width);
                    self.set_value(new_value);
                    return Ok(true);
                }
            }
        }

        // Handle arrow keys
        if let UIEvent::Input(crate::event::Event::Key(key)) = event {
            match key.code {
                crate::event::KeyCode::Left => {
                    self.set_value(self.value - self.step);
                    return Ok(true);
                }
                crate::event::KeyCode::Right => {
                    self.set_value(self.value + self.step);
                    return Ok(true);
                }
                _ => {}
            }
        }

        Ok(false)
    }

    fn preferred_size(&self, available: Rect) -> Rect {
        // Prefer a single row, full width
        Rect::new(available.x, available.y, available.width, 1)
    }
}

