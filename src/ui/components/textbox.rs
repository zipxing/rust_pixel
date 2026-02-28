// RustPixel UI Framework - TextBox Component
// copyright zipxing@hotmail.com 2022～2025

//! TextBox component for text input.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Color, Style};
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult, WidgetEvent,
    next_widget_id
};
use crate::impl_widget_base;
use crate::event::{Event as InputEvent, KeyCode, KeyModifiers};
use unicode_width::UnicodeWidthStr;

/// TextBox component for text input
#[allow(clippy::type_complexity)]
pub struct TextBox {
    base: BaseWidget,
    text: String,
    cursor_pos: usize,
    scroll_offset: usize,
    placeholder: String,
    max_length: Option<usize>,
    password_char: Option<char>,
    on_changed: Option<Box<dyn FnMut(&str) + Send>>,
    on_enter: Option<Box<dyn FnMut(&str) + Send>>,
}

impl Default for TextBox {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBox {
    pub fn new() -> Self {
        Self {
            base: BaseWidget::new(next_widget_id()),
            text: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            placeholder: String::new(),
            max_length: None,
            password_char: None,
            on_changed: None,
            on_enter: None,
        }
    }
    
    pub fn with_style(mut self, style: Style) -> Self {
        self.base.style = style;
        self
    }
    
    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = placeholder.to_string();
        self
    }
    
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }
    
    pub fn with_password(mut self, password_char: char) -> Self {
        self.password_char = Some(password_char);
        self
    }
    
    pub fn on_changed<F>(mut self, callback: F) -> Self
    where
        F: FnMut(&str) + Send + 'static,
    {
        self.on_changed = Some(Box::new(callback));
        self
    }
    
    pub fn on_enter<F>(mut self, callback: F) -> Self
    where
        F: FnMut(&str) + Send + 'static,
    {
        self.on_enter = Some(Box::new(callback));
        self
    }
    
    pub fn set_text(&mut self, text: &str) {
        let new_text = if let Some(max_len) = self.max_length {
            text.chars().take(max_len).collect()
        } else {
            text.to_string()
        };
        
        if self.text != new_text {
            self.text = new_text;
            self.cursor_pos = self.cursor_pos.min(self.text.len());
            self.update_scroll();
            self.mark_dirty();
            self.notify_changed();
        }
    }
    
    pub fn text(&self) -> &str {
        &self.text
    }
    
    pub fn clear(&mut self) {
        self.set_text("");
    }
    
    pub fn insert_char(&mut self, ch: char) {
        if let Some(max_len) = self.max_length {
            if self.text.len() >= max_len {
                return;
            }
        }
        
        self.text.insert(self.cursor_pos, ch);
        self.cursor_pos += ch.len_utf8();
        self.update_scroll();
        self.mark_dirty();
        self.notify_changed();
    }
    
    pub fn delete_char(&mut self) {
        if self.cursor_pos < self.text.len() {
            self.text.remove(self.cursor_pos);
            self.update_scroll();
            self.mark_dirty();
            self.notify_changed();
        }
    }
    
    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            let mut char_start = self.cursor_pos - 1;
            while char_start > 0 && !self.text.is_char_boundary(char_start) {
                char_start -= 1;
            }
            self.text.remove(char_start);
            self.cursor_pos = char_start;
            self.update_scroll();
            self.mark_dirty();
            self.notify_changed();
        }
    }
    
    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            let mut new_pos = self.cursor_pos - 1;
            while new_pos > 0 && !self.text.is_char_boundary(new_pos) {
                new_pos -= 1;
            }
            self.cursor_pos = new_pos;
            self.update_scroll();
            self.mark_dirty();
        }
    }
    
    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.text.len() {
            let mut new_pos = self.cursor_pos + 1;
            while new_pos < self.text.len() && !self.text.is_char_boundary(new_pos) {
                new_pos += 1;
            }
            self.cursor_pos = new_pos;
            self.update_scroll();
            self.mark_dirty();
        }
    }
    
    pub fn move_cursor_home(&mut self) {
        self.cursor_pos = 0;
        self.scroll_offset = 0;
        self.mark_dirty();
    }
    
    pub fn move_cursor_end(&mut self) {
        self.cursor_pos = self.text.len();
        self.update_scroll();
        self.mark_dirty();
    }
    
    fn update_scroll(&mut self) {
        let bounds = self.bounds();
        let visible_width = bounds.width.saturating_sub(2) as usize; // Account for border/padding
        
        if visible_width == 0 {
            return;
        }
        
        let cursor_display_pos = self.text[..self.cursor_pos].width();
        
        // Adjust scroll offset to keep cursor visible
        if cursor_display_pos < self.scroll_offset {
            self.scroll_offset = cursor_display_pos;
        } else if cursor_display_pos >= self.scroll_offset + visible_width {
            self.scroll_offset = cursor_display_pos.saturating_sub(visible_width - 1);
        }
    }
    
    fn notify_changed(&mut self) {
        if let Some(ref mut callback) = self.on_changed {
            callback(&self.text);
        }
    }
    
    fn notify_enter(&mut self) {
        if let Some(ref mut callback) = self.on_enter {
            callback(&self.text);
        }
    }
}

