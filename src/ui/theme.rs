// RustPixel UI Framework - Theme System
// copyright zipxing@hotmail.com 2022～2025

//! Theme and styling system for UI components.

use crate::render::style::{Color, Style, Modifier};
use std::collections::HashMap;

/// Theme definition containing styles for different widget states
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub styles: HashMap<String, ComponentStyle>,
}

/// Style for a specific component and its states
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct ComponentStyle {
    pub normal: Style,
    pub hovered: Style,
    pub focused: Style,
    pub pressed: Style,
    pub disabled: Style,
}


impl ComponentStyle {
    pub fn new(base_style: Style) -> Self {
        Self {
            normal: base_style,
            hovered: base_style,
            focused: base_style,
            pressed: base_style,
            disabled: base_style,
        }
    }
    
    pub fn with_hover(mut self, style: Style) -> Self {
        self.hovered = style;
        self
    }
    
    pub fn with_focus(mut self, style: Style) -> Self {
        self.focused = style;
        self
    }
    
    pub fn with_pressed(mut self, style: Style) -> Self {
        self.pressed = style;
        self
    }
    
    pub fn with_disabled(mut self, style: Style) -> Self {
        self.disabled = style;
        self
    }
    
    /// Get style for current widget state
    pub fn get_style(&self, focused: bool, hovered: bool, pressed: bool, enabled: bool) -> Style {
        if !enabled {
            self.disabled
        } else if pressed {
            self.pressed
        } else if focused {
            self.focused
        } else if hovered {
            self.hovered
        } else {
            self.normal
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            styles: HashMap::new(),
        }
    }
    
    /// Set style for a component
    pub fn set_style(&mut self, component: &str, style: ComponentStyle) {
        self.styles.insert(component.to_string(), style);
    }
    
    /// Get style for a component
    pub fn get_style(&self, component: &str) -> Option<&ComponentStyle> {
        self.styles.get(component)
    }
    
    /// Create a dark theme
    pub fn dark() -> Self {
        let mut theme = Self::new("dark");
        
        // Button styles
        let button_style = ComponentStyle::new(
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
        )
        .with_hover(
            Style::default()
                .fg(Color::White)
                .bg(Color::Gray)
        )
        .with_focus(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Gray)
                .add_modifier(Modifier::BOLD)
        )
        .with_pressed(
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
        )
        .with_disabled(
            Style::default()
                .fg(Color::DarkGray)
                .bg(Color::Black)
        );
        
        theme.set_style("button", button_style);
        
        // Label styles
        let label_style = ComponentStyle::new(
            Style::default()
                .fg(Color::White)
                .bg(Color::Reset)
        )
        .with_disabled(
            Style::default()
                .fg(Color::DarkGray)
                .bg(Color::Reset)
        );
        
        theme.set_style("label", label_style);
        
