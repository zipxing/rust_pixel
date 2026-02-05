// RustPixel UI Framework - AnimatedLabel Component
// copyright zipxing@hotmail.com 2022～2025

//! AnimatedLabel: a label with per-character spotlight animation.
//!
//! Each character is highlighted sequentially with a scale pulse,
//! driven entirely through Style (including per-cell scale).
//! Works in both TUI buffer and sprite rendering paths.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::Style;
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id,
};
use crate::ui::components::label::TextAlign;
use unicode_width::UnicodeWidthStr;

/// Label widget with sequential per-character scale animation.
pub struct AnimatedLabel {
    base: BaseWidget,
    text: String,
    align: TextAlign,
    /// Style for the currently highlighted character
    highlight_style: Style,
    /// Animation frames per character spotlight
    frames_per_char: usize,
    /// Scale pulse amplitude (e.g. 0.55 → scale range 1.0~1.55)
    scale_amplitude: f32,
    /// Current animation frame counter
    frame: usize,
}

impl AnimatedLabel {
    pub fn new(text: &str) -> Self {
        Self {
            base: BaseWidget::new(next_widget_id()),
            text: text.to_string(),
            align: TextAlign::Left,
            highlight_style: Style::default(),
            frames_per_char: 12,
            scale_amplitude: 0.55,
            frame: 0,
        }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.base.style = style;
        self
    }

    pub fn with_highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn with_frames_per_char(mut self, n: usize) -> Self {
        self.frames_per_char = n.max(1);
        self
    }

    pub fn with_scale_amplitude(mut self, amp: f32) -> Self {
        self.scale_amplitude = amp;
        self
    }

    pub fn set_text(&mut self, text: &str) {
        if self.text != text {
            self.text = text.to_string();
            self.mark_dirty();
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Widget for AnimatedLabel {
    fn id(&self) -> WidgetId { self.base.id }
    fn bounds(&self) -> Rect { self.base.bounds }
    fn set_bounds(&mut self, bounds: Rect) {
        self.base.bounds = bounds;
        self.base.state.dirty = true;
    }
    fn state(&self) -> &WidgetState { &self.base.state }
    fn state_mut(&mut self) -> &mut WidgetState { &mut self.base.state }

    fn update(&mut self, _dt: f32, _ctx: &mut Context) -> UIResult<()> {
        self.frame = self.frame.wrapping_add(1);
        self.mark_dirty();
        Ok(())
    }

    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible {
            return Ok(());
        }

        let bounds = self.bounds();
        if bounds.width == 0 || bounds.height == 0 {
            return Ok(());
        }

        let buffer_area = *buffer.area();
        if bounds.y >= buffer_area.y + buffer_area.height
            || bounds.x >= buffer_area.x + buffer_area.width
        {
            return Ok(());
        }

        let text_width = self.text.width() as u16;
        if text_width == 0 {
            return Ok(());
        }

        let start_x = match self.align {
            TextAlign::Left => bounds.x,
            TextAlign::Center => bounds.x + bounds.width.saturating_sub(text_width) / 2,
            TextAlign::Right => bounds.x + bounds.width.saturating_sub(text_width),
        };

        // Animation state
        let char_count = self.text.chars().count();
        let cycle_len = char_count * self.frames_per_char;
        let frame_in_cycle = self.frame % cycle_len;
        let active_idx = frame_in_cycle / self.frames_per_char;
        let progress =
            (frame_in_cycle % self.frames_per_char) as f32 / self.frames_per_char as f32;
        let active_scale =
            1.0 + self.scale_amplitude * (progress * std::f32::consts::PI).sin();

        // Render each character with its own style
        let base_style = self.base.style.scale_uniform(1.0);

        for (i, ch) in self.text.chars().enumerate() {
            let x = start_x + i as u16;
            if x >= bounds.x + bounds.width || x >= buffer_area.x + buffer_area.width {
                break;
            }
            let style = if i == active_idx {
                self.highlight_style.scale_uniform(active_scale)
            } else {
                base_style
            };
            let s = ch.to_string();
            buffer.set_string(x, bounds.y, &s, style);
        }

        Ok(())
    }

    fn handle_event(&mut self, _event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        Ok(false)
    }

    fn preferred_size(&self, available: Rect) -> Rect {
        let width = (self.text.width() as u16).min(available.width);
        Rect::new(available.x, available.y, width, 1)
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
