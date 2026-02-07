// RustPixel UI Framework - Label Component
// copyright zipxing@hotmail.com 2022～2025

//! Label component for displaying text, with optional animation effects.
//!
//! Supports per-character animations driven entirely through Style
//! (fg, bg, modifier, per-cell scale). Horizontal character spacing
//! is always fixed — animations only change visual appearance, never layout.
//!
//! Available animations:
//! - **Spotlight**: sequential per-char scale pulse
//! - **Wave**: sinusoidal scale wave traveling across characters
//! - **FadeIn**: characters reveal left-to-right with scale-up
//! - **Typewriter**: characters appear one by one with optional cursor

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::Style;
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id
};
use unicode_width::UnicodeWidthStr;

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// Animation effect that can be applied to a Label.
///
/// All animations render per-character with fixed horizontal spacing —
/// only visual attributes (scale, color, visibility) change, never position.
#[derive(Debug, Clone)]
pub enum LabelAnimation {
    /// Per-character spotlight: each character is highlighted sequentially
    /// with a scale pulse.
    Spotlight {
        /// Style for the currently highlighted character
        highlight_style: Style,
        /// Animation frames per character (speed control)
        frames_per_char: usize,
        /// Scale pulse amplitude (e.g. 0.55 → scale range 1.0~1.55)
        scale_amplitude: f32,
    },

    /// Sinusoidal scale wave traveling across characters.
    /// Each character's scale oscillates based on its position in the wave.
    Wave {
        /// Scale amplitude (e.g. 0.3 → scale range 0.7~1.3)
        amplitude: f32,
        /// Characters per full wave cycle
        wavelength: f32,
        /// Phase shift per frame (radians); controls wave speed
        speed: f32,
    },

    /// Characters fade in left-to-right with a scale-up effect.
    /// Each character scales from near-zero to 1.0 as it's revealed.
    FadeIn {
        /// Frames to fully reveal each character
        frames_per_char: usize,
        /// Restart after all characters are revealed
        loop_anim: bool,
    },

    /// Characters appear one by one (typewriter effect).
    /// Unrevealed characters are invisible; an optional cursor blinks at
    /// the current typing position.
    Typewriter {
        /// Frames between each character appearing
        frames_per_char: usize,
        /// Show a blinking cursor at the typing position
        show_cursor: bool,
        /// Restart after all characters are shown
        loop_anim: bool,
    },
}

/// Label widget for displaying text with optional animation
pub struct Label {
    base: BaseWidget,
    text: String,
    align: TextAlign,
    wrap: bool,
    /// Optional animation effect
    animation: Option<LabelAnimation>,
    /// Current animation frame counter
    frame: usize,
}

impl Label {
    pub fn new(text: &str) -> Self {
        Self {
            base: BaseWidget::new(next_widget_id()),
            text: text.to_string(),
            align: TextAlign::Left,
            wrap: false,
            animation: None,
            frame: 0,
        }
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.base.style = style;
        self
    }

    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn with_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Set an animation effect on this label
    pub fn with_animation(mut self, animation: LabelAnimation) -> Self {
        self.animation = Some(animation);
        self
    }

    /// Convenience: configure a spotlight animation
    pub fn with_spotlight(mut self, highlight_style: Style, frames_per_char: usize, scale_amplitude: f32) -> Self {
        self.animation = Some(LabelAnimation::Spotlight {
            highlight_style,
            frames_per_char: frames_per_char.max(1),
            scale_amplitude,
        });
        self
    }

    /// Convenience: configure a wave animation
    pub fn with_wave(mut self, amplitude: f32, wavelength: f32, speed: f32) -> Self {
        self.animation = Some(LabelAnimation::Wave {
            amplitude,
            wavelength: wavelength.max(1.0),
            speed,
        });
        self
    }

    /// Convenience: configure a fade-in animation
    pub fn with_fade_in(mut self, frames_per_char: usize, loop_anim: bool) -> Self {
        self.animation = Some(LabelAnimation::FadeIn {
            frames_per_char: frames_per_char.max(1),
            loop_anim,
        });
        self
    }