impl Widget for TextBox {
    impl_widget_base!(TextBox, base);
    
    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible {
            return Ok(());
        }
        
        let bounds = self.bounds();
        if bounds.width == 0 || bounds.height == 0 {
            return Ok(());
        }
        
        // Use base style for now
        let style = self.base.style;
        
        // Clear background
        for y in bounds.y..bounds.y + bounds.height {
            for x in bounds.x..bounds.x + bounds.width {
                buffer.get_mut(x, y).set_style(style).set_symbol(" ");
            }
        }
        
        // Render border
        self.render_border(buffer, style)?;
        
        // Render text content
        self.render_content(buffer, style)?;
        
        // Render cursor if focused
        if self.state().focused {
            self.render_cursor(buffer, style)?;
        }
        
        Ok(())
    }
    
    fn handle_event(&mut self, event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        if !self.state().enabled {
            return Ok(false);
        }
        
        match event {
            UIEvent::Input(InputEvent::Key(key_event)) => {
                if self.state().focused {
                    match key_event.code {
                        KeyCode::Char(ch) => {
                            if !key_event.modifiers.contains(KeyModifiers::CONTROL) {
                                self.insert_char(ch);
                                return Ok(true);
                            }
                        }
                        KeyCode::Backspace => {
                            self.backspace();
                            return Ok(true);
                        }
                        KeyCode::Delete => {
                            self.delete_char();
                            return Ok(true);
                        }
                        KeyCode::Left => {
                            self.move_cursor_left();
                            return Ok(true);
                        }
                        KeyCode::Right => {
                            self.move_cursor_right();
                            return Ok(true);
                        }
                        KeyCode::Home => {
                            self.move_cursor_home();
                            return Ok(true);
                        }
                        KeyCode::End => {
                            self.move_cursor_end();
                            return Ok(true);
                        }
                        KeyCode::Enter => {
                            self.notify_enter();
                            return Ok(true);
                        }
                        _ => {}
                    }
                }
            }
            UIEvent::Widget(WidgetEvent::FocusGained(id)) if *id == self.id() => {
                self.state_mut().focused = true;
                self.mark_dirty();
                return Ok(true);
            }
            UIEvent::Widget(WidgetEvent::FocusLost(id)) if *id == self.id() => {
                self.state_mut().focused = false;
                self.mark_dirty();
                return Ok(true);
            }
            _ => {}
        }
        
        Ok(false)
    }
    
    fn preferred_size(&self, available: Rect) -> Rect {
        let height = 3.min(available.height); // 3 lines for better visibility (border + content + border)
        let width = available.width; // TextBox usually wants to use available width
        
        Rect::new(available.x, available.y, width, height)
    }
}

