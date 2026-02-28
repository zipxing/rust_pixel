// RustPixel UI Framework - Layout System
// copyright zipxing@hotmail.com 2022～2025

//! Layout system for automatic widget positioning and sizing.

use crate::util::Rect;
use crate::ui::Widget;

/// Layout direction for linear layouts
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

/// Alignment options for widgets within their allocated space
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    Start,    // Left/Top
    Center,   // Center
    End,      // Right/Bottom
    Stretch,  // Fill available space
}

/// Layout constraints for widgets
#[derive(Debug, Clone, Copy)]
pub struct LayoutConstraints {
    pub min_width: u16,
    pub max_width: u16,
    pub min_height: u16,
    pub max_height: u16,
    pub weight: f32,  // For weighted layouts
}

impl Default for LayoutConstraints {
    fn default() -> Self {
        Self {
            min_width: 0,
            max_width: u16::MAX,
            min_height: 0,
            max_height: u16::MAX,
            weight: 1.0,
        }
    }
}

/// Trait for layout algorithms
pub trait Layout {
    /// Calculate layout for a list of widgets within given bounds
    fn layout(&self, widgets: &mut [Box<dyn Widget>], bounds: Rect, constraints: &[LayoutConstraints]);
}

/// Linear layout (horizontal or vertical)
#[derive(Debug)]
pub struct LinearLayout {
    pub direction: Direction,
    pub alignment: Alignment,
    pub spacing: u16,
    pub padding: Padding,
}

#[derive(Debug, Clone, Copy)]
#[derive(Default)]
pub struct Padding {
    pub left: u16,
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
}


impl Padding {
    pub fn all(value: u16) -> Self {
        Self { left: value, top: value, right: value, bottom: value }
    }
    
    pub fn horizontal(value: u16) -> Self {
        Self { left: value, top: 0, right: value, bottom: 0 }
    }
    
    pub fn vertical(value: u16) -> Self {
        Self { left: 0, top: value, right: 0, bottom: value }
    }
}

impl Default for LinearLayout {
    fn default() -> Self {
        Self {
            direction: Direction::Vertical,
            alignment: Alignment::Stretch,
            spacing: 0,
            padding: Padding::default(),
        }
    }
}

impl LinearLayout {
    pub fn horizontal() -> Self {
        Self {
            direction: Direction::Horizontal,
            ..Default::default()
        }
    }
    
    pub fn vertical() -> Self {
        Self {
            direction: Direction::Vertical,
            ..Default::default()
        }
    }
    
    pub fn with_spacing(mut self, spacing: u16) -> Self {
        self.spacing = spacing;
        self
    }
    
    pub fn with_padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }
    
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }
}

impl Layout for LinearLayout {
    fn layout(&self, widgets: &mut [Box<dyn Widget>], bounds: Rect, constraints: &[LayoutConstraints]) {
        if widgets.is_empty() {
            return;
        }
        
        // Calculate available space after padding
        let available_width = bounds.width.saturating_sub(self.padding.left + self.padding.right);
        let available_height = bounds.height.saturating_sub(self.padding.top + self.padding.bottom);
        
        match self.direction {
            Direction::Horizontal => self.layout_horizontal(widgets, bounds, constraints, available_width, available_height),
            Direction::Vertical => self.layout_vertical(widgets, bounds, constraints, available_width, available_height),
        }
    }
}

impl LinearLayout {
    fn layout_horizontal(&self, widgets: &mut [Box<dyn Widget>], bounds: Rect, constraints: &[LayoutConstraints], available_width: u16, available_height: u16) {
        let widget_count = widgets.len();
        let total_spacing = self.spacing * (widget_count.saturating_sub(1)) as u16;
        let content_width = available_width.saturating_sub(total_spacing);
        
        // Calculate total weight
        let total_weight: f32 = constraints.iter().map(|c| c.weight).sum();
        
        let mut x = bounds.x + self.padding.left;
        let y = bounds.y + self.padding.top;
        
        for (i, (widget, constraint)) in widgets.iter_mut().zip(constraints.iter()).enumerate() {
            // Calculate width based on weight
            let widget_width = if total_weight > 0.0 {
                ((content_width as f32) * (constraint.weight / total_weight)) as u16
            } else {
                content_width / widget_count as u16
            };
            
            let widget_width = widget_width.clamp(constraint.min_width, constraint.max_width);
            
            // Calculate height based on alignment
            let widget_height = match self.alignment {
                Alignment::Stretch => available_height,
                _ => {
                    let preferred = widget.preferred_size(Rect::new(x, y, widget_width, available_height));
                    preferred.height.min(available_height)
                }
            };
            
            let widget_height = widget_height.clamp(constraint.min_height, constraint.max_height);
            
            // Calculate Y position based on alignment
            let widget_y = match self.alignment {
                Alignment::Start => y,
                Alignment::Center => y + (available_height.saturating_sub(widget_height)) / 2,
                Alignment::End => y + available_height.saturating_sub(widget_height),
                Alignment::Stretch => y,
            };
            
            widget.set_bounds(Rect::new(x, widget_y, widget_width, widget_height));
            
            x += widget_width + if i < widget_count - 1 { self.spacing } else { 0 };
        }
    }
    