    /// Convenience: configure a typewriter animation
    pub fn with_typewriter(mut self, frames_per_char: usize, show_cursor: bool, loop_anim: bool) -> Self {
        self.animation = Some(LabelAnimation::Typewriter {
            frames_per_char: frames_per_char.max(1),
            show_cursor,
            loop_anim,
        });
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

    pub fn set_align(&mut self, align: TextAlign) {
        if self.align != align {
            self.align = align;
            self.mark_dirty();
        }
    }

    pub fn set_wrap(&mut self, wrap: bool) {
        if self.wrap != wrap {
            self.wrap = wrap;
            self.mark_dirty();
        }
    }

    pub fn set_animation(&mut self, animation: Option<LabelAnimation>) {
        self.animation = animation;
        self.frame = 0;
        self.mark_dirty();
    }

    pub fn animation(&self) -> Option<&LabelAnimation> {
        self.animation.as_ref()
    }
}

impl Widget for Label {
    fn id(&self) -> WidgetId { self.base.id }
    fn bounds(&self) -> Rect { self.base.bounds }
    fn set_bounds(&mut self, bounds: Rect) {
        self.base.bounds = bounds;
        self.base.state.dirty = true;
    }
    fn state(&self) -> &WidgetState { &self.base.state }
    fn state_mut(&mut self) -> &mut WidgetState { &mut self.base.state }
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }

    fn update(&mut self, _dt: f32, _ctx: &mut Context) -> UIResult<()> {
        if self.animation.is_some() {
            self.frame = self.frame.wrapping_add(1);
            self.mark_dirty();
        }
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

        let style = self.base.style;

        // Animated render path (per-character)
        if let Some(ref anim) = self.animation {
            return self.render_animated(buffer, style, anim);
        }

        // Static render path
        if self.wrap {
            self.render_wrapped(buffer, style)?;
        } else {
            self.render_single_line(buffer, style)?;
        }

        Ok(())
    }

    fn handle_event(&mut self, _event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        Ok(false)
    }

    fn preferred_size(&self, available: Rect) -> Rect {
        if self.text.is_empty() {
            return Rect::new(available.x, available.y, 0, 1);
        }

        if self.wrap {
            let lines = self.wrap_text(available.width);
            let height = (lines.len() as u16).min(available.height);
            let width = if lines.is_empty() {
                0
            } else {
                lines.iter()
                    .map(|line| line.width() as u16)
                    .max()
                    .unwrap_or(0)
                    .min(available.width)
            };
            Rect::new(available.x, available.y, width, height)
        } else {
            let width = (self.text.width() as u16).min(available.width);
            Rect::new(available.x, available.y, width, 1)
        }
    }
}

// ========== Private rendering methods ==========

impl Label {
    /// Compute visual text width in cells, accounting for per-cell scale.
    /// When scale_x >= 1.0, each character occupies scale_x cells visually
    /// (graph.rs expands slot width for scale >= 1.0).
    fn visual_text_width(&self) -> u16 {
        let scale_x = self.base.style.scale_x.unwrap_or(1.0);
        if scale_x > 1.0 {
            (self.text.width() as f32 * scale_x).ceil() as u16
        } else {
            self.text.width() as u16
        }
    }

    fn render_single_line(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        let text_width = self.visual_text_width();

        if text_width == 0 {
            return Ok(());
        }

        let buffer_area = *buffer.area();
        if bounds.y >= buffer_area.y + buffer_area.height || bounds.x >= buffer_area.x + buffer_area.width {
            return Ok(());
        }

        let start_x = match self.align {
            TextAlign::Left => bounds.x,
            TextAlign::Center => bounds.x + (bounds.width.saturating_sub(text_width)) / 2,
            TextAlign::Right => bounds.x + bounds.width.saturating_sub(text_width),
        };

        if start_x < bounds.x + bounds.width && start_x < buffer_area.x + buffer_area.width {
            buffer.set_string(start_x, bounds.y, &self.text, style);
        }

        Ok(())
    }