        // TextBox styles
        let textbox_style = ComponentStyle::new(
            Style::default()
                .fg(Color::White)
                .bg(Color::Black)
        )
        .with_focus(
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::UNDERLINED)
        )
        .with_disabled(
            Style::default()
                .fg(Color::DarkGray)
                .bg(Color::Black)
        );
        
        theme.set_style("textbox", textbox_style);
        
        // Panel styles
        let panel_style = ComponentStyle::new(
            Style::default()
                .fg(Color::White)
                .bg(Color::Reset)
        );
        
        theme.set_style("panel", panel_style);
        
        // List styles
        let list_style = ComponentStyle::new(
            Style::default()
                .fg(Color::White)
                .bg(Color::Black)
        );
        
        theme.set_style("list", list_style);
        
        // List item styles
        let listitem_style = ComponentStyle::new(
            Style::default()
                .fg(Color::White)
                .bg(Color::Reset)
        )
        .with_hover(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Gray)
        )
        .with_focus(
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        );
        
        theme.set_style("listitem", listitem_style);
        
        theme
    }
    
    /// Create a light theme
    pub fn light() -> Self {
        let mut theme = Self::new("light");
        
        // Button styles
        let button_style = ComponentStyle::new(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Gray)
        )
        .with_hover(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Gray)
        )
        .with_focus(
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        )
        .with_pressed(
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
        )
        .with_disabled(
            Style::default()
                .fg(Color::Gray)
                .bg(Color::Gray)
        );
        
        theme.set_style("button", button_style);
        
        // Label styles
        let label_style = ComponentStyle::new(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Reset)
        )
        .with_disabled(
            Style::default()
                .fg(Color::Gray)
                .bg(Color::Reset)
        );
        
        theme.set_style("label", label_style);
        
        // TextBox styles
        let textbox_style = ComponentStyle::new(
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
        )
        .with_focus(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Gray)
                .add_modifier(Modifier::UNDERLINED)
        )
        .with_disabled(
            Style::default()
                .fg(Color::Gray)
                .bg(Color::White)
        );
        
        theme.set_style("textbox", textbox_style);
        
        // Panel styles
        let panel_style = ComponentStyle::new(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Reset)
        );
        
        theme.set_style("panel", panel_style);
        
        // List styles
        let list_style = ComponentStyle::new(
            Style::default()
                .fg(Color::Black)
                .bg(Color::White)
        );
        
        theme.set_style("list", list_style);
        
        // List item styles
        let listitem_style = ComponentStyle::new(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Reset)
        )
        .with_hover(
            Style::default()
                .fg(Color::White)
                .bg(Color::Gray)
        )
        .with_focus(
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        );
        
        theme.set_style("listitem", listitem_style);
        
        theme
    }
    
    /// Create a terminal-friendly theme
    pub fn terminal() -> Self {
        let mut theme = Self::new("terminal");
        
        // Use basic colors that work well in terminals
        let button_style = ComponentStyle::new(
            Style::default()
                .fg(Color::Green)
                .bg(Color::Reset)
                .add_modifier(Modifier::BOLD)
        )
        .with_hover(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
        )
        .with_focus(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Reset)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        )
        .with_pressed(
            Style::default()
                .fg(Color::White)
                .bg(Color::Green)
        )
        .with_disabled(
            Style::default()
                .fg(Color::DarkGray)
                .bg(Color::Reset)
        );
        
        theme.set_style("button", button_style);
        
        // Simple label style
        let label_style = ComponentStyle::new(
            Style::default()
                .fg(Color::Reset)
                .bg(Color::Reset)
        );
        
        theme.set_style("label", label_style);
        
        theme
    }
}

/// Global theme manager
pub struct ThemeManager {
    current_theme: Theme,
    available_themes: HashMap<String, Theme>,
}

impl Default for ThemeManager {
    fn default() -> Self {
        let mut manager = Self {
            current_theme: Theme::dark(),
            available_themes: HashMap::new(),
        };
        
        // Register built-in themes
        manager.register_theme(Theme::dark());
        manager.register_theme(Theme::light());
        manager.register_theme(Theme::terminal());
        
        manager
    }
}

impl ThemeManager {
    pub fn new() -> Self {
        Default::default()
    }
    
    /// Register a new theme
    pub fn register_theme(&mut self, theme: Theme) {
        self.available_themes.insert(theme.name.clone(), theme);
    }
    
    /// Set the current theme by name
    pub fn set_theme(&mut self, name: &str) -> Result<(), String> {
        if let Some(theme) = self.available_themes.get(name) {
            self.current_theme = theme.clone();
            Ok(())
        } else {
            Err(format!("Theme '{}' not found", name))
        }
    }
    
    /// Get the current theme
    pub fn current_theme(&self) -> &Theme {
        &self.current_theme
    }
    
    /// Get list of available theme names
    pub fn available_themes(&self) -> Vec<&String> {
        self.available_themes.keys().collect()
    }
    
    /// Get style for a component in the current theme
    pub fn get_component_style(&self, component: &str) -> Option<&ComponentStyle> {
        self.current_theme.get_style(component)
    }
}