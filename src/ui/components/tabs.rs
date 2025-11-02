// RustPixel UI Framework - Tabs Component
// copyright zipxing@hotmail.com 2022～2025

//! Tabs component - character-cell tab view with a simple tab bar and page area.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::Style;
use crate::util::Rect;
use crate::ui::{
    Widget, Container, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id,
};
use crate::impl_widget_base;

/// Tabs widget: a container with a tab bar (single row) and a content area for the selected page.
pub struct Tabs {
    base: BaseWidget,
    titles: Vec<String>,
    pages: Vec<Box<dyn Widget>>, // one page per title
    selected: usize,
    tab_spacing: u16,
    tab_style: Style,
    tab_active_style: Style,
}

impl Tabs {
    pub fn new() -> Self {
        let id = next_widget_id();
        Self {
            base: BaseWidget::new(id),
            titles: Vec::new(),
            pages: Vec::new(),
            selected: 0,
            tab_spacing: 1,
            tab_style: Style::default(),
            tab_active_style: Style::default(),
        }
    }

    pub fn with_bounds(mut self, bounds: Rect) -> Self {
        self.base.bounds = bounds;
        self
    }

    pub fn with_tab_style(mut self, style: Style) -> Self {
        self.tab_style = style;
        self
    }

    pub fn with_tab_active_style(mut self, style: Style) -> Self {
        self.tab_active_style = style;
        self
    }

    pub fn with_tab_spacing(mut self, spacing: u16) -> Self {
        self.tab_spacing = spacing;
        self
    }

    pub fn with_style(mut self, base: Style, active: Style) -> Self {
        self.tab_style = base;
        self.tab_active_style = active;
        self
    }

    pub fn add_tab(&mut self, title: &str, page: Box<dyn Widget>) {
        self.titles.push(title.to_string());
        self.pages.push(page);
        if self.pages.len() == 1 {
            self.selected = 0;
        }
        self.mark_dirty();
        // Layout immediately if we have bounds
        if self.bounds().width > 0 && self.bounds().height > 0 {
            self.layout_selected_page();
        }
    }

    pub fn set_selected(&mut self, index: usize) {
        if index < self.pages.len() && self.selected != index {
            self.selected = index;
            self.mark_dirty();
            self.layout_selected_page();
        }
    }

    fn tabbar_area(&self) -> Rect {
        let b = self.bounds();
        Rect::new(b.x, b.y, b.width, if b.height > 0 { 1 } else { 0 })
    }

    fn content_area(&self) -> Rect {
        let b = self.bounds();
        if b.height > 1 {
            Rect::new(b.x, b.y + 1, b.width, b.height - 1)
        } else {
            Rect::new(b.x, b.y, 0, 0)
        }
    }

    fn layout_selected_page(&mut self) {
        let area = self.content_area();
        if let Some(page) = self.pages.get_mut(self.selected) {
            page.set_bounds(area);
            // If the page is a Panel, trigger its layout
            if let Some(panel) = page.as_any_mut().downcast_mut::<crate::ui::Panel>() {
                panel.layout();
            }
        }
    }

    fn render_tabbar(&self, buffer: &mut Buffer) {
        let bar = self.tabbar_area();
        if bar.width == 0 || bar.height == 0 { return; }

        // Clear tab bar line
        for x in bar.x..bar.x + bar.width {
            buffer.get_mut(x, bar.y).set_symbol(" ").set_style(self.tab_style);
        }

        // Draw titles from left to right
        let mut x = bar.x;
        for (i, title) in self.titles.iter().enumerate() {
            // Surround active tab with [ ] and apply active style
            let display = if i == self.selected { format!("[{}]", title) } else { title.clone() };
            let style = if i == self.selected { self.tab_active_style } else { self.tab_style };

            // Truncate if exceeds available space
            if x >= bar.x + bar.width { break; }
            let max_len = (bar.x + bar.width - x) as usize;
            let text = if display.len() > max_len { &display[..max_len] } else { &display };
            buffer.set_string(x, bar.y, text, style);

            // Advance position
            x = x.saturating_add(text.len() as u16 + self.tab_spacing);
        }
    }
}

impl Widget for Tabs {
    impl_widget_base!(Tabs, base);

    fn render(&self, buffer: &mut Buffer, ctx: &Context) -> UIResult<()> {
        if !self.state().visible { return Ok(()); }
        let b = self.bounds();
        if b.width == 0 || b.height == 0 { return Ok(()); }

        // Draw tab bar
        self.render_tabbar(buffer);

        // Render selected page
        if let Some(page) = self.pages.get(self.selected) {
            page.render(buffer, ctx)?;
        }

        Ok(())
    }

    fn handle_event(&mut self, event: &UIEvent, _ctx: &mut Context) -> UIResult<bool> {
        // Process raw input to detect tab clicks; children handle their own events
        if let UIEvent::Input(input) = event {
            if let crate::event::Event::Mouse(mev) = input {
                if let crate::event::MouseEventKind::Down(crate::event::MouseButton::Left) = mev.kind {
                    let bar = self.tabbar_area();
                    if mev.row == bar.y && mev.column >= bar.x && mev.column < bar.x + bar.width {
                        // Hit test by scanning titles rendered positions
                        let mut x = bar.x;
                        for (i, title) in self.titles.iter().enumerate() {
                            let display = if i == self.selected { format!("[{}]", title) } else { title.clone() };
                            let w = display.len() as u16;
                            if mev.column >= x && mev.column < x + w {
                                self.set_selected(i);
                                return Ok(true);
                            }
                            x = x.saturating_add(w + self.tab_spacing);
                        }
                    }
                }
            }
        }

        // Forward events to selected page
        if let Some(page) = self.pages.get_mut(self.selected) {
            if page.handle_event(event, _ctx)? { return Ok(true); }
        }

        Ok(false)
    }

    fn preferred_size(&self, available: Rect) -> Rect {
        // Tabs prefer to fill available space; tab bar consumes one row
        available
    }
}

impl Container for Tabs {
    fn add_child(&mut self, child: Box<dyn Widget>) {
        // If added via generic API, create a default title
        let title = format!("Tab {}", self.pages.len() + 1);
        self.add_tab(&title, child);
    }

    fn remove_child(&mut self, id: WidgetId) -> Option<Box<dyn Widget>> {
        if let Some(idx) = self.pages.iter().position(|p| p.id() == id) {
            self.titles.remove(idx);
            self.mark_dirty();
            Some(self.pages.remove(idx))
        } else {
            None
        }
    }

    fn get_child(&self, id: WidgetId) -> Option<&dyn Widget> {
        self.pages.iter().find(|c| c.id() == id).map(|c| c.as_ref())
    }

    fn get_child_mut(&mut self, id: WidgetId) -> Option<&mut dyn Widget> {
        self.pages.iter_mut().find(|c| c.id() == id).map(|c| c.as_mut())
    }

    fn children(&self) -> &[Box<dyn Widget>] {
        &self.pages
    }

    fn children_mut(&mut self) -> &mut Vec<Box<dyn Widget>> {
        &mut self.pages
    }

    fn layout(&mut self) {
        self.layout_selected_page();
    }
}


