// RustPixel UI Framework - List Component
// copyright zipxing@hotmail.com 2022～2025

//! List component for displaying and selecting items.

use crate::context::Context;
use crate::render::{Buffer, Cell};
use crate::render::style::{Color, Style};
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult, WidgetEvent, WidgetValue,
    next_widget_id
};
use crate::impl_widget_base;
use crate::event::{Event as InputEvent, KeyEvent, KeyCode, MouseEvent, MouseEventKind, MouseButton};
use unicode_width::UnicodeWidthStr;

/// List item data
#[derive(Debug, Clone)]
pub struct ListItem {
    pub text: String,
    pub data: Option<String>, // Optional data associated with the item
    pub enabled: bool,
}

impl ListItem {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            data: None,
            enabled: true,
        }
    }
    
    pub fn with_data(mut self, data: &str) -> Self {
        self.data = Some(data.to_string());
        self
    }
    
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// List selection mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionMode {
    None,     // No selection
    Single,   // Single item selection
    Multiple, // Multiple item selection
}

/// List component for displaying selectable items
pub struct List {
    base: BaseWidget,
    items: Vec<ListItem>,
    selected_indices: Vec<usize>,
    selection_mode: SelectionMode,
    scroll_offset: usize,
    show_scrollbar: bool,
    on_selection_changed: Option<Box<dyn FnMut(Vec<usize>) + Send>>,
}

impl List {
    pub fn new() -> Self {
        Self {
            base: BaseWidget::new(next_widget_id()),
            items: Vec::new(),
            selected_indices: Vec::new(),
            selection_mode: SelectionMode::Single,
            scroll_offset: 0,
            show_scrollbar: true,
            on_selection_changed: None,
        }
    }
    
    pub fn with_style(mut self, style: Style) -> Self {
        self.base.style = style;
        self
    }
    
    pub fn with_selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }
    
    pub fn with_scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }
    
    pub fn on_selection_changed<F>(mut self, callback: F) -> Self
    where
        F: FnMut(Vec<usize>) + Send + 'static,
    {
        self.on_selection_changed = Some(Box::new(callback));
        self
    }
    
    pub fn add_item(&mut self, item: ListItem) {
        self.items.push(item);
        self.mark_dirty();
    }
    
    pub fn add_text_item(&mut self, text: &str) {
        self.add_item(ListItem::new(text));
    }
    
    pub fn remove_item(&mut self, index: usize) -> Option<ListItem> {
        if index < self.items.len() {
            let item = self.items.remove(index);
            
            // Update selection indices
            self.selected_indices.retain(|&i| i != index);
            for selected in &mut self.selected_indices {
                if *selected > index {
                    *selected -= 1;
                }
            }
            
            self.mark_dirty();
            Some(item)
        } else {
            None
        }
    }
    
    pub fn clear(&mut self) {
        self.items.clear();
        self.selected_indices.clear();
        self.scroll_offset = 0;
        self.mark_dirty();
    }
    
    pub fn select_item(&mut self, index: usize) {
        if index >= self.items.len() || self.selection_mode == SelectionMode::None {
            return;
        }
        
        match self.selection_mode {
            SelectionMode::Single => {
                self.selected_indices.clear();
                self.selected_indices.push(index);
            }
            SelectionMode::Multiple => {
                if !self.selected_indices.contains(&index) {
                    self.selected_indices.push(index);
                }
            }
            SelectionMode::None => return,
        }
        
        self.mark_dirty();
        self.notify_selection_changed();
    }
    
    pub fn deselect_item(&mut self, index: usize) {
        self.selected_indices.retain(|&i| i != index);
        self.mark_dirty();
        self.notify_selection_changed();
    }
    
    pub fn selected_indices(&self) -> &[usize] {
        &self.selected_indices
    }
    
    pub fn selected_items(&self) -> Vec<&ListItem> {
        self.selected_indices.iter()
            .filter_map(|&i| self.items.get(i))
            .collect()
    }
    
    pub fn items(&self) -> &[ListItem] {
        &self.items
    }
    
    pub fn scroll_to(&mut self, index: usize) {
        let bounds = self.bounds();
        let visible_count = bounds.height as usize;
        
        if index < self.scroll_offset {
            self.scroll_offset = index;
        } else if index >= self.scroll_offset + visible_count {
            self.scroll_offset = index.saturating_sub(visible_count - 1);
        }
        
        self.mark_dirty();
    }
    
    fn notify_selection_changed(&mut self) {
        if let Some(ref mut callback) = self.on_selection_changed {
            callback(self.selected_indices.clone());
        }
    }
    
    fn visible_range(&self) -> (usize, usize) {
        let bounds = self.bounds();
        let visible_count = bounds.height as usize;
        let start = self.scroll_offset;
        let end = (start + visible_count).min(self.items.len());
        (start, end)
    }
}

impl Widget for List {
    impl_widget_base!(List, base);
    
    fn render(&self, buffer: &mut Buffer, ctx: &Context) -> UIResult<()> {
        if !self.state().visible {
            return Ok(());
        }
        
        let bounds = self.bounds();
        if bounds.width == 0 || bounds.height == 0 {
            return Ok(());
        }
        
        // Use base style for now
        let list_style = self.base.style;
        
        let item_style: Option<&crate::ui::ComponentStyle> = None; // TODO: integrate with theme system
        
        // Clear background
        for y in bounds.y..bounds.y + bounds.height {
            for x in bounds.x..bounds.x + bounds.width {
                buffer.get_mut(x, y).set_style(list_style).set_symbol(" ");
            }
        }
        
        // Render items
        let (start, end) = self.visible_range();
        let list_width = if self.show_scrollbar && self.items.len() > bounds.height as usize {
            bounds.width.saturating_sub(1)
        } else {
            bounds.width
        };
        
        for (display_index, item_index) in (start..end).enumerate() {
            let y = bounds.y + display_index as u16;
            let item = &self.items[item_index];
            let is_selected = self.selected_indices.contains(&item_index);
            
            // Get item style
            let current_style = if let Some(item_theme) = item_style {
                item_theme.get_style(is_selected, false, false, item.enabled)
            } else if is_selected {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                list_style
            };
            
            // Truncate text if needed
            let display_text = if item.text.width() > list_width as usize {
                let mut truncated = String::new();
                let mut width = 0;
                for ch in item.text.chars() {
                    let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
                    if width + ch_width > list_width as usize {
                        break;
                    }
                    truncated.push(ch);
                    width += ch_width;
                }
                truncated
            } else {
                item.text.clone()
            };
            
            // Render item text
            buffer.set_string(bounds.x, y, &display_text, current_style);
            
            // Fill remaining space with background
            for x in (bounds.x + display_text.width() as u16)..bounds.x + list_width {
                buffer.get_mut(x, y).set_style(current_style).set_symbol(" ");
            }
        }
        
        // Render scrollbar if needed
        if self.show_scrollbar && self.items.len() > bounds.height as usize {
            self.render_scrollbar(buffer, list_style)?;
        }
        
        Ok(())
    }
    
    fn handle_event(&mut self, event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        if !self.state().enabled {
            return Ok(false);
        }
        
        match event {
            UIEvent::Input(InputEvent::Key(key_event)) => {
                match key_event.code {
                    KeyCode::Up => {
                        if let Some(&last_selected) = self.selected_indices.last() {
                            if last_selected > 0 {
                                let new_index = last_selected - 1;
                                self.select_item(new_index);
                                self.scroll_to(new_index);
                                return Ok(true);
                            }
                        }
                    }
                    KeyCode::Down => {
                        if let Some(&last_selected) = self.selected_indices.last() {
                            if last_selected + 1 < self.items.len() {
                                let new_index = last_selected + 1;
                                self.select_item(new_index);
                                self.scroll_to(new_index);
                                return Ok(true);
                            }
                        } else if !self.items.is_empty() {
                            self.select_item(0);
                            return Ok(true);
                        }
                    }
                    KeyCode::Enter => {
                        // Selection is already handled by select_item
                        return Ok(true);
                    }
                    _ => {}
                }
            }
            UIEvent::Input(InputEvent::Mouse(mouse_event)) => {
                if self.hit_test(mouse_event.column, mouse_event.row) {
                    match mouse_event.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            let bounds = self.bounds();
                            let clicked_row = mouse_event.row.saturating_sub(bounds.y) as usize;
                            let (start, _) = self.visible_range();
                            let item_index = start + clicked_row;
                            
                            if item_index < self.items.len() && self.items[item_index].enabled {
                                self.select_item(item_index);
                                return Ok(true);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        
        Ok(false)
    }
    
    fn preferred_size(&self, available: Rect) -> Rect {
        let height = (self.items.len() as u16).min(available.height);
        let width = self.items.iter()
            .map(|item| item.text.width() as u16)
            .max()
            .unwrap_or(0)
            .min(available.width);
        
        Rect::new(available.x, available.y, width, height)
    }
}

impl List {
    fn render_scrollbar(&self, buffer: &mut Buffer, style: Style) -> UIResult<()> {
        let bounds = self.bounds();
        let scrollbar_x = bounds.x + bounds.width - 1;
        
        if bounds.height < 2 {
            return Ok(());
        }
        
        let total_items = self.items.len();
        let visible_items = bounds.height as usize;
        let scrollbar_height = bounds.height as usize;
        
        // Calculate scrollbar position
        let scroll_ratio = self.scroll_offset as f32 / (total_items - visible_items) as f32;
        let thumb_position = (scroll_ratio * (scrollbar_height - 1) as f32) as usize;
        
        // Render scrollbar track
        for y in bounds.y..bounds.y + bounds.height {
            buffer.get_mut(scrollbar_x, y).set_symbol("│").set_style(style);
        }
        
        // Render scrollbar thumb
        if thumb_position < scrollbar_height {
            let thumb_y = bounds.y + thumb_position as u16;
            buffer.get_mut(scrollbar_x, thumb_y).set_symbol("█").set_style(style);
        }
        
        Ok(())
    }
}