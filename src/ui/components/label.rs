// RustPixel UI Framework - Label Component
// copyright zipxing@hotmail.com 2022～2025

//! Label component for displaying text.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::Style;
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id
};
use crate::impl_widget_base;
use unicode_width::UnicodeWidthStr;

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// Label widget for displaying text
pub struct Label {
    base: BaseWidget,
    text: String,
    align: TextAlign,
    wrap: bool,
}

impl Label {
    pub fn new(text: &str) -> Self {
        Self {
            base: BaseWidget::new(next_widget_id()),
            text: text.to_string(),
            align: TextAlign::Left,
            wrap: false,
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
}

impl Widget for Label {
    impl_widget_base!(Label, base);
    
    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible {
            return Ok(());
        }
        
        let bounds = self.bounds();
        if bounds.width == 0 || bounds.height == 0 {
            return Ok(());
        }
        
        let style = self.base.style;
        
        if self.wrap {
            self.render_wrapped(buffer, style)?;
        } else {
            self.render_single_line(buffer, style)?;
        }
        
        Ok(())
    }
    
    fn handle_event(&mut self, _event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        // Labels don't handle events by default
        Ok(false)
    }
    
    fn preferred_size(&self, available: Rect) -> Rect {
        if self.text.is_empty() {
            return Rect::new(available.x, available.y, 0, 1);
        }
        
        if self.wrap {
            // Calculate wrapped text size
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
            // Single line
            let width = (self.text.width() as u16).min(available.width);
            Rect::new(available.x, available.y, width, 1)
        }
    }
}

impl Label {
    fn render_single_line(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        let text_width = self.text.width() as u16;
        
        if text_width == 0 {
            return Ok(());
        }
        
        let start_x = match self.align {
            TextAlign::Left => bounds.x,
            TextAlign::Center => bounds.x + (bounds.width.saturating_sub(text_width)) / 2,
            TextAlign::Right => bounds.x + bounds.width.saturating_sub(text_width),
        };
        
        if start_x < bounds.x + bounds.width {
            buffer.set_string(start_x, bounds.y, &self.text, style);
        }
        
        Ok(())
    }
    
    fn render_wrapped(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        let lines = self.wrap_text(bounds.width);
        
        for (i, line) in lines.iter().enumerate() {
            let y = bounds.y + i as u16;
            if y >= bounds.y + bounds.height {
                break;
            }
            
            let line_width = line.width() as u16;
            let start_x = match self.align {
                TextAlign::Left => bounds.x,
                TextAlign::Center => bounds.x + (bounds.width.saturating_sub(line_width)) / 2,
                TextAlign::Right => bounds.x + bounds.width.saturating_sub(line_width),
            };
            
            if start_x < bounds.x + bounds.width {
                buffer.set_string(start_x, y, line, style);
            }
        }
        
        Ok(())
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
            
            // If adding this word would exceed the width, start a new line
            if current_width > 0 && current_width + 1 + word_width > width {
                lines.push(current_line);
                current_line = word.to_string();
                current_width = word_width;
            } else {
                // Add word to current line
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