// RustPixel UI Framework - Button Component
// copyright zipxing@hotmail.com 2022～2025

//! Button component for user interaction.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Color, Style};
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult, WidgetEvent,
    next_widget_id
};
use crate::impl_widget_base;
use crate::event::{Event as InputEvent, MouseEventKind, MouseButton};
use unicode_width::UnicodeWidthStr;

/// Button style variants
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonStyle {
    Normal,
    Flat,
    Outlined,
}

/// Button component
pub struct Button {
    base: BaseWidget,
    text: String,
    button_style: ButtonStyle,
    callback: Option<Box<dyn FnMut() + Send>>,
}

impl Button {
    pub fn new(text: &str) -> Self {
        Self {
            base: BaseWidget::new(next_widget_id()),
            text: text.to_string(),
            button_style: ButtonStyle::Normal,
            callback: None,
        }
    }
    
    pub fn with_style(mut self, style: Style) -> Self {
        self.base.style = style;
        self
    }
    
    pub fn with_button_style(mut self, button_style: ButtonStyle) -> Self {
        self.button_style = button_style;
        self
    }
    
    pub fn on_click<F>(mut self, callback: F) -> Self 
    where
        F: FnMut() + Send + 'static,
    {
        self.callback = Some(Box::new(callback));
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
    
    pub fn click(&mut self) {
        if self.state().enabled {
            if let Some(ref mut callback) = self.callback {
                callback();
            }
        }
    }
}

impl Widget for Button {
    impl_widget_base!(Button, base);
    
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
        
        // Render button background
        self.render_background(buffer, style)?;
        
        // Render button text
        self.render_text(buffer, style)?;
        
        // Render button border if needed
        if self.button_style == ButtonStyle::Outlined {
            self.render_border(buffer, style)?;
        }
        
        Ok(())
    }
    
    fn handle_event(&mut self, event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        if !self.state().enabled {
            return Ok(false);
        }
        
        match event {
            UIEvent::Input(InputEvent::Mouse(mouse_event)) => {
                if self.hit_test(mouse_event.column, mouse_event.row) {
                    match mouse_event.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            self.state_mut().pressed = true;
                            self.mark_dirty();
                            return Ok(true);
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            if self.state().pressed {
                                self.state_mut().pressed = false;
                                self.mark_dirty();
                                self.click();
                                return Ok(true);
                            }
                        }
                        _ => {}
                    }
                }
            }
            UIEvent::Widget(WidgetEvent::MouseEnter(id)) if *id == self.id() => {
                self.state_mut().hovered = true;
                self.mark_dirty();
                return Ok(true);
            }
            UIEvent::Widget(WidgetEvent::MouseLeave(id)) if *id == self.id() => {
                self.state_mut().hovered = false;
                self.state_mut().pressed = false;
                self.mark_dirty();
                return Ok(true);
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
        let text_width = self.text.width() as u16;
        let padding = match self.button_style {
            ButtonStyle::Normal => 4, // 2 padding on each side
            ButtonStyle::Flat => 2,   // 1 padding on each side
            ButtonStyle::Outlined => 4, // 2 padding on each side
        };
        
        let width = (text_width + padding).min(available.width);
        let height = 1.min(available.height);
        
        Rect::new(available.x, available.y, width, height)
    }
}

impl Button {
    fn render_background(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        
        match self.button_style {
            ButtonStyle::Normal | ButtonStyle::Outlined => {
                // Fill background
                for y in bounds.y..bounds.y + bounds.height {
                    for x in bounds.x..bounds.x + bounds.width {
                        buffer.get_mut(x, y).set_style(style);
                        if buffer.get(x, y).symbol == " " {
                            buffer.get_mut(x, y).set_symbol(" ");
                        }
                    }
                }
            }
            ButtonStyle::Flat => {
                // Only color the text area, no background fill
            }
        }
        
        Ok(())
    }
    
    fn render_text(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        
        if self.text.is_empty() {
            return Ok(());
        }
        
        let text_width = self.text.width() as u16;
        let padding = match self.button_style {
            ButtonStyle::Normal => 2,
            ButtonStyle::Flat => 1,
            ButtonStyle::Outlined => 2,
        };
        
        // Center the text
        let available_width = bounds.width.saturating_sub(padding);
        let start_x = bounds.x + padding / 2;
        
        if text_width <= available_width && bounds.height > 0 {
            let text_x = start_x + (available_width.saturating_sub(text_width)) / 2;
            let text_y = bounds.y + bounds.height / 2;
            buffer.set_string(text_x, text_y, &self.text, style);
        }
        
        Ok(())
    }
    
    fn render_border(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        
        if bounds.width < 2 || bounds.height < 1 {
            return Ok(());
        }
        
        // Simple border using box drawing characters
        let border_style = Style::default().fg(style.fg.unwrap_or(Color::Reset));
        
        // Top and bottom borders
        for x in bounds.x..bounds.x + bounds.width {
            if bounds.height > 1 {
                buffer.get_mut(x, bounds.y).set_symbol("─").set_style(border_style);
                buffer.get_mut(x, bounds.y + bounds.height - 1).set_symbol("─").set_style(border_style);
            }
        }
        
        // Left and right borders
        for y in bounds.y..bounds.y + bounds.height {
            buffer.get_mut(bounds.x, y).set_symbol("│").set_style(border_style);
            if bounds.width > 1 {
                buffer.get_mut(bounds.x + bounds.width - 1, y).set_symbol("│").set_style(border_style);
            }
        }
        
        // Corners
        if bounds.width > 1 && bounds.height > 1 {
            buffer.get_mut(bounds.x, bounds.y).set_symbol("┌").set_style(border_style);
            buffer.get_mut(bounds.x + bounds.width - 1, bounds.y).set_symbol("┐").set_style(border_style);
            buffer.get_mut(bounds.x, bounds.y + bounds.height - 1).set_symbol("└").set_style(border_style);
            buffer.get_mut(bounds.x + bounds.width - 1, bounds.y + bounds.height - 1).set_symbol("┘").set_style(border_style);
        }
        
        Ok(())
    }
}