// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! # Render Module
//!
//! Supports two rendering modes: text mode and graphics mode.
//!
//! ## Sub-modules
//! - `adapter`: Render adapter interfaces (crossterm, SDL, web, winit)
//! - `cell`: Basic rendering unit, i.e., a character
//! - `buffer`: Vector composed of cells, manages screen buffer
//! - `sprite`: Basic rendering component, further encapsulates buffer
//! - `style`: Defines rendering attributes such as foreground and background colors
//! - `panel`: Rendering panel, compatible with text mode and graphics mode
//! - `graph`: Graphics rendering related data structures and functions
//! - `image`: Image processing functionality
//! - `symbols`: Symbol and character processing

pub mod adapter;
pub mod buffer;
pub mod cell;
#[cfg(graphics_mode)]
pub mod graph;
pub mod image;
pub mod panel;
pub mod sprite;
pub mod style;
pub mod symbol_map;
pub mod symbols;

// re-export
pub use adapter::{Adapter, AdapterBase};
pub use buffer::Buffer;
pub use cell::Cell;
#[cfg(graphics_mode)]
pub use graph::{
    init_sym_height, init_sym_width, push_render_buffer, render_border, render_logo,
    render_main_buffer, render_pixel_sprites, RenderCell, PIXEL_LOGO, PIXEL_LOGO_HEIGHT,
    PIXEL_LOGO_WIDTH, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, PIXEL_TEXTURE_FILE,
};
pub use panel::Panel;
pub use sprite::Sprites;
pub use style::{Color, Style};
pub use symbol_map::{SymbolIndex, SymbolMap, SymbolMapStats, SymbolRegion};
