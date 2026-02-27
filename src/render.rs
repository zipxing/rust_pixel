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
//! - `scene`: Rendering scene, compatible with text mode and graphics mode
//! - `graph`: Graphics rendering related data structures and functions
//! - `image`: Image processing functionality
//! - `symbols`: Symbol and character processing
//! - `effect`: CPU-based buffer effects (distortion, noise, blur, etc.)

pub mod adapter;
pub mod buffer;
pub mod cell;
pub mod effect;
#[cfg(graphics_mode)]
pub mod graph;
#[cfg(graphics_mode)]
mod logo_data;
pub mod image;
pub mod scene;
pub mod sprite;
pub mod style;
pub mod symbol_map;
pub mod symbols;

// re-export
pub use adapter::{Adapter, AdapterBase};
pub use buffer::{Buffer, BufferMode, Borders, BorderType, SYMBOL_LINE};
pub use cell::{
    Cell, Glyph, cellsym, cellsym_block, decode_pua, is_pua_sprite,
    TuiCharType, detect_tui_char_type, is_tui_char, is_cjk,
    PUA_BASE, PUA_END, PUA_BLOCK_SIZE,
};
#[cfg(graphics_mode)]
pub use graph::{
    init_sym_height, init_sym_width, push_render_buffer, render_border, render_logo,
    render_main_buffer, render_layers, RenderCell, PIXEL_LOGO_HEIGHT,
    PIXEL_LOGO_WIDTH, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, PIXEL_TEXTURE_FILE,
};
pub use effect::{
    // CPU Effects (Buffer级别)
    apply_distortion, BufferEffect, EffectChain, EffectParams,
    BlurEffect, FadeEffect, NoiseEffect, PixelateEffect, RippleEffect, SwirlEffect, WaveEffect,
    dissolve_chain, distortion_chain, glitch_chain,
    // GPU Effects (RenderTexture级别)
    GpuTransition, GpuBlendEffect,
};
pub use scene::Scene;
pub use sprite::Layer;
pub use style::{Color, Style};
pub use symbol_map::{ascii_to_petscii, SymbolIndex, SymbolMap, SymbolMapStats, SymbolRegion};