impl TextBox {
    fn render_border(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        
        if bounds.width < 2 || bounds.height < 2 {
            return Ok(());
        }
        
        let border_style = if self.state().focused {
            Style::default().fg(Color::Yellow).bg(style.bg.unwrap_or(Color::Reset))
        } else {
            Style::default().fg(Color::Gray).bg(style.bg.unwrap_or(Color::Reset))
        };
        
        // Draw full border box
        // Top border
        for x in bounds.x..bounds.x + bounds.width {
            buffer.get_mut(x, bounds.y).set_symbol("─").set_style(border_style);
        }
        
        // Bottom border
        for x in bounds.x..bounds.x + bounds.width {
            buffer.get_mut(x, bounds.y + bounds.height - 1).set_symbol("─").set_style(border_style);
        }
        
        // Left and right borders
        for y in bounds.y..bounds.y + bounds.height {
            buffer.get_mut(bounds.x, y).set_symbol("│").set_style(border_style);
            buffer.get_mut(bounds.x + bounds.width - 1, y).set_symbol("│").set_style(border_style);
        }
        
        // Corners
        buffer.get_mut(bounds.x, bounds.y).set_symbol("┌").set_style(border_style);
        buffer.get_mut(bounds.x + bounds.width - 1, bounds.y).set_symbol("┐").set_style(border_style);
        buffer.get_mut(bounds.x, bounds.y + bounds.height - 1).set_symbol("└").set_style(border_style);
        buffer.get_mut(bounds.x + bounds.width - 1, bounds.y + bounds.height - 1).set_symbol("┘").set_style(border_style);
        
        Ok(())
    }
    
    fn render_content(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        // Account for border (1 char on each side)
        let content_width = bounds.width.saturating_sub(2) as usize;
        let content_y = bounds.y + 1; // Place text in the middle row
        
        if content_width == 0 {
            return Ok(());
        }
        
        let display_text = if self.text.is_empty() {
            self.placeholder.clone()
        } else if let Some(password_char) = self.password_char {
            // For password fields, show password characters
            std::iter::repeat_n(password_char, self.text.chars().count()).collect()
        } else {
            self.text.clone()
        };
        
        // Calculate visible portion
        let visible_text = if display_text.width() > self.scroll_offset {
            let mut result = String::new();
            let mut current_width = 0;
            let mut skip_width = self.scroll_offset;
            
            for ch in display_text.chars() {
                let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                
                if skip_width > 0 {
                    skip_width = skip_width.saturating_sub(ch_width);
                    continue;
                }
                
                if current_width + ch_width > content_width {
                    break;
                }
                
                result.push(ch);
                current_width += ch_width;
            }
            
            result
        } else {
            String::new()
        };
        
        // Render text
        let text_style = if self.text.is_empty() {
            Style::default().fg(Color::DarkGray).bg(style.bg.unwrap_or(Color::Reset))
        } else {
            style
        };
        
        buffer.set_string(bounds.x + 1, content_y, &visible_text, text_style); // Account for left border
        
        Ok(())
    }
    
    fn render_cursor(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        let cursor_display_pos = self.text[..self.cursor_pos].width();
        
        if cursor_display_pos >= self.scroll_offset {
            let cursor_x = bounds.x + 1 + (cursor_display_pos - self.scroll_offset) as u16; // Account for left border
            let cursor_y = bounds.y + 1; // Place cursor in content area (middle row)
            
            if cursor_x < bounds.x + bounds.width {
                let cursor_style = Style::default()
                    .fg(style.bg.unwrap_or(Color::Black))
                    .bg(style.fg.unwrap_or(Color::White));
                
                let cursor_char = if cursor_x < bounds.x + bounds.width - 1 
                    && buffer.get(cursor_x, cursor_y).symbol != " " {
                    buffer.get(cursor_x, cursor_y).symbol.clone()
                } else {
                    " ".to_string()
                };
                
                buffer.get_mut(cursor_x, cursor_y).set_symbol(&cursor_char).set_style(cursor_style);
            }
        }
        
        Ok(())
    }
}