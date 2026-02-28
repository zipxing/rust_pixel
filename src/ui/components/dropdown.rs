// RustPixel UI Framework - Dropdown Component
// copyright zipxing@hotmail.com 2022～2025

//! Dropdown/Select component - character-cell dropdown selector.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Style, Color};
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id,
};
use crate::impl_widget_base;

/// Dropdown widget: a collapsible dropdown selector.
pub struct Dropdown {
    base: BaseWidget,
    options: Vec<String>,
    selected: Option<usize>,
    expanded: bool,
    style: Style,
    selected_style: Style,
    highlight_style: Style,
    hover_index: Option<usize>,
    on_change: Option<Box<dyn Fn(usize) + 'static>>,
}

impl Default for Dropdown {
    fn default() -> Self {
        Self::new()
    }
}

impl Dropdown {
    pub fn new() -> Self {
        let id = next_widget_id();
        Self {
            base: BaseWidget::new(id),
            options: Vec::new(),
            selected: None,
            expanded: false,
            style: Style::default().fg(Color::White).bg(Color::Black),
            selected_style: Style::default().fg(Color::Green).bg(Color::Black),
            highlight_style: Style::default().fg(Color::Black).bg(Color::White),
            hover_index: None,
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
            self.expanded = false;
            self.mark_dirty();
            if let Some(ref callback) = self.on_change {
                callback(index);
            }
        }
    }

    pub fn get_selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
        self.mark_dirty();
    }

    fn selector_rect(&self) -> Rect {
        let b = self.bounds();
        Rect::new(b.x, b.y, b.width, 1)
    }

    fn dropdown_rect(&self) -> Rect {
        let b = self.bounds();
        // Dropdown can extend beyond bounds when expanded (overlay effect)
        let height = self.options.len() as u16;
        Rect::new(b.x, b.y + 1, b.width, height)
    }
}

impl Widget for Dropdown {
    impl_widget_base!(Dropdown, base);

    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible { return Ok(()); }
        let b = self.bounds();
        if b.width == 0 || b.height == 0 { return Ok(()); }

        // Check if position is within buffer bounds
        let buffer_area = *buffer.area();
        if b.y >= buffer_area.y + buffer_area.height || b.x >= buffer_area.x + buffer_area.width {
            return Ok(());
        }

        // Render selector box
        let selector = self.selector_rect();
        if selector.y < buffer_area.y + buffer_area.height {
            let selected_text = self.selected
                .and_then(|i| self.options.get(i))
                .map(|s| s.as_str())
                .unwrap_or("Select...");
            
            let display = format!("[{}▼]", selected_text);
            let max_len = selector.width.saturating_sub(3).min(buffer_area.width.saturating_sub(selector.x - buffer_area.x)) as usize;
            let text = if display.len() > max_len + 3 {
                format!("[{}▼]", &selected_text[..max_len.saturating_sub(3)])
            } else {
                display
            };
            
            if selector.x < buffer_area.x + buffer_area.width {
                buffer.set_string(selector.x, selector.y, &text, self.selected_style);
            }
        }

        // Render dropdown list if expanded
        if self.expanded {
            let dropdown = self.dropdown_rect();
            for (i, option) in self.options.iter().enumerate() {
                let y = dropdown.y + i as u16;
                if y >= buffer_area.y + buffer_area.height || y >= dropdown.y + dropdown.height {
                    break;
                }

                let is_hover = self.hover_index == Some(i);
                let style = if is_hover { self.highlight_style } else { self.style };
                
                let max_len = dropdown.width.min(buffer_area.width.saturating_sub(dropdown.x - buffer_area.x)) as usize;
                let text = if option.len() > max_len {
                    &option[..max_len]
                } else {
                    option
                };
                
                // Fill the entire line with background
                for x in dropdown.x..dropdown.x + dropdown.width.min(buffer_area.width.saturating_sub(dropdown.x - buffer_area.x)) {
                    buffer.get_mut(x, y).set_symbol(" ").set_style(style);
                }
                
                if dropdown.x < buffer_area.x + buffer_area.width {
                    buffer.set_string(dropdown.x, y, text, style);
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
                let selector = self.selector_rect();
                
                // Click on selector
                if mouse_event.row == selector.y 
                    && mouse_event.column >= selector.x 
                    && mouse_event.column < selector.x + selector.width {
                    self.toggle();
                    return Ok(true);
                }

                // Click on dropdown option
                if self.expanded {
                    let dropdown = self.dropdown_rect();
                    if mouse_event.row >= dropdown.y 
                        && mouse_event.row < dropdown.y + dropdown.height
                        && mouse_event.column >= dropdown.x 
                        && mouse_event.column < dropdown.x + dropdown.width {
                        let index = (mouse_event.row - dropdown.y) as usize;
                        if index < self.options.len() {
                            self.set_selected(index);
                            return Ok(true);
                        }
                    }
                }
            }
            
            // Track hover for highlighting
            if let crate::event::MouseEventKind::Moved = mouse_event.kind {
                if self.expanded {
                    let dropdown = self.dropdown_rect();
                    if mouse_event.row >= dropdown.y 
                        && mouse_event.row < dropdown.y + dropdown.height
                        && mouse_event.column >= dropdown.x 
                        && mouse_event.column < dropdown.x + dropdown.width {
                        let index = (mouse_event.row - dropdown.y) as usize;
                        if index < self.options.len() {
                            self.hover_index = Some(index);
                            self.mark_dirty();
                        }
                    } else {
                        self.hover_index = None;
                        self.mark_dirty();
                    }
                }
            }
        }

        // Handle keyboard
        if let UIEvent::Input(crate::event::Event::Key(key)) = event {
            match key.code {
                crate::event::KeyCode::Enter | crate::event::KeyCode::Char(' ') => {
                    self.toggle();
                    return Ok(true);
                }
                crate::event::KeyCode::Esc => {
                    if self.expanded {
                        self.expanded = false;
                        self.mark_dirty();
                        return Ok(true);
                    }
                }
                crate::event::KeyCode::Up => {
                    if self.expanded {
                        if let Some(current) = self.selected {
                            if current > 0 {
                                self.set_selected(current - 1);
                                return Ok(true);
                            }
                        }
                    }
                }
                crate::event::KeyCode::Down => {
                    if self.expanded {
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
                }
                _ => {}
            }
        }

        Ok(false)
    }

    fn preferred_size(&self, available: Rect) -> Rect {
        // Always request only 1 row (dropdown list is an overlay)
        let height = 1;
        
        // Width based on longest option
        let width = self.options.iter()
            .map(|opt| opt.len() as u16 + 3)
            .max()
            .unwrap_or(12)
            .min(available.width);
        
        Rect::new(available.x, available.y, width, height)
    }
}

