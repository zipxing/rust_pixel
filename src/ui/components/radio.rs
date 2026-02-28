// RustPixel UI Framework - Radio Component
// copyright zipxing@hotmail.com 2022～2025

//! Radio component - character-cell radio button group.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Style, Color};
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id,
};
use crate::impl_widget_base;

/// RadioGroup widget: a group of mutually exclusive radio buttons.
pub struct RadioGroup {
    base: BaseWidget,
    options: Vec<String>,
    selected: Option<usize>,
    style: Style,
    selected_style: Style,
    spacing: u16,
    on_change: Option<Box<dyn Fn(usize) + 'static>>,
}

impl Default for RadioGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl RadioGroup {
    pub fn new() -> Self {
        let id = next_widget_id();
        Self {
            base: BaseWidget::new(id),
            options: Vec::new(),
            selected: None,
            style: Style::default().fg(Color::White).bg(Color::Black),
            selected_style: Style::default().fg(Color::Green).bg(Color::Black),
            spacing: 1,
            on_change: None,
        }
    }

    pub fn with_options(mut self, options: Vec<String>) -> Self {
        self.options = options;
        self
    }

    pub fn add_option(&mut self, option: &str) {
        self.options.push(option.to_string());
        self.mark_dirty();
    }

    pub fn with_selected(mut self, index: usize) -> Self {
        if index < self.options.len() {
            self.selected = Some(index);
        }
        self
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_selected_style(mut self, style: Style) -> Self {
        self.selected_style = style;
        self
    }

    pub fn with_spacing(mut self, spacing: u16) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(usize) + 'static,
    {
        self.on_change = Some(Box::new(callback));
        self
    }

    pub fn set_selected(&mut self, index: usize) {
        if index < self.options.len() && self.selected != Some(index) {
            self.selected = Some(index);
            self.mark_dirty();
            if let Some(ref callback) = self.on_change {
                callback(index);
            }
        }
    }

    pub fn get_selected(&self) -> Option<usize> {
        self.selected
    }

    fn get_option_rect(&self, index: usize) -> Option<Rect> {
        let b = self.bounds();
        if index >= self.options.len() {
            return None;
        }
        
        let y = b.y + (index as u16 * (1 + self.spacing));
        if y >= b.y + b.height {
            return None;
        }
        
        Some(Rect::new(b.x, y, b.width, 1))
    }
}

impl Widget for RadioGroup {
    impl_widget_base!(RadioGroup, base);

    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible { return Ok(()); }
        let b = self.bounds();
        if b.width == 0 || b.height == 0 { return Ok(()); }

        // Check if position is within buffer bounds
        let buffer_area = *buffer.area();
        if b.y >= buffer_area.y + buffer_area.height || b.x >= buffer_area.x + buffer_area.width {
            return Ok(());
        }

        // Render each option
        for (i, option) in self.options.iter().enumerate() {
            if let Some(rect) = self.get_option_rect(i) {
                // Check if this line is within buffer bounds
                if rect.y >= buffer_area.y + buffer_area.height {
                    break;
                }

                let is_selected = self.selected == Some(i);
                let radio_symbol = if is_selected { "(●)" } else { "( )" };
                let radio_style = if is_selected { self.selected_style } else { self.style };
                
                // Render radio button
                if rect.x + 3 < buffer_area.x + buffer_area.width {
                    buffer.set_string(rect.x, rect.y, radio_symbol, radio_style);
                }

                // Render label if there's space
                if rect.width > 4 && !option.is_empty() {
                    let label_x = rect.x + 4;
                    if label_x < buffer_area.x + buffer_area.width {
                        let max_len = (rect.width.saturating_sub(4))
                            .min(buffer_area.width.saturating_sub(label_x - buffer_area.x)) as usize;
                        let label_text = if option.len() > max_len {
                            &option[..max_len]
                        } else {
                            option
                        };
                        buffer.set_string(label_x, rect.y, label_text, self.style);
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_event(&mut self, event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        if !self.state().visible { return Ok(false); }

        // Handle mouse click
        if let UIEvent::Input(crate::event::Event::Mouse(mouse_event)) = event {
            if let crate::event::MouseEventKind::Down(crate::event::MouseButton::Left) = mouse_event.kind {
                // Check which option was clicked
                for (i, _) in self.options.iter().enumerate() {
                    if let Some(rect) = self.get_option_rect(i) {
                        if mouse_event.column >= rect.x && mouse_event.column < rect.x + rect.width
                            && mouse_event.row == rect.y {
                            self.set_selected(i);
                            return Ok(true);
                        }
                    }
                }
            }
        }

        // Handle arrow keys
        if let UIEvent::Input(crate::event::Event::Key(key)) = event {
            match key.code {
                crate::event::KeyCode::Up => {
                    if let Some(current) = self.selected {
                        if current > 0 {
                            self.set_selected(current - 1);
                            return Ok(true);
                        }
                    }
                }
                crate::event::KeyCode::Down => {
                    if let Some(current) = self.selected {
                        if current + 1 < self.options.len() {
                            self.set_selected(current + 1);
                            return Ok(true);
                        }
                    } else if !self.options.is_empty() {
                        self.set_selected(0);
                        return Ok(true);
                    }
                }
                _ => {}
            }
        }

        Ok(false)
    }

    fn preferred_size(&self, available: Rect) -> Rect {
        // Calculate height based on number of options and spacing
        let height = if self.options.is_empty() {
            1
        } else {
            (self.options.len() as u16 * (1 + self.spacing)).saturating_sub(self.spacing)
        };
        
        // Width based on longest option
        let width = self.options.iter()
            .map(|opt| 4 + opt.len() as u16)
            .max()
            .unwrap_or(4)
            .min(available.width);
        
        Rect::new(available.x, available.y, width, height.min(available.height))
    }
}