    fn render_wrapped(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        let lines = self.wrap_text(bounds.width);

        let buffer_area = *buffer.area();
        if bounds.y >= buffer_area.y + buffer_area.height || bounds.x >= buffer_area.x + buffer_area.width {
            return Ok(());
        }

        let scale_x = self.base.style.scale_x.unwrap_or(1.0);

        for (i, line) in lines.iter().enumerate() {
            let y = bounds.y + i as u16;
            if y >= bounds.y + bounds.height || y >= buffer_area.y + buffer_area.height {
                break;
            }

            let line_width = if scale_x > 1.0 {
                (line.width() as f32 * scale_x).ceil() as u16
            } else {
                line.width() as u16
            };
            let start_x = match self.align {
                TextAlign::Left => bounds.x,
                TextAlign::Center => bounds.x + (bounds.width.saturating_sub(line_width)) / 2,
                TextAlign::Right => bounds.x + bounds.width.saturating_sub(line_width),
            };

            if start_x < bounds.x + bounds.width && start_x < buffer_area.x + buffer_area.width {
                buffer.set_string(start_x, y, line, style);
            }
        }

        Ok(())
    }

    /// Compute start_x for aligned text (shared by all animated renders).
    /// Uses visual text width (scale-aware) for correct centering.
    fn aligned_start_x(&self, text_width: u16) -> u16 {
        let bounds = self.bounds();
        match self.align {
            TextAlign::Left => bounds.x,
            TextAlign::Center => bounds.x + bounds.width.saturating_sub(text_width) / 2,
            TextAlign::Right => bounds.x + bounds.width.saturating_sub(text_width),
        }
    }

    fn render_animated(&self, buffer: &mut Buffer, base_style: Style, anim: &LabelAnimation) -> UIResult<()> {
        let bounds = self.bounds();
        let buffer_area = *buffer.area();
        if bounds.y >= buffer_area.y + buffer_area.height
            || bounds.x >= buffer_area.x + buffer_area.width
        {
            return Ok(());
        }

        let text_width = self.visual_text_width();
        if text_width == 0 {
            return Ok(());
        }

        let start_x = self.aligned_start_x(text_width);

        match anim {
            LabelAnimation::Spotlight { highlight_style, frames_per_char, scale_amplitude } => {
                self.render_spotlight(buffer, base_style, start_x, *highlight_style, *frames_per_char, *scale_amplitude);
            }
            LabelAnimation::Wave { amplitude, wavelength, speed } => {
                self.render_wave(buffer, base_style, start_x, *amplitude, *wavelength, *speed);
            }
            LabelAnimation::FadeIn { frames_per_char, loop_anim } => {
                self.render_fade_in(buffer, base_style, start_x, *frames_per_char, *loop_anim);
            }
            LabelAnimation::Typewriter { frames_per_char, show_cursor, loop_anim } => {
                self.render_typewriter(buffer, base_style, start_x, *frames_per_char, *show_cursor, *loop_anim);
            }
        }

        Ok(())
    }

    fn render_spotlight(&self, buffer: &mut Buffer, base_style: Style, start_x: u16,
                        highlight_style: Style, frames_per_char: usize, scale_amplitude: f32) {
        let bounds = self.bounds();
        let buffer_area = *buffer.area();
        let char_count = self.text.chars().count();
        if char_count == 0 { return; }

        let cycle_len = char_count * frames_per_char;
        let frame_in_cycle = self.frame % cycle_len;
        let active_idx = frame_in_cycle / frames_per_char;
        let progress = (frame_in_cycle % frames_per_char) as f32 / frames_per_char as f32;
        let active_scale = 1.0 + scale_amplitude * (progress * std::f32::consts::PI).sin();
        let normal_style = base_style.scale_uniform(1.0);

        for (i, ch) in self.text.chars().enumerate() {
            let x = start_x + i as u16;
            if x >= bounds.x + bounds.width || x >= buffer_area.x + buffer_area.width { break; }
            let style = if i == active_idx {
                highlight_style.scale_uniform(active_scale)
            } else {
                normal_style
            };
            buffer.set_string(x, bounds.y, &ch.to_string(), style);
        }
    }

    fn render_wave(&self, buffer: &mut Buffer, base_style: Style, start_x: u16,
                   amplitude: f32, wavelength: f32, speed: f32) {
        let bounds = self.bounds();
        let buffer_area = *buffer.area();
        let pi2 = 2.0 * std::f32::consts::PI;

        for (i, ch) in self.text.chars().enumerate() {
            let x = start_x + i as u16;
            if x >= bounds.x + bounds.width || x >= buffer_area.x + buffer_area.width { break; }

            let phase = speed * self.frame as f32 + i as f32 * pi2 / wavelength;
            let scale = (1.0 + amplitude * phase.sin()).max(0.1);
            let style = base_style.scale_uniform(scale);
            buffer.set_string(x, bounds.y, &ch.to_string(), style);
        }
    }

    fn render_fade_in(&self, buffer: &mut Buffer, base_style: Style, start_x: u16,
                      frames_per_char: usize, loop_anim: bool) {
        let bounds = self.bounds();
        let buffer_area = *buffer.area();
        let char_count = self.text.chars().count();
        if char_count == 0 { return; }

        let total_frames = char_count * frames_per_char;
        let effective_frame = if loop_anim {
            self.frame % total_frames
        } else {
            self.frame.min(total_frames)
        };

        let fully_revealed = effective_frame / frames_per_char;
        let sub_progress = (effective_frame % frames_per_char) as f32 / frames_per_char as f32;
        let normal_style = base_style.scale_uniform(1.0);

        for (i, ch) in self.text.chars().enumerate() {
            let x = start_x + i as u16;
            if x >= bounds.x + bounds.width || x >= buffer_area.x + buffer_area.width { break; }

            if i < fully_revealed {
                // Already revealed — normal size
                buffer.set_string(x, bounds.y, &ch.to_string(), normal_style);
            } else if i == fully_revealed && fully_revealed < char_count {
                // Currently appearing — scale from 0.1 to 1.0
                let scale = 0.1 + 0.9 * sub_progress;
                let style = base_style.scale_uniform(scale);
                buffer.set_string(x, bounds.y, &ch.to_string(), style);
            }
            // Unrevealed chars: leave buffer cell untouched (invisible)
        }
    }

    fn render_typewriter(&self, buffer: &mut Buffer, base_style: Style, start_x: u16,
                         frames_per_char: usize, show_cursor: bool, loop_anim: bool) {
        let bounds = self.bounds();
        let buffer_area = *buffer.area();
        let char_count = self.text.chars().count();
        if char_count == 0 { return; }

        // Extra pause at end before looping (one "character" worth of frames)
        let total_frames = char_count * frames_per_char;
        let loop_len = total_frames + frames_per_char;
        let effective_frame = if loop_anim {
            self.frame % loop_len
        } else {
            self.frame.min(total_frames)
        };

        let revealed = (effective_frame / frames_per_char).min(char_count);
        let normal_style = base_style.scale_uniform(1.0);

        for (i, ch) in self.text.chars().enumerate() {
            let x = start_x + i as u16;
            if x >= bounds.x + bounds.width || x >= buffer_area.x + buffer_area.width { break; }

            if i < revealed {
                buffer.set_string(x, bounds.y, &ch.to_string(), normal_style);
            }
            // Unrevealed chars: leave buffer cell untouched
        }

        // Blinking cursor at typing position
        if show_cursor && revealed < char_count {
            let cursor_x = start_x + revealed as u16;
            if cursor_x < bounds.x + bounds.width && cursor_x < buffer_area.x + buffer_area.width {
                // Blink: visible for 8 frames, invisible for 8 frames
                if (self.frame / 8) % 2 == 0 {
                    buffer.set_string(cursor_x, bounds.y, "▌", base_style.scale_uniform(1.0));
                }
            }
        }
    }

    fn wrap_text(&self, width: u16) -> Vec<String> {
        if width == 0 {
            return vec![];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut current_width = 0;

        for word in self.text.split_whitespace() {
            let word_width = word.width() as u16;

            if current_width > 0 && current_width + 1 + word_width > width {
                lines.push(current_line);
                current_line = word.to_string();
                current_width = word_width;
            } else {
                if current_width > 0 {
                    current_line.push(' ');
                    current_width += 1;
                }
                current_line.push_str(word);
                current_width += word_width;
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }
}
