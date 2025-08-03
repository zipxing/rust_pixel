// RustPixel UI Framework
// copyright zipxing@hotmail.com 2022～2025

//! # RustPixel UI Framework
//!
//! A simple and practical UI framework based on rust_pixel's character rendering engine.
//! Designed for building editor applications, gallery viewers, and other character-based UIs.
//!
//! ## Core Concepts
//!
//! - **Widget**: Basic UI component trait
//! - **Layout**: Automatic positioning and sizing system
//! - **Event**: Unified input event handling
//! - **Theme**: Style and appearance management
//!
//! ## Example Usage
//!
//! ```rust
//! use rust_pixel::ui::*;
//!
//! let mut app = UIApp::new();
//! let mut panel = Panel::new(Rect::new(0, 0, 80, 25));
//! 
//! panel.add_child(Box::new(
//!     Label::new("Hello UI Framework!")
//!         .style(Style::default().fg(Color::Green))
//! ));
//! 
//! panel.add_child(Box::new(
//!     Button::new("Click Me")
//!         .on_click(|_| println!("Button clicked!"))
//! ));
//! 
//! app.set_root(Box::new(panel));
//! app.run();
//! ```

pub mod widget;
pub mod layout;
pub mod event;
pub mod theme;
pub mod components;
pub mod app;

// Re-exports for convenience
pub use widget::*;
pub use layout::*;
pub use event::*;
pub use theme::*;
pub use components::*;
pub use app::*;





/// UI Framework result type
pub type UIResult<T> = Result<T, UIError>;

/// UI Framework error types
#[derive(Debug)]
pub enum UIError {
    /// Widget not found
    WidgetNotFound(String),
    /// Invalid layout configuration
    InvalidLayout(String),
    /// Event handling error
    EventError(String),
    /// Theme error
    ThemeError(String),
}

impl std::fmt::Display for UIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UIError::WidgetNotFound(msg) => write!(f, "Widget not found: {}", msg),
            UIError::InvalidLayout(msg) => write!(f, "Invalid layout: {}", msg),
            UIError::EventError(msg) => write!(f, "Event error: {}", msg),
            UIError::ThemeError(msg) => write!(f, "Theme error: {}", msg),
        }
    }
}

impl std::error::Error for UIError {}