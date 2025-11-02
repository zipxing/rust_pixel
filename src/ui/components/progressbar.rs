// RustPixel UI Framework - ProgressBar Component
// copyright zipxing@hotmail.com 2022～2025

//! ProgressBar component - character-cell progress indicator.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Style, Color};
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id,
};
use crate::impl_widget_base;

/// ProgressBar widget: displays progress as a filled bar.
pub struct ProgressBar {
    base: BaseWidget,
    value: f32, // 0.0 to 1.0
    bar_style: Style,
    fill_style: Style,
    show_percentage: bool,
    fill_char: char,
    empty_char: char,
}

impl ProgressBar {
    pub fn new() -> Self {
        let id = next_widget_id();
        Self {
            base: BaseWidget::new(id),
            value: 0.0,
            bar_style: Style::default().fg(Color::Gray).bg(Color::Black),
            fill_style: Style::default().fg(Color::White).bg(Color::Green),
            show_percentage: true,
            fill_char: '█',
            empty_char: '░',
        }
    }

    pub fn with_value(mut self, value: f32) -> Self {
        self.value = value.clamp(0.0, 1.0);
        self
    }

    pub fn with_bar_style(mut self, style: Style) -> Self {
        self.bar_style = style;
        self
    }

    pub fn with_fill_style(mut self, style: Style) -> Self {
        self.fill_style = style;
        self
    }

    pub fn with_show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    pub fn with_chars(mut self, fill: char, empty: char) -> Self {
        self.fill_char = fill;
        self.empty_char = empty;
        self
    }

    pub fn set_value(&mut self, value: f32) {
        let clamped = value.clamp(0.0, 1.0);
        if (self.value - clamped).abs() > 0.001 {
            self.value = clamped;
            self.mark_dirty();
        }
    }

    pub fn get_value(&self) -> f32 {
        self.value
    }
}

impl Widget for ProgressBar {
    impl_widget_base!(ProgressBar, base);

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

        // Calculate fill width
        let fill_width = ((b.width as f32 * self.value) as u16).min(b.width);
        let fill_end = (b.x + fill_width).min(max_x);

        // Render filled portion
        for x in b.x..fill_end {
            for y in b.y..max_y {
                buffer.get_mut(x, y)
                    .set_symbol(&self.fill_char.to_string())
                    .set_style(self.fill_style);
            }
        }

        // Render empty portion
        for x in fill_end..max_x {
            for y in b.y..max_y {
                buffer.get_mut(x, y)
                    .set_symbol(&self.empty_char.to_string())
                    .set_style(self.bar_style);
            }
        }

        // Render percentage text if enabled
        if self.show_percentage && b.width >= 5 && b.y < buffer_area.y + buffer_area.height {
            let percentage = format!("{}%", (self.value * 100.0) as u8);
            let text_x = b.x + (b.width.saturating_sub(percentage.len() as u16)) / 2;
            let text_y = b.y + b.height / 2;
            
            // Only render if within bounds
            if text_y < buffer_area.y + buffer_area.height && text_x < buffer_area.x + buffer_area.width {
                let text_style = Style::default().fg(Color::Black).bg(Color::White);
                buffer.set_string(text_x, text_y, &percentage, text_style);
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, _event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        // ProgressBar doesn't handle events by default
        Ok(false)
    }

    fn preferred_size(&self, available: Rect) -> Rect {
        // Prefer a single row, full width
        Rect::new(available.x, available.y, available.width, 1)
    }
}