    fn layout_vertical(&self, widgets: &mut [Box<dyn Widget>], bounds: Rect, constraints: &[LayoutConstraints], available_width: u16, available_height: u16) {
        let widget_count = widgets.len();
        let total_spacing = self.spacing * (widget_count.saturating_sub(1)) as u16;
        let content_height = available_height.saturating_sub(total_spacing);
        
        // Calculate total weight
        let total_weight: f32 = constraints.iter().map(|c| c.weight).sum();
        
        let x = bounds.x + self.padding.left;
        let mut y = bounds.y + self.padding.top;
        
        for (i, (widget, constraint)) in widgets.iter_mut().zip(constraints.iter()).enumerate() {
            // Calculate height based on alignment
            let widget_height = match self.alignment {
                Alignment::Stretch => {
                    // Use weight-based allocation for stretch
                    if total_weight > 0.0 {
                        ((content_height as f32) * (constraint.weight / total_weight)) as u16
                    } else {
                        content_height / widget_count as u16
                    }
                }
                _ => {
                    // Use preferred size for non-stretch alignments
                    let preferred = widget.preferred_size(Rect::new(x, y, available_width, content_height));
                    preferred.height.min(content_height)
                }
            };
            
            let widget_height = widget_height.clamp(constraint.min_height, constraint.max_height);
            
            // Calculate width based on alignment
            let widget_width = match self.alignment {
                Alignment::Stretch => available_width,
                _ => {
                    let preferred = widget.preferred_size(Rect::new(x, y, available_width, widget_height));
                    preferred.width.min(available_width)
                }
            };
            
            let widget_width = widget_width.clamp(constraint.min_width, constraint.max_width);
            
            // Calculate X position based on alignment
            let widget_x = match self.alignment {
                Alignment::Start => x,
                Alignment::Center => x + (available_width.saturating_sub(widget_width)) / 2,
                Alignment::End => x + available_width.saturating_sub(widget_width),
                Alignment::Stretch => x,
            };
            
            widget.set_bounds(Rect::new(widget_x, y, widget_width, widget_height));
            
            y += widget_height + if i < widget_count - 1 { self.spacing } else { 0 };
        }
    }
}

/// Grid layout for arranging widgets in a grid
#[derive(Debug)]
pub struct GridLayout {
    pub rows: u16,
    pub cols: u16,
    pub spacing: u16,
    pub padding: Padding,
}

impl GridLayout {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self {
            rows,
            cols,
            spacing: 0,
            padding: Padding::default(),
        }
    }
    
    pub fn with_spacing(mut self, spacing: u16) -> Self {
        self.spacing = spacing;
        self
    }
    
    pub fn with_padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }
}

impl Layout for GridLayout {
    fn layout(&self, widgets: &mut [Box<dyn Widget>], bounds: Rect, _constraints: &[LayoutConstraints]) {
        if widgets.is_empty() || self.rows == 0 || self.cols == 0 {
            return;
        }
        
        let available_width = bounds.width.saturating_sub(self.padding.left + self.padding.right);
        let available_height = bounds.height.saturating_sub(self.padding.top + self.padding.bottom);
        
        let total_h_spacing = self.spacing * (self.cols.saturating_sub(1));
        let total_v_spacing = self.spacing * (self.rows.saturating_sub(1));
        
        let cell_width = (available_width.saturating_sub(total_h_spacing)) / self.cols;
        let cell_height = (available_height.saturating_sub(total_v_spacing)) / self.rows;
        
        for (i, widget) in widgets.iter_mut().enumerate() {
            let row = (i as u16) / self.cols;
            let col = (i as u16) % self.cols;
            
            if row >= self.rows {
                break; // No more space in grid
            }
            
            let x = bounds.x + self.padding.left + col * (cell_width + self.spacing);
            let y = bounds.y + self.padding.top + row * (cell_height + self.spacing);
            
            widget.set_bounds(Rect::new(x, y, cell_width, cell_height));
        }
    }
}

/// Free-form layout where widgets position themselves
#[derive(Debug, Default)]
pub struct FreeLayout;

impl Layout for FreeLayout {
    fn layout(&self, _widgets: &mut [Box<dyn Widget>], _bounds: Rect, _constraints: &[LayoutConstraints]) {
        // Widgets position themselves - no automatic layout
    }
}