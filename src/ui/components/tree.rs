// RustPixel UI Framework - Tree Component
// copyright zipxing@hotmail.com 2022～2025

//! Tree component for hierarchical data display.

use crate::context::Context;
use crate::render::Buffer;
use crate::render::style::{Color, Style};
use crate::util::Rect;
use crate::ui::{
    Widget, BaseWidget, WidgetId, WidgetState, UIEvent, UIResult,
    next_widget_id
};
use crate::impl_widget_base;
use crate::event::{Event as InputEvent, KeyCode, MouseEventKind, MouseButton};
use unicode_width::UnicodeWidthStr;
use std::collections::HashMap;

/// Tree node identifier
pub type NodeId = usize;

/// Tree node data
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: NodeId,
    pub text: String,
    pub data: Option<String>,
    pub expanded: bool,
    pub children: Vec<NodeId>,
    pub parent: Option<NodeId>,
}

impl TreeNode {
    pub fn new(id: NodeId, text: &str) -> Self {
        Self {
            id,
            text: text.to_string(),
            data: None,
            expanded: true,
            children: Vec::new(),
            parent: None,
        }
    }
    
    pub fn with_data(mut self, data: &str) -> Self {
        self.data = Some(data.to_string());
        self
    }
    
    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }
}

/// Tree component for displaying hierarchical data
pub struct Tree {
    base: BaseWidget,
    nodes: HashMap<NodeId, TreeNode>,
    root_nodes: Vec<NodeId>,
    selected_node: Option<NodeId>,
    scroll_offset: usize,
    next_node_id: NodeId,
    show_lines: bool,
    on_selection_changed: Option<Box<dyn FnMut(Option<NodeId>) + Send>>,
    on_node_expanded: Option<Box<dyn FnMut(NodeId, bool) + Send>>,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            base: BaseWidget::new(next_widget_id()),
            nodes: HashMap::new(),
            root_nodes: Vec::new(),
            selected_node: None,
            scroll_offset: 0,
            next_node_id: 1,
            show_lines: true,
            on_selection_changed: None,
            on_node_expanded: None,
        }
    }
    
    pub fn with_style(mut self, style: Style) -> Self {
        self.base.style = style;
        self
    }
    
    pub fn with_lines(mut self, show_lines: bool) -> Self {
        self.show_lines = show_lines;
        self
    }
    
    pub fn on_selection_changed<F>(mut self, callback: F) -> Self
    where
        F: FnMut(Option<NodeId>) + Send + 'static,
    {
        self.on_selection_changed = Some(Box::new(callback));
        self
    }
    
    pub fn on_node_expanded<F>(mut self, callback: F) -> Self
    where
        F: FnMut(NodeId, bool) + Send + 'static,
    {
        self.on_node_expanded = Some(Box::new(callback));
        self
    }
    
    pub fn add_root_node(&mut self, text: &str) -> NodeId {
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        
        let node = TreeNode::new(node_id, text);
        self.nodes.insert(node_id, node);
        self.root_nodes.push(node_id);
        self.mark_dirty();
        
        node_id
    }
    
    pub fn add_child_node(&mut self, parent_id: NodeId, text: &str) -> Option<NodeId> {
        if !self.nodes.contains_key(&parent_id) {
            return None;
        }
        
        let node_id = self.next_node_id;
        self.next_node_id += 1;
        
        let mut node = TreeNode::new(node_id, text);
        node.parent = Some(parent_id);
        
        self.nodes.insert(node_id, node);
        
        // Add to parent's children
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.push(node_id);
        }
        
        self.mark_dirty();
        Some(node_id)
    }
    
    pub fn remove_node(&mut self, node_id: NodeId) -> bool {
        if let Some(node) = self.nodes.remove(&node_id) {
            // Remove from parent's children or root nodes
            if let Some(parent_id) = node.parent {
                if let Some(parent) = self.nodes.get_mut(&parent_id) {
                    parent.children.retain(|&id| id != node_id);
                }
            } else {
                self.root_nodes.retain(|&id| id != node_id);
            }
            
            // Remove all children recursively
            let children = node.children.clone();
            for child_id in children {
                self.remove_node(child_id);
            }
            
            // Update selection if this node was selected
            if self.selected_node == Some(node_id) {
                self.selected_node = None;
                self.notify_selection_changed();
            }
            
            self.mark_dirty();
            true
        } else {
            false
        }
    }
    
    pub fn expand_node(&mut self, node_id: NodeId) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            if !node.expanded {
                node.expanded = true;
                self.mark_dirty();
                self.notify_node_expanded(node_id, true);
            }
        }
    }
    
    pub fn collapse_node(&mut self, node_id: NodeId) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            if node.expanded {
                node.expanded = false;
                self.mark_dirty();
                self.notify_node_expanded(node_id, false);
            }
        }
    }
    
    pub fn toggle_node(&mut self, node_id: NodeId) {
        if let Some(node) = self.nodes.get(&node_id) {
            let expanded = node.expanded;
            let _ = node; // Release borrow
            
            if expanded {
                self.collapse_node(node_id);
            } else {
                self.expand_node(node_id);
            }
        }
    }
    
    pub fn select_node(&mut self, node_id: Option<NodeId>) {
        if self.selected_node != node_id {
            self.selected_node = node_id;
            self.mark_dirty();
            self.notify_selection_changed();
        }
    }
    
    pub fn selected_node(&self) -> Option<NodeId> {
        self.selected_node
    }
    
    pub fn get_node(&self, node_id: NodeId) -> Option<&TreeNode> {
        self.nodes.get(&node_id)
    }
    
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.root_nodes.clear();
        self.selected_node = None;
        self.scroll_offset = 0;
        self.next_node_id = 1;
        self.mark_dirty();
    }
    
    fn notify_selection_changed(&mut self) {
        if let Some(ref mut callback) = self.on_selection_changed {
            callback(self.selected_node);
        }
    }
    
    fn notify_node_expanded(&mut self, node_id: NodeId, expanded: bool) {
        if let Some(ref mut callback) = self.on_node_expanded {
            callback(node_id, expanded);
        }
    }
    
    fn get_visible_nodes(&self) -> Vec<(NodeId, usize)> {
        let mut visible = Vec::new();
        
        for &root_id in &self.root_nodes {
            self.collect_visible_nodes(root_id, 0, &mut visible);
        }
        
        visible
    }
    
    fn collect_visible_nodes(&self, node_id: NodeId, depth: usize, visible: &mut Vec<(NodeId, usize)>) {
        visible.push((node_id, depth));
        
        if let Some(node) = self.nodes.get(&node_id) {
            if node.expanded {
                for &child_id in &node.children {
                    self.collect_visible_nodes(child_id, depth + 1, visible);
                }
            }
        }
    }
}

impl Widget for Tree {
    impl_widget_base!(Tree, base);
    
    fn render(&self, buffer: &mut Buffer, _ctx: &Context) -> UIResult<()> {
        if !self.state().visible {
            return Ok(());
        }
        
        let bounds = self.bounds();
        if bounds.width == 0 || bounds.height == 0 {
            return Ok(());
        }
        
        // Use base style for now
        let tree_style = self.base.style;
        
        // Clear background
        for y in bounds.y..bounds.y + bounds.height {
            for x in bounds.x..bounds.x + bounds.width {
                buffer.get_mut(x, y).set_style(tree_style).set_symbol(" ");
            }
        }
        
        // Get visible nodes
        let visible_nodes = self.get_visible_nodes();
        let visible_count = bounds.height as usize;
        
        // Render visible nodes
        for (display_index, &(node_id, depth)) in visible_nodes.iter()
            .skip(self.scroll_offset)
            .take(visible_count)
            .enumerate() {
            
            let y = bounds.y + display_index as u16;
            self.render_node(buffer, node_id, depth, y, tree_style)?;
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
                        let visible_nodes = self.get_visible_nodes();
                        if let Some(current_index) = self.selected_node.and_then(|id| {
                            visible_nodes.iter().position(|(node_id, _)| *node_id == id)
                        }) {
                            if current_index > 0 {
                                let (new_node_id, _) = visible_nodes[current_index - 1];
                                self.select_node(Some(new_node_id));
                                return Ok(true);
                            }
                        }
                    }
                    KeyCode::Down => {
                        let visible_nodes = self.get_visible_nodes();
                        if let Some(current_index) = self.selected_node.and_then(|id| {
                            visible_nodes.iter().position(|(node_id, _)| *node_id == id)
                        }) {
                            if current_index + 1 < visible_nodes.len() {
                                let (new_node_id, _) = visible_nodes[current_index + 1];
                                self.select_node(Some(new_node_id));
                                return Ok(true);
                            }
                        } else if !visible_nodes.is_empty() {
                            let (first_node_id, _) = visible_nodes[0];
                            self.select_node(Some(first_node_id));
                            return Ok(true);
                        }
                    }
                    KeyCode::Right => {
                        if let Some(node_id) = self.selected_node {
                            self.expand_node(node_id);
                            return Ok(true);
                        }
                    }
                    KeyCode::Left => {
                        if let Some(node_id) = self.selected_node {
                            self.collapse_node(node_id);
                            return Ok(true);
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(node_id) = self.selected_node {
                            self.toggle_node(node_id);
                            return Ok(true);
                        }
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
                            let visible_nodes = self.get_visible_nodes();
                            
                            if let Some(&(node_id, _)) = visible_nodes.get(self.scroll_offset + clicked_row) {
                                self.select_node(Some(node_id));
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
        let visible_nodes = self.get_visible_nodes();
        let height = (visible_nodes.len() as u16).min(available.height);
        
        let max_width = visible_nodes.iter()
            .map(|(node_id, depth)| {
                if let Some(node) = self.nodes.get(node_id) {
                    let indent = if self.show_lines { depth * 2 + 3 } else { depth * 2 };
                    indent + node.text.width()
                } else {
                    0
                }
            })
            .max()
            .unwrap_or(0) as u16;
        
        let width = max_width.min(available.width);
        
        Rect::new(available.x, available.y, width, height)
    }
}

impl Tree {
    fn render_node(&self, buffer: &mut Buffer, node_id: NodeId, depth: usize, y: u16, base_style: Style) -> UIResult<()> {
        if let Some(node) = self.nodes.get(&node_id) {
            let bounds = self.bounds();
            let is_selected = self.selected_node == Some(node_id);
            
            let node_style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::White)
            } else {
                base_style
            };
            
            let mut x = bounds.x;
            
            // Render indentation and tree lines
            if self.show_lines && depth > 0 {
                for _i in 0..depth {
                    if x + 1 < bounds.x + bounds.width {
                        buffer.set_string(x, y, "  ", base_style);
                        x += 2;
                    }
                }
            } else {
                x += (depth * 2) as u16;
            }
            
            // Render expand/collapse indicator
            if !node.children.is_empty() {
                let indicator = if node.expanded { "▼" } else { "▶" };
                if x < bounds.x + bounds.width {
                    buffer.set_string(x, y, indicator, node_style);
                    x += 1;
                }
            } else if x < bounds.x + bounds.width {
                buffer.set_string(x, y, " ", node_style);
                x += 1;
            }
            
            // Render node text
            if x < bounds.x + bounds.width {
                let available_width = (bounds.x + bounds.width).saturating_sub(x) as usize;
                let display_text = if node.text.width() > available_width {
                    node.text.chars().take(available_width).collect()
                } else {
                    node.text.clone()
                };
                
                buffer.set_string(x, y, &display_text, node_style);
            }
        }
        
        Ok(())
    }
}