// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # Render Adapter Module
//! 
//! This module defines the render adapter architecture for RustPixel, providing unified
//! rendering interfaces across different platforms and rendering backends.
//! 
//! ## Supported Rendering Backends
//! - **SDL**: Desktop platform based on SDL2 library
//! - **Winit**: Cross-platform window management with OpenGL
//! - **Web**: WebGL-based browser rendering 
//! - **Crossterm**: Terminal text-mode rendering
//!
//! ## Architecture Overview
//! 
//! Based on the principle.md design document, RustPixel uses a trait-based adapter
//! pattern to abstract different rendering backends:
//! 
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Game Loop (Frame-based)                  │
//! │  ┌─────────────┐        ┌─────────────┐                     │
//! │  │   Model     │◄──────►│   Render    │                     │
//! │  │  (Logic)    │ Events │ (Graphics)  │                     │
//! │  └─────────────┘        └─────────────┘                     │
//! │         │                       │                           │
//! │         ▼                       ▼                           │
//! │  ┌─────────────────────────────────────────────────────────┐ │
//! │  │              Message Bus + Timer                        │ │
//! │  └─────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//!          │
//!          ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  Adapter Interface                          │
//! │  ┌─────────────┬─────────────┬─────────────┬─────────────┐  │
//! │  │     SDL     │    Winit    │     Web     │  Crossterm  │  │
//! │  │   Adapter   │   Adapter   │   Adapter   │   Adapter   │  │
//! │  └─────────────┴─────────────┴─────────────┴─────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Rendering Pipeline
//! 
//! The rendering system supports two modes based on principle.md:
//! 
//! ### Text Mode Rendering
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                       Panel                                 │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
//! │  │   Layer 1   │  │   Layer 2   │  │   Layer N   │         │
//! │  │ (Sprites)   │  │ (Sprites)   │  │ (Sprites)   │         │
//! │  └─────────────┘  └─────────────┘  └─────────────┘         │
//! │         │                │                │                 │
//! │         └────────────────┼────────────────┘                 │
//! │                          ▼                                  │
//! │              ┌─────────────────────┐                        │
//! │              │    Main Buffer      │                        │
//! │              │    (Characters)     │                        │
//! │              └─────────────────────┘                        │
//! │                          │                                  │
//! │                          ▼                                  │
//! │              ┌─────────────────────┐                        │
//! │              │  Double Buffering   │                        │
//! │              │   + Diff Check      │                        │
//! │              └─────────────────────┘                        │
//! │                          │                                  │
//! │                          ▼                                  │
//! │              ┌─────────────────────┐                        │
//! │              │      Terminal       │                        │
//! │              │    (Crossterm)      │                        │
//! │              └─────────────────────┘                        │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//! 
//! ### Graphics Mode Rendering
//! 
//! Graphics mode uses a more complex two-pass rendering pipeline:
//! 
//! #### Pass 1: Buffer to RenderCell Conversion
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   Graphics Mode Pass 1                      │
//! │                                                             │
//! │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
//! │  │   Buffer    │    │   Sprites   │    │    Logo     │     │
//! │  │(Characters) │    │ (Objects)   │    │ (Startup)   │     │
//! │  └─────────────┘    └─────────────┘    └─────────────┘     │
//! │         │                 │                    │           │
//! │         └─────────────────┼────────────────────┘           │
//! │                           ▼                                │
//! │              ┌─────────────────────┐                       │
//! │              │   RenderCell Array  │                       │
//! │              │  (GPU-ready Data)   │                       │
//! │              └─────────────────────┘                       │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//! 
//! #### Pass 2: OpenGL Rendering Pipeline
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   OpenGL Render Pipeline                    │
//! │                                                             │
//! │  ┌─────────────────────┐                                    │
//! │  │    RenderBuffer     │                                    │
//! │  │  (Vec<RenderCell>)  │                                    │
//! │  └─────────────────────┘                                    │
//! │             │                                               │
//! │             ▼                                               │
//! │  ┌─────────────────────┐    ┌─────────────────────┐        │
//! │  │  Symbols Shader     │    │  Transition Shader  │        │
//! │  │ (Instanced Render)  │    │   (Effect Mixing)   │        │
//! │  └─────────────────────┘    └─────────────────────┘        │
//! │             │                           │                  │
//! │             ▼                           ▼                  │
//! │  ┌─────────────────────┐    ┌─────────────────────┐        │
//! │  │   Render Texture    │    │   Render Texture    │        │
//! │  │      (Main)         │    │    (Transition)     │        │
//! │  └─────────────────────┘    └─────────────────────┘        │
//! │             │                           │                  │
//! │             └─────────────┬─────────────┘                  │
//! │                           ▼                                │
//! │                ┌─────────────────────┐                     │
//! │                │  General2D Shader   │                     │
//! │                │  (Final Composite)  │                     │
//! │                └─────────────────────┘                     │
//! │                           │                                │
//! │                           ▼                                │
//! │                ┌─────────────────────┐                     │
//! │                │       Screen        │                     │
//! │                │    (Framebuffer)    │                     │
//! │                └─────────────────────┘                     │
//! └─────────────────────────────────────────────────────────────┘
//! ```

#![allow(unused_variables)]
use crate::{
    event::Event,
    render::{buffer::Buffer, sprite::Sprites},
    util::{Rand, Rect},
};
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
use crate::{
    render::adapter::gl::{color::GlColor, pixel::GlPixel, transform::GlTransform},
    render::style::Color,
    util::{ARect, PointF32, PointI32, PointU16},
    LOGO_FRAME,
};
use std::any::Any;
use std::sync::OnceLock;
use std::time::Duration;
// use log::info;

/// OpenGL rendering subsystem for winit, SDL and web modes
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub mod gl;

/// WGPU rendering subsystem - modern GPU API for cross-platform rendering
#[cfg(feature = "wgpu")]
pub mod wgpu;

/// SDL adapter module - Desktop rendering backend based on SDL2
#[cfg(all(feature = "sdl", not(target_arch = "wasm32")))]
pub mod sdl;

/// Web adapter module - WebGL-based browser rendering backend
#[cfg(target_arch = "wasm32")]
pub mod web;

/// Winit adapter module - Cross-platform window management with OpenGL
#[cfg(all(feature = "winit", not(target_arch = "wasm32")))]
pub mod winit;

/// Crossterm adapter module - Terminal-based text mode rendering
#[cfg(not(any(
    feature = "sdl",
    feature = "winit",
    target_os = "android",
    target_os = "ios",
    target_arch = "wasm32"
)))]
pub mod cross;

/// Path to the symbols texture file
/// 
/// The symbols texture contains 8x8 blocks where each block contains 16x16 symbols,
/// totaling 128 × 128 symbols. This texture serves as a character atlas for rendering
/// text and symbols in graphics mode.
/// 
/// Layout:
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                   Symbols Texture (128×128)                 │
/// │  ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐         │
/// │  │Block│Block│Block│Block│Block│Block│Block│Block│         │
/// │  │ 0,0 │ 1,0 │ 2,0 │ 3,0 │ 4,0 │ 5,0 │ 6,0 │ 7,0 │         │
/// │  │16×16│16×16│16×16│16×16│16×16│16×16│16×16│16×16│         │
/// │  ├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤         │
/// │  │Block│Block│Block│Block│Block│Block│Block│Block│         │
/// │  │ 0,1 │ 1,1 │ 2,1 │ 3,1 │ 4,1 │ 5,1 │ 6,1 │ 7,1 │         │
/// │  │16×16│16×16│16×16│16×16│16×16│16×16│16×16│16×16│         │
/// │  ├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤         │
/// │  │  ⋮  │  ⋮  │  ⋮  │  ⋮  │  ⋮  │  ⋮  │  ⋮  │  ⋮  │         │
/// │  └─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┘         │
/// └─────────────────────────────────────────────────────────────┘
/// ```
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub const PIXEL_TEXTURE_FILE: &str = "assets/pix/symbols.png";

/// Calculate symbol width based on texture dimensions
/// 
/// Symbol size is calculated based on the size of the texture.
/// For a 128×128 symbol grid, each symbol's width is texture_width / 128.
pub fn init_sym_width(width: u32) -> f32 {
    width as f32 / 128.0
}

/// Calculate symbol height based on texture dimensions
/// 
/// Symbol size is calculated based on the size of the texture.
/// For a 128×128 symbol grid, each symbol's height is texture_height / 128.
pub fn init_sym_height(height: u32) -> f32 {
    height as f32 / 128.0
}

/// Global symbol width in pixels (initialized once)
/// 
/// This value is calculated once during initialization and cached for performance.
/// Used throughout the rendering pipeline for texture coordinate calculations.
pub static PIXEL_SYM_WIDTH: OnceLock<f32> = OnceLock::new();

/// Global symbol height in pixels (initialized once)
/// 
/// This value is calculated once during initialization and cached for performance.
/// Used throughout the rendering pipeline for texture coordinate calculations.
pub static PIXEL_SYM_HEIGHT: OnceLock<f32> = OnceLock::new();

/// RustPixel Logo width in characters
/// 
/// This defines the width of the startup logo animation in character units.
/// Used during the initial startup sequence for graphics mode rendering.
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub const PIXEL_LOGO_WIDTH: usize = 27;

/// RustPixel Logo height in characters
/// 
/// This defines the height of the startup logo animation in character units.
/// Used during the initial startup sequence for graphics mode rendering.
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub const PIXEL_LOGO_HEIGHT: usize = 12;

/// RustPixel Logo data array
/// 
/// Contains the logo image data as a flattened array of bytes. Each character
/// position is represented by 3 bytes: [symbol_id, texture_id, flags].
/// The array size is PIXEL_LOGO_WIDTH × PIXEL_LOGO_HEIGHT × 3 bytes.
/// 
/// Logo display process:
/// 1. During startup (stage <= LOGO_FRAME), this data is rendered
/// 2. Each triplet [symbol, texture, flags] defines a character
/// 3. Logo is centered on screen with animated effects
/// 4. After logo timeout, normal game rendering begins
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub const PIXEL_LOGO: [u8; PIXEL_LOGO_WIDTH * PIXEL_LOGO_HEIGHT * 3] = [
    32, 15, 1, 32, 202, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 239, 1, 32, 15, 1, 100, 239, 1, 32,
    239, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0,
    32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 1, 32, 15, 1, 32, 15, 0, 32, 15, 1, 32, 165, 1, 32,
    165, 0, 32, 87, 1, 32, 15, 1, 18, 202, 1, 21, 202, 1, 19, 202, 1, 20, 202, 1, 32, 15, 1, 47,
    239, 1, 47, 239, 1, 116, 239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15,
    0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32,
    15, 0, 32, 87, 1, 32, 165, 0, 32, 165, 1, 32, 240, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1,
    100, 239, 1, 100, 239, 1, 81, 49, 1, 47, 239, 1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 32, 15,
    1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1,
    100, 239, 1, 32, 239, 1, 100, 239, 1, 32, 239, 1, 32, 15, 0, 32, 87, 1, 32, 15, 0, 32, 165, 0,
    47, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 47, 239, 1, 47, 238, 1, 47,
    238, 1, 47, 238, 1, 47, 239, 1, 100, 239, 1, 46, 239, 1, 47, 239, 1, 47, 239, 1, 47, 239, 1,
    104, 239, 1, 104, 239, 1, 104, 239, 1, 104, 239, 1, 47, 239, 1, 47, 239, 1, 47, 239, 1, 84,
    239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 160, 49, 1, 160, 49, 1, 160, 49, 1, 160,
    49, 1, 81, 49, 1, 32, 15, 1, 160, 86, 1, 32, 15, 1, 160, 49, 1, 47, 236, 1, 47, 236, 1, 46,
    234, 1, 160, 49, 1, 47, 239, 1, 81, 49, 1, 160, 49, 1, 160, 49, 1, 160, 49, 1, 160, 49, 1, 47,
    239, 1, 160, 49, 1, 32, 15, 1, 84, 239, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 87, 1, 160, 45,
    1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 160, 45, 1, 32, 15, 1, 160, 45, 1, 32, 235, 1, 116, 235, 1,
    160, 45, 1, 47, 236, 1, 160, 45, 1, 47, 239, 1, 116, 239, 1, 160, 45, 1, 46, 234, 1, 32, 15, 1,
    46, 234, 1, 47, 239, 1, 116, 239, 1, 160, 45, 1, 32, 15, 1, 84, 239, 1, 32, 15, 0, 32, 15, 1,
    32, 15, 0, 32, 197, 1, 160, 147, 1, 32, 239, 1, 100, 239, 1, 100, 239, 1, 160, 147, 1, 32, 15,
    1, 160, 147, 1, 32, 235, 1, 116, 235, 1, 46, 235, 1, 81, 147, 1, 47, 239, 1, 47, 239, 1, 100,
    239, 1, 160, 147, 1, 160, 147, 1, 160, 147, 1, 160, 147, 1, 47, 239, 1, 32, 15, 1, 160, 147, 1,
    32, 239, 1, 84, 239, 1, 100, 239, 1, 100, 239, 1, 100, 239, 1, 32, 239, 1, 160, 147, 1, 47,
    239, 1, 104, 239, 1, 104, 240, 1, 160, 147, 1, 32, 15, 1, 160, 147, 1, 32, 15, 1, 116, 235, 1,
    160, 147, 1, 47, 239, 1, 160, 147, 1, 47, 239, 1, 47, 239, 1, 160, 147, 1, 104, 238, 1, 104,
    238, 1, 104, 238, 1, 104, 238, 1, 47, 242, 1, 160, 147, 1, 47, 239, 1, 104, 239, 1, 104, 239,
    1, 104, 239, 1, 47, 239, 1, 84, 239, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 81,
    214, 1, 47, 239, 1, 81, 214, 1, 47, 239, 1, 160, 214, 1, 47, 239, 1, 32, 0, 1, 46, 235, 1, 160,
    214, 1, 47, 236, 1, 81, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 160, 214, 1, 47, 242, 1,
    81, 214, 1, 81, 214, 1, 81, 214, 1, 81, 214, 1, 81, 214, 1, 47, 239, 1, 32, 165, 1, 160, 214,
    1, 103, 239, 1, 32, 242, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 0, 1,
    32, 0, 1, 32, 87, 1, 32, 87, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15,
    0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 165, 0, 32,
    165, 0, 160, 214, 1, 103, 239, 1, 32, 242, 1, 32, 97, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32,
    15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 97, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0,
    32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 97,
    0, 32, 165, 0, 32, 15, 1, 90, 214, 1, 47, 239, 1, 32, 0, 1, 32, 15, 0, 32, 0, 1, 32, 0, 1, 32,
    15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 0, 1, 32, 15, 0, 32, 0, 1, 32, 0, 1, 32, 0, 1, 32,
    0, 1, 32, 0, 1, 32, 0, 1, 32, 0, 1, 32, 15, 0, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32,
    15, 1, 32, 15, 1, 32, 15, 1,
];

/// Pre-render unit structure for OpenGL rendering
/// 
/// RenderCell serves as an intermediate data format between the game buffer
/// and the GPU rendering pipeline. This design provides several advantages:
/// 
/// ## Design Benefits
/// - **GPU-Optimized**: Data is pre-formatted for efficient GPU upload
/// - **Batch Processing**: Multiple cells can be rendered in a single draw call
/// - **Flexible Rendering**: Supports rotation, scaling, and complex effects
/// - **Memory Efficient**: Compact representation for large scenes
/// 
/// ## Rendering Pipeline Integration
/// ```text
/// ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
/// │   Buffer    │───►│ RenderCell  │───►│ OpenGL/GPU  │
/// │(Characters) │    │   Array     │    │  Rendering  │
/// └─────────────┘    └─────────────┘    └─────────────┘
/// ```
/// 
/// Each RenderCell contains all information needed to render one character
/// or sprite, including colors, position, rotation, and texture coordinates.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct RenderCell {
    /// Foreground color as RGBA components (0.0-1.0 range)
    /// 
    /// Used for character/symbol rendering. The alpha component controls
    /// transparency and blending operations.
    pub fcolor: (f32, f32, f32, f32),
    
    /// Optional background color as RGBA components
    /// 
    /// When present, renders a colored background behind the symbol.
    /// If None, the background is transparent.
    pub bcolor: Option<(f32, f32, f32, f32)>,
    
    /// Texture and symbol index packed into a single value
    /// 
    /// - High bits: texture index (which texture to use)
    /// - Low bits: symbol index (which character/symbol in the texture)
    pub texsym: usize,
    
    /// X position in screen coordinates
    pub x: f32,
    
    /// Y position in screen coordinates  
    pub y: f32,
    
    /// Width in pixels
    pub w: u32,
    
    /// Height in pixels
    pub h: u32,
    
    /// Rotation angle in radians
    /// 
    /// Used for sprite rotation effects. 0.0 means no rotation.
    pub angle: f32,
    
    /// Center X coordinate for rotation
    /// 
    /// Defines the pivot point around which rotation occurs.
    pub cx: f32,
    
    /// Center Y coordinate for rotation
    /// 
    /// Defines the pivot point around which rotation occurs.
    pub cy: f32,
}

/// Adapter base data structure containing shared information and OpenGL resources
/// 
/// AdapterBase holds common data and OpenGL resources shared across all graphics
/// mode adapters (SDL, Winit, Web). This design follows the principle of separation
/// of concerns while avoiding code duplication.
/// 
/// ## Architecture Role
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                  Adapter Interface                          │
/// │  ┌─────────────┬─────────────┬─────────────┬─────────────┐  │
/// │  │     SDL     │    Winit    │     Web     │  Crossterm  │  │
/// │  │   Adapter   │   Adapter   │   Adapter   │   Adapter   │  │
/// │  │      │      │      │      │      │      │      │      │  │
/// │  │  ┌───▼───┐  │  ┌───▼───┐  │  ┌───▼───┐  │      │      │  │
/// │  │  │ Base  │  │  │ Base  │  │  │ Base  │  │     N/A     │  │
/// │  │  │ Data  │  │  │ Data  │  │  │ Data  │  │ (Terminal)  │  │
/// │  │  └───────┘  │  └───────┘  │  └───────┘  │             │  │
/// │  └─────────────┴─────────────┴─────────────┴─────────────┘  │
/// └─────────────────────────────────────────────────────────────┘
/// ```
pub struct AdapterBase {
    /// Game name identifier
    pub game_name: String,
    
    /// Project root path for asset loading
    pub project_path: String,
    
    /// Window title displayed in graphics mode
    pub title: String,
    
    /// Game area width in character cells
    pub cell_w: u16,
    
    /// Game area height in character cells
    pub cell_h: u16,
    
    /// Physical window width in pixels
    pub pixel_w: u32,
    
    /// Physical window height in pixels
    pub pixel_h: u32,
    
    /// Horizontal scaling ratio for different DPI displays
    /// 
    /// Used to handle high-DPI displays and maintain consistent rendering
    /// across different screen resolutions.
    pub ratio_x: f32,
    
    /// Vertical scaling ratio for different DPI displays
    /// 
    /// Used to handle high-DPI displays and maintain consistent rendering
    /// across different screen resolutions.
    pub ratio_y: f32,
    
    /// Random number generator for effects and animations
    pub rd: Rand,
    
    /// Render flag controlling immediate vs buffered rendering
    /// 
    /// - true: Direct rendering to screen (normal mode)
    /// - false: Buffered rendering for external access (used for FFI/WASM)
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    pub rflag: bool,
    
    /// Render buffer storing RenderCell array for buffered mode
    /// 
    /// When rflag is false, rendered data is stored here instead of
    /// being directly drawn to screen. Used for external access to
    /// rendering data (e.g., Python FFI, WASM exports).
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    pub rbuf: Vec<RenderCell>,
    
    /// OpenGL context handle
    /// 
    /// Provides access to OpenGL functions for rendering operations.
    /// Uses the glow crate for cross-platform OpenGL abstraction.
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    pub gl: Option<glow::Context>,
    
    /// OpenGL pixel renderer instance
    /// 
    /// High-level OpenGL rendering interface that manages shaders,
    /// textures, and render targets for the pixel-based rendering pipeline.
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    pub gl_pixel: Option<GlPixel>,
}

impl AdapterBase {
    pub fn new(gn: &str, project_path: &str) -> Self {
        Self {
            game_name: gn.to_string(),
            project_path: project_path.to_string(),
            title: "".to_string(),
            cell_w: 0,
            cell_h: 0,
            pixel_w: 0,
            pixel_h: 0,
            ratio_x: 1.0,
            ratio_y: 1.0,
            rd: Rand::new(),
            #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
            rflag: true,
            #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
            rbuf: vec![],
            #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
            gl: None,
            #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
            gl_pixel: None,
        }
    }
}

/// Unified rendering interface definition
/// 
/// The Adapter trait defines a common interface for all rendering backends in RustPixel.
/// This design follows the adapter pattern, allowing different rendering technologies
/// to be used interchangeably while providing a consistent API.
/// 
/// ## Supported Backends
/// - **SDL Adapter**: Desktop rendering with SDL2
/// - **Winit Adapter**: Cross-platform window management with OpenGL  
/// - **Web Adapter**: Browser rendering with WebGL
/// - **Crossterm Adapter**: Terminal text mode rendering
/// 
/// ## Interface Design Principles
/// 1. **Abstraction**: Hide backend-specific implementation details
/// 2. **Consistency**: Same API across all platforms
/// 3. **Performance**: Minimal overhead in the abstraction layer
/// 4. **Flexibility**: Support for different rendering modes and features
/// 
/// ## Typical Usage Flow
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                    Game Application                          │
/// │                                                             │
/// │  1. init() ──────────► Initialize renderer                  │
/// │  2. Loop:                                                   │
/// │     ├── poll_event() ─► Handle input events                 │
/// │     ├── (game logic) ──► Update game state                  │
/// │     └── draw_all_to_screen() ──► Render frame               │
/// │  3. (cleanup) ────────► Automatic cleanup on drop          │
/// └─────────────────────────────────────────────────────────────┘
/// ```
pub trait Adapter {
    /// Initialize the rendering adapter
    /// 
    /// Sets up the rendering backend with specified parameters. This includes
    /// creating windows, initializing OpenGL contexts, loading textures, and
    /// preparing all necessary resources for rendering.
    /// 
    /// # Parameters
    /// - `w`: Game area width in character cells
    /// - `h`: Game area height in character cells  
    /// - `rx`: Horizontal scaling ratio for high-DPI displays
    /// - `ry`: Vertical scaling ratio for high-DPI displays
    /// - `s`: Window title string
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, s: String);
    
    /// Reset the adapter to initial state
    /// 
    /// Clears any accumulated state while keeping the rendering context alive.
    /// Used for restarting games or switching between different game modes.
    fn reset(&mut self);
    
    /// Get mutable reference to the base adapter data
    /// 
    /// Provides access to shared data structures like OpenGL context,
    /// render buffers, and common settings. Used internally by adapter
    /// implementations.
    fn get_base(&mut self) -> &mut AdapterBase;
    
    /// Poll for input events with timeout
    /// 
    /// Checks for user input events (keyboard, mouse, window events) and
    /// fills the provided event vector. Returns true if events were received.
    /// 
    /// # Parameters
    /// - `timeout`: Maximum time to wait for events
    /// - `ev`: Event vector to fill with received events
    /// 
    /// # Returns
    /// true if events were received, false if timeout occurred
    fn poll_event(&mut self, timeout: Duration, ev: &mut Vec<Event>) -> bool;

    /// Main rendering function - draws complete frame to screen
    /// 
    /// This is the core rendering method that processes the game buffer and
    /// sprites, then renders them to the screen. The implementation varies
    /// by backend but follows the same general pipeline:
    /// 
    /// 1. Convert game data to render format (RenderCell for graphics mode)
    /// 2. Process sprites and effects  
    /// 3. Apply any transitions or post-processing
    /// 4. Present the final image to screen
    /// 
    /// # Parameters
    /// - `current_buffer`: Current frame's character buffer
    /// - `previous_buffer`: Previous frame's buffer (for diff rendering)
    /// - `pixel_sprites`: Array of sprites to render
    /// - `stage`: Rendering stage (affects logo display, transitions, etc.)
    /// 
    /// # Returns
    /// Result indicating success or error message
    fn draw_all_to_screen(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String>;

    fn set_size(&mut self, w: u16, h: u16) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.cell_w = w;
        bs.cell_h = h;
        self
    }

    fn size(&mut self) -> Rect {
        let bs = self.get_base();
        Rect::new(0, 0, bs.cell_w, bs.cell_h)
    }

    fn set_ratiox(&mut self, rx: f32) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.ratio_x = rx;
        self
    }

    fn set_ratioy(&mut self, ry: f32) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.ratio_y = ry;
        self
    }

    fn set_pixel_size(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.pixel_w = ((bs.cell_w + 2) as f32 * PIXEL_SYM_WIDTH.get().expect("lazylock init")
            / bs.ratio_x) as u32;
        bs.pixel_h = ((bs.cell_h + 2) as f32 * PIXEL_SYM_HEIGHT.get().expect("lazylock init")
            / bs.ratio_y) as u32;
        self
    }

    fn set_title(&mut self, s: String) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.title = s;
        self
    }

    fn cell_width(&self) -> f32;
    fn cell_height(&self) -> f32;
    fn hide_cursor(&mut self) -> Result<(), String>;
    fn show_cursor(&mut self) -> Result<(), String>;
    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), String>;
    fn get_cursor(&mut self) -> Result<(u16, u16), String>;

    /// Main OpenGL rendering pipeline with double buffering and render textures
    /// 
    /// This method implements the core graphics rendering pipeline for SDL, Winit, and Web
    /// modes. It follows a two-pass rendering approach with multiple render targets:
    /// 
    /// ## Rendering Pipeline Architecture  
    /// ```text
    /// ┌─────────────────────────────────────────────────────────────┐
    /// │                     Pass 1: Data Conversion                 │
    /// │                                                             │
    /// │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
    /// │  │   Buffer    │    │   Sprites   │    │    Logo     │     │
    /// │  │             │    │             │    │             │     │
    /// │  └─────────────┘    └─────────────┘    └─────────────┘     │
    /// │         │                 │                    │           │
    /// │         └─────────────────┼────────────────────┘           │
    /// │                           ▼                                │
    /// │                ┌─────────────────────┐                     │
    /// │                │  draw_all_to_render │                     │
    /// │                │      _buffer()      │                     │
    /// │                └─────────────────────┘                     │
    /// │                           │                                │
    /// │                           ▼                                │
    /// │                ┌─────────────────────┐                     │
    /// │                │Vec<RenderCell> rbuf │                     │
    /// │                └─────────────────────┘                     │
    /// └─────────────────────────────────────────────────────────────┘
    ///                              │
    ///                              ▼
    /// ┌─────────────────────────────────────────────────────────────┐
    /// │                     Pass 2: GPU Rendering                   │
    /// │                                                             │
    /// │  rflag == true (Normal Mode)        rflag == false (Buffer) │
    /// │  ┌─────────────────────────┐        ┌─────────────────────┐ │
    /// │  │ draw_render_buffer_to_  │        │   Store rbuf in     │ │
    /// │  │   texture(rbuf, 2)     │        │   base.rbuf for     │ │
    /// │  │         │               │        │   external access   │ │
    /// │  │         ▼               │        └─────────────────────┘ │
    /// │  │ ┌─────────────────────┐ │                                │
    /// │  │ │  Render Texture 2   │ │                                │
    /// │  │ │    (Main Scene)     │ │                                │
    /// │  │ └─────────────────────┘ │                                │
    /// │  │         │               │                                │
    /// │  │         ▼               │                                │
    /// │  │ draw_render_textures_   │                                │
    /// │  │     to_screen()         │                                │
    /// │  │         │               │                                │
    /// │  │         ▼               │                                │
    /// │  │ ┌─────────────────────┐ │                                │
    /// │  │ │  Screen/Backbuffer  │ │                                │
    /// │  │ │  (Final Composite)  │ │                                │
    /// │  │ └─────────────────────┘ │                                │
    /// │  └─────────────────────────┘                                │
    /// └─────────────────────────────────────────────────────────────┘
    /// ```
    /// 
    /// ## Render Targets
    /// - **Render Texture 2**: Main game content (characters, sprites, borders)
    /// - **Render Texture 3**: Transition effects and overlays
    /// - **Screen Buffer**: Final composite output
    /// 
    /// ## Rendering Modes
    /// - **rflag=true**: Normal rendering directly to screen
    /// - **rflag=false**: Buffered mode - stores render data for external access (FFI/WASM)
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    fn draw_all_graph(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) {
        // Pass 1: Convert game data (buffer + sprites) to GPU-ready format
        let rbuf =
            self.draw_all_to_render_buffer(current_buffer, previous_buffer, pixel_sprites, stage);
        
        // Pass 2: Render to screen or buffer based on mode
        if self.get_base().rflag {
            // Normal mode: Render through GPU pipeline
            // 1. Draw RenderCell array to render_texture 2 (main scene)
            self.draw_render_buffer_to_texture(&rbuf, 2, false);
            // 2. Composite render_texture 2 & 3 to screen (final output)
            self.draw_render_textures_to_screen();
        } else {
            // Buffered mode: Store render data for external access
            // Used by FFI interfaces and WASM exports to access raw render data
            self.get_base().rbuf = rbuf;
        }
    }

    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    fn only_render_buffer(&mut self) {
        self.get_base().rflag = false;
    }

    /// Render texture composition to screen - final rendering stage
    /// 
    /// This method performs the final composite rendering step, combining multiple
    /// render textures into the final screen output. It handles layer composition,
    /// scaling for different display ratios, and transition effects.
    /// 
    /// ## Rendering Order and Layers
    /// ```text
    /// ┌─────────────────────────────────────────────────────────────┐
    /// │                    Screen Composition                        │
    /// │                                                             │
    /// │  Background (Clear Color)                                   │
    /// │      ▲                                                      │
    /// │      │                                                      │
    /// │  ┌───┴─────────────────────┐  ← Layer 1: Main Content      │
    /// │  │   Render Texture 2      │    - Game buffer              │
    /// │  │   (Main Game Content)   │    - Sprites                  │
    /// │  │   - Characters & Tiles  │    - Borders                  │
    /// │  │   - Sprites & Objects   │    - Logo (during startup)    │
    /// │  │   - Borders & UI        │                               │
    /// │  └─────────────────────────┘                               │
    /// │      ▲                                                      │
    /// │      │                                                      │
    /// │  ┌───┴─────────────────────┐  ← Layer 2: Effects & Trans  │
    /// │  │   Render Texture 3      │    - Transition effects      │
    /// │  │   (Transitions & FX)    │    - Overlays                │
    /// │  │   - Screen transitions  │    - Post-processing         │
    /// │  │   - Visual effects      │    - Special effects         │
    /// │  │   - Overlays           │                               │
    /// │  └─────────────────────────┘                               │
    /// │      ▲                                                      │
    /// │      │                                                      │
    /// │  ┌───┴─────────────────────┐  ← Final Output               │
    /// │  │      Screen Buffer      │                               │
    /// │  │    (Framebuffer 0)      │                               │
    /// │  └─────────────────────────┘                               │
    /// └─────────────────────────────────────────────────────────────┘
    /// ```
    /// 
    /// ## High-DPI Display Scaling
    /// The method handles different display pixel densities by calculating proper
    /// scaling ratios and viewport transformations, ensuring consistent rendering
    /// across various screen types including Retina displays.
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    fn draw_render_textures_to_screen(&mut self) {
        let bs = self.get_base();

        if let (Some(pix), Some(gl)) = (&mut bs.gl_pixel, &mut bs.gl) {
            // Bind to screen framebuffer (0) for final output
            pix.bind_screen(gl);
            let c = GlColor::new(1.0, 1.0, 1.0, 1.0);

            // Layer 1: Draw render_texture 2 (main game content)
            // Contains: characters, sprites, borders, logo
            if !pix.get_render_texture_hidden(2) {
                let t = GlTransform::new();
                // Full-screen quad with identity transform
                pix.draw_general2d(gl, 2, [0.0, 0.0, 1.0, 1.0], &t, &c);
            }

            // Layer 2: Draw render_texture 3 (transition effects and overlays)  
            // Contains: transition effects, visual overlays, special effects
            if !pix.get_render_texture_hidden(3) {
                // Calculate proper scaling for high-DPI displays
                let pcw = pix.canvas_width as f32;   // Physical canvas width
                let pch = pix.canvas_height as f32;  // Physical canvas height
                let rx = bs.ratio_x;  // Horizontal scaling ratio
                let ry = bs.ratio_y;  // Vertical scaling ratio
                
                // Calculate scaled dimensions for transition layer
                // Base size is 40x25 characters scaled by symbol size and DPI ratio
                let pw = 40.0 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
                let ph = 25.0 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;

                // Create transform with proper scaling
                let mut t2 = GlTransform::new();
                t2.scale(pw / pcw, ph / pch);
                
                // Draw transition layer with calculated viewport and transform
                // Positioned at bottom-left with calculated dimensions
                pix.draw_general2d(
                    gl,
                    3,
                    [0.0 / pcw, (pch - ph) / pch, pw / pcw, ph / pch],
                    &t2,
                    &c,
                );
            }
        }
    }

    // draw buffer to render texture...
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    fn draw_buffer_to_texture(&mut self, buf: &Buffer, rtidx: usize) {
        let rbuf = self.buffer_to_render_buffer(buf);
        // For debug...
        // self.draw_render_buffer(&rbuf, rtidx, true);
        self.draw_render_buffer_to_texture(&rbuf, rtidx, false);
    }

    // draw render buffer to render texture...
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    fn draw_render_buffer_to_texture(&mut self, rbuf: &[RenderCell], rtidx: usize, debug: bool) {
        let bs = self.get_base();
        let rx = bs.ratio_x;
        let ry = bs.ratio_y;
        if let (Some(pix), Some(gl)) = (&mut bs.gl_pixel, &mut bs.gl) {
            pix.bind_target(gl, rtidx);
            if debug {
                // set red background for debug...
                pix.set_clear_color(GlColor::new(1.0, 0.0, 0.0, 1.0));
            } else {
                pix.set_clear_color(GlColor::new(0.0, 0.0, 0.0, 1.0));
            }
            pix.clear(gl);
            pix.render_rbuf(gl, rbuf, rx, ry);
        }
    }

    // buffer to render buffer...
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    fn buffer_to_render_buffer(&mut self, cb: &Buffer) -> Vec<RenderCell> {
        let mut rbuf = vec![];
        let rx = self.get_base().ratio_x;
        let ry = self.get_base().ratio_y;
        let pz = PointI32 { x: 0, y: 0 };
        let mut rfunc = |fc: &(u8, u8, u8, u8),
                         bc: &Option<(u8, u8, u8, u8)>,
                         _s0: ARect,
                         _s1: ARect,
                         s2: ARect,
                         texidx: usize,
                         symidx: usize| {
            push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, 0.0, &pz);
        };
        render_main_buffer(cb, cb.area.width, rx, ry, true, &mut rfunc);
        rbuf
    }

    // draw main buffer & pixel sprites to render buffer...
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    fn draw_all_to_render_buffer(
        &mut self,
        cb: &Buffer,
        _pb: &Buffer,
        ps: &mut Vec<Sprites>,
        stage: u32,
    ) -> Vec<RenderCell> {
        let mut rbuf = vec![];
        let width = cb.area.width;
        let pz = PointI32 { x: 0, y: 0 };

        // render logo...
        if stage <= LOGO_FRAME {
            render_logo(
                self.get_base().ratio_x,
                self.get_base().ratio_y,
                self.get_base().pixel_w,
                self.get_base().pixel_h,
                &mut self.get_base().rd,
                stage,
                |fc, _s1, s2, texidx, symidx| {
                    push_render_buffer(&mut rbuf, fc, &None, texidx, symidx, s2, 0.0, &pz);
                },
            );
            return rbuf;
        }

        let cw = self.get_base().cell_w;
        let ch = self.get_base().cell_h;
        let rx = self.get_base().ratio_x;
        let ry = self.get_base().ratio_y;
        let mut rfunc = |fc: &(u8, u8, u8, u8),
                         bc: &Option<(u8, u8, u8, u8)>,
                         _s0: ARect,
                         _s1: ARect,
                         s2: ARect,
                         texidx: usize,
                         symidx: usize| {
            push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, 0.0, &pz);
        };

        // render windows border, for sdl and winit mode
        #[cfg(any(feature = "sdl", feature = "winit"))]
        render_border(cw, ch, rx, ry, &mut rfunc);

        // render main buffer...
        if stage > LOGO_FRAME {
            render_main_buffer(cb, width, rx, ry, false, &mut rfunc);
        }

        // render pixel_sprites...
        if stage > LOGO_FRAME {
            for item in ps {
                if item.is_pixel && !item.is_hidden {
                    render_pixel_sprites(
                        item,
                        rx,
                        ry,
                        |fc, bc, _s0, _s1, s2, texidx, symidx, angle, ccp| {
                            push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, angle, &ccp);
                        },
                    );
                }
            }
        }
        rbuf
    }

    fn as_any(&mut self) -> &mut dyn Any;
}

#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
/// Convert game data to RenderCell format with texture coordinate calculation
/// 
/// This function converts individual game elements (characters, sprites, etc.) into
/// GPU-ready RenderCell format. It handles texture coordinate calculation, color
/// conversion, and transformation parameters.
/// 
/// ## Conversion Process
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                  Data Transformation                        │
/// │                                                             │
/// │  Game Data Input:                                           │
/// │  ├── Colors (u8 RGBA) ────────► Normalized (f32 RGBA)      │
/// │  ├── Texture & Symbol Index ──► Packed texsym value        │
/// │  ├── Screen Rectangle ─────────► Position & dimensions     │
/// │  ├── Rotation angle ───────────► Angle + center point      │
/// │  └── Background color ─────────► Optional background       │
/// │                                                             │
/// │                       ▼                                     │
/// │               ┌─────────────────────┐                       │
/// │               │    RenderCell       │                       │
/// │               │   (GPU-ready)       │                       │
/// │               └─────────────────────┘                       │
/// └─────────────────────────────────────────────────────────────┘
/// ```
/// 
/// # Parameters
/// - `rbuf`: Target RenderCell vector to append to
/// - `fc`: Foreground color as (R,G,B,A) in 0-255 range
/// - `bgc`: Optional background color  
/// - `texidx`: Texture index in the texture atlas
/// - `symidx`: Symbol index within the texture
/// - `s`: Screen rectangle (position and size)
/// - `angle`: Rotation angle in radians
/// - `ccp`: Center point for rotation
fn push_render_buffer(
    rbuf: &mut Vec<RenderCell>,
    fc: &(u8, u8, u8, u8),
    bgc: &Option<(u8, u8, u8, u8)>,
    texidx: usize,
    symidx: usize,
    s: ARect,
    angle: f64,
    ccp: &PointI32,
) {
    let mut wc = RenderCell {
        fcolor: (
            fc.0 as f32 / 255.0,
            fc.1 as f32 / 255.0,
            fc.2 as f32 / 255.0,
            fc.3 as f32 / 255.0,
        ),
        ..Default::default()
    };
    if let Some(bc) = bgc {
        wc.bcolor = Some((
            bc.0 as f32 / 255.0,
            bc.1 as f32 / 255.0,
            bc.2 as f32 / 255.0,
            bc.3 as f32 / 255.0,
        ));
    } else {
        wc.bcolor = None;
    }
    let x = symidx as u32 % 16u32 + (texidx as u32 % 8u32) * 16u32;
    let y = symidx as u32 / 16u32 + (texidx as u32 / 8u32) * 16u32;
    wc.texsym = (y * 16u32 * 8u32 + x) as usize;
    wc.x = s.x as f32 + PIXEL_SYM_WIDTH.get().expect("lazylock init");
    wc.y = s.y as f32 + PIXEL_SYM_HEIGHT.get().expect("lazylock init");
    wc.w = s.w;
    wc.h = s.h;
    if angle == 0.0 {
        wc.angle = angle as f32;
    } else {
        let mut aa = (1.0 - angle / 180.0) * std::f64::consts::PI;
        let pi2 = std::f64::consts::PI * 2.0;
        while aa < 0.0 {
            aa += pi2;
        }
        while aa > pi2 {
            aa -= pi2;
        }
        wc.angle = aa as f32;
    }
    wc.cx = ccp.x as f32;
    wc.cy = ccp.y as f32;
    rbuf.push(wc);
}

#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
fn render_helper(
    cell_w: u16,
    r: PointF32,
    i: usize,
    sh: &(u8, u8, Color, Color),
    p: PointU16,
    is_border: bool,
) -> (ARect, ARect, ARect, usize, usize) {
    let w = *PIXEL_SYM_WIDTH.get().expect("lazylock init") as i32;
    let h = *PIXEL_SYM_HEIGHT.get().expect("lazylock init") as i32;
    let dstx = i as u16 % cell_w;
    let dsty = i as u16 / cell_w;
    let tex_count = 64;
    let tx = if sh.1 < tex_count { sh.1 as usize } else { 1 };
    let srcy = sh.0 as u32 / w as u32 + (tx as u32 / 2u32) * w as u32;
    let srcx = sh.0 as u32 % w as u32 + (tx as u32 % 2u32) * w as u32;
    let bsrcy = 160u32 / w as u32;
    let bsrcx = 160u32 % w as u32 + w as u32;

    (
        // background sym rect in texture(sym=160 tex=1)
        ARect {
            x: w * bsrcx as i32,
            y: h * bsrcy as i32,
            w: w as u32,
            h: h as u32,
        },
        // sym rect in texture
        ARect {
            x: w * srcx as i32,
            y: h * srcy as i32,
            w: w as u32,
            h: h as u32,
        },
        // dst rect in render texture
        ARect {
            x: (dstx + if is_border { 0 } else { 1 }) as i32 * (w as f32 / r.x) as i32 + p.x as i32,
            y: (dsty + if is_border { 0 } else { 1 }) as i32 * (h as f32 / r.y) as i32 + p.y as i32,
            w: (w as f32 / r.x) as u32,
            h: (h as f32 / r.y) as u32,
        },
        // texture id
        tx,
        // sym id
        sh.0 as usize,
    )
}

#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
/// Render pixel sprites with rotation and transformation support
/// 
/// This function processes individual sprite objects and converts them to renderable
/// format. It supports advanced features like rotation, scaling, and complex
/// transformations while maintaining efficient rendering performance.
/// 
/// ## Sprite Rendering Pipeline
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                   Sprite Processing                         │
/// │                                                             │
/// │  ┌─────────────┐                                            │
/// │  │   Sprite    │                                            │
/// │  │   Object    │                                            │
/// │  │  ┌───────┐  │  ┌─────────────────────────────────────┐  │
/// │  │  │Pixels │  │  │        Transformation               │  │
/// │  │  │Array  │  │  │  ┌─────────────────────────────────┐ │  │
/// │  │  └───────┘  │  │  │  1. Position calculation        │ │  │
/// │  │     │       │  │  │  2. Rotation matrix applied     │ │  │
/// │  │     ▼       │  │  │  3. Scaling based on rx/ry     │ │  │
/// │  │  ┌───────┐  │  │  │  4. Color & texture mapping    │ │  │
/// │  │  │Colors │  │  │  └─────────────────────────────────┘ │  │
/// │  │  │&Flags │  │  └─────────────────────────────────────┘  │
/// │  │  └───────┘  │                     │                     │
/// │  └─────────────┘                     ▼                     │
/// │                        ┌─────────────────────┐              │
/// │                        │  Callback Function  │              │
/// │                        │ (push_render_buffer) │              │
/// │                        └─────────────────────┘              │
/// │                                 │                           │
/// │                                 ▼                           │
/// │                        ┌─────────────────────┐              │
/// │                        │    RenderCell       │              │
/// │                        │      Array          │              │
/// │                        └─────────────────────┘              │
/// └─────────────────────────────────────────────────────────────┘
/// ```
/// 
/// ## Features Supported
/// - **Rotation**: Full 360-degree rotation around sprite center
/// - **Scaling**: Display ratio compensation for different screen densities
/// - **Transparency**: Alpha blending and background color support
/// - **Instanced Rendering**: Efficient batch processing for multiple sprites
/// 
/// # Parameters
/// - `pixel_spt`: Sprite object containing pixel data and transformation info
/// - `rx`: Horizontal scaling ratio for display compensation
/// - `ry`: Vertical scaling ratio for display compensation  
/// - `f`: Callback function to process each sprite pixel
pub fn render_pixel_sprites<F>(pixel_spt: &mut Sprites, rx: f32, ry: f32, mut f: F)
where
    // Callback signature: (fg_color, bg_color, bg_rect, sym_rect, dst_rect, tex_idx, sym_idx, angle, center_point)
    F: FnMut(
        &(u8, u8, u8, u8),
        &Option<(u8, u8, u8, u8)>,
        ARect,
        ARect,
        ARect,
        usize,
        usize,
        f64,
        PointI32,
    ),
{
    // sort by render_weight...
    pixel_spt.update_render_index();
    for si in &pixel_spt.render_index {
        let s = &pixel_spt.sprites[si.0];
        if s.is_hidden() {
            continue;
        }
        let px = s.content.area.x;
        let py = s.content.area.y;
        let pw = s.content.area.width;
        let ph = s.content.area.height;

        for (i, cell) in s.content.content.iter().enumerate() {
            let sh = &cell.get_cell_info();
            let (s0, s1, s2, texidx, symidx) = render_helper(
                pw,
                PointF32 { x: rx, y: ry },
                i,
                sh,
                PointU16 { x: px, y: py },
                false,
            );
            let x = i % pw as usize;
            let y = i / pw as usize;
            // center point ...
            let ccp = PointI32 {
                x: ((pw as f32 / 2.0 - x as f32) * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx) as i32,
                y: ((ph as f32 / 2.0 - y as f32) * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry) as i32,
            };
            let mut fc = sh.2.get_rgba();
            fc.3 = s.alpha;
            let bc;
            if sh.3 != Color::Reset {
                let mut brgba = sh.3.get_rgba();
                brgba.3 = s.alpha;
                bc = Some(brgba);
            } else {
                bc = None;
            }
            f(&fc, &bc, s0, s1, s2, texidx, symidx, s.angle, ccp);
        }
    }
}

#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
/// Main buffer rendering with character-to-pixel conversion
/// 
/// This function processes the main game buffer containing character data and
/// converts it to renderable format. It follows the principle.md design where
/// characters are the fundamental rendering unit, with each character mapped
/// to symbols in the texture atlas.
/// 
/// ## Buffer Rendering Process
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                   Main Buffer Processing                    │
/// │                                                             │
/// │  ┌─────────────────────┐                                   │
/// │  │      Buffer         │                                   │
/// │  │   ┌─────────────┐   │                                   │
/// │  │   │ Character   │   │    ┌─────────────────────────────┐│
/// │  │   │   Grid      │   │    │   Per-Character Process    ││
/// │  │   │             │   │    │                             ││
/// │  │   │ ┌─┬─┬─┬─┐   │   │    │ 1. Read character data      ││
/// │  │   │ │A│B│C│D│   │   │    │ 2. Extract colors & symbol  ││
/// │  │   │ ├─┼─┼─┼─┤   │───────► │ 3. Calculate screen pos     ││
/// │  │   │ │E│F│G│H│   │   │    │ 4. Map to texture coords    ││
/// │  │   │ ├─┼─┼─┼─┤   │   │    │ 5. Call render callback     ││
/// │  │   │ │I│J│K│L│   │   │    │                             ││
/// │  │   │ └─┴─┴─┴─┘   │   │    └─────────────────────────────┘│
/// │  │   └─────────────┘   │                     │             │
/// │  └─────────────────────┘                     ▼             │
/// │                                ┌─────────────────────┐     │
/// │                                │   RenderCell Array  │     │
/// │                                │   (GPU-ready data)  │     │
/// │                                └─────────────────────┘     │
/// └─────────────────────────────────────────────────────────────┘
/// ```
/// 
/// ## Character Data Structure
/// Each character in the buffer contains:
/// - **Symbol Index**: Which character/symbol to display
/// - **Texture Index**: Which texture sheet to use  
/// - **Foreground Color**: Primary character color
/// - **Background Color**: Optional background fill color
/// - **Style Flags**: Bold, italic, underline, etc.
/// 
/// # Parameters
/// - `buf`: Game buffer containing character grid data
/// - `width`: Buffer width in characters
/// - `rx`: Horizontal scaling ratio for display adaptation
/// - `ry`: Vertical scaling ratio for display adaptation
/// - `border`: Include border rendering (for windowed modes)
/// - `f`: Callback function to process each character
pub fn render_main_buffer<F>(buf: &Buffer, width: u16, rx: f32, ry: f32, border: bool, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, ARect, ARect, usize, usize),
{
    for (i, cell) in buf.content.iter().enumerate() {
        // symidx, texidx, fg, bg
        let sh = cell.get_cell_info();
        let (s0, s1, s2, texidx, symidx) = render_helper(
            width,
            PointF32 { x: rx, y: ry },
            i,
            &sh,
            PointU16 { x: 0, y: 0 },
            border,
        );
        let fc = sh.2.get_rgba();
        let bc = if sh.3 != Color::Reset {
            Some(sh.3.get_rgba())
        } else {
            None
        };
        f(&fc, &bc, s0, s1, s2, texidx, symidx);
    }
}

#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
/// Window border rendering for windowed display modes
/// 
/// This function renders decorative borders around the game area for SDL and Winit
/// modes. The border provides visual separation between the game content and the
/// desktop environment, creating a more polished windowed gaming experience.
/// 
/// ## Border Layout
/// ```text
/// ┌───────────────────────────────────────────────────────┐
/// │                      Window Border                    │
/// │  ┌─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┐  │
/// │  ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤  │
/// │  ├─┤                 Game Content Area           ├─┤  │
/// │  ├─┤                                             ├─┤  │
/// │  ├─┤                     80 x 40                 ├─┤  │
/// │  ├─┤                  Character Grid             ├─┤  │
/// │  ├─┤                                             ├─┤  │
/// │  ├─┤                                             ├─┤  │
/// │  ├─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┼─┤  │
/// │  └─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┴─┘  │
/// └───────────────────────────────────────────────────────┘
/// ```
/// 
/// The border consists of:
/// - **Top/Bottom Edges**: Horizontal line characters
/// - **Left/Right Edges**: Vertical line characters  
/// - **Corners**: Corner junction characters
/// - **Consistent Styling**: Matches the game's visual theme
/// 
/// # Parameters
/// - `cell_w`: Game area width in characters
/// - `cell_h`: Game area height in characters
/// - `rx`: Horizontal scaling ratio
/// - `ry`: Vertical scaling ratio
/// - `f`: Callback function to render each border character
pub fn render_border<F>(cell_w: u16, cell_h: u16, rx: f32, ry: f32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, ARect, ARect, usize, usize),
{
    let sh_top = (102u8, 1u8, Color::Indexed(7), Color::Reset);
    let sh_other = (24u8, 2u8, Color::Indexed(7), Color::Reset);
    let sh_close = (214u8, 1u8, Color::Indexed(7), Color::Reset);

    for n in 0..cell_h as usize + 2 {
        for m in 0..cell_w as usize + 2 {
            if n != 0 && n != cell_h as usize + 1 && m != 0 && m != cell_w as usize + 1 {
                continue;
            }
            let rsh;
            if n == 0 {
                if m as u16 <= cell_w {
                    rsh = &sh_top;
                } else {
                    rsh = &sh_close;
                }
            } else {
                rsh = &sh_other;
            }
            let (s0, s1, s2, texidx, symidx) = render_helper(
                cell_w + 2,
                PointF32 { x: rx, y: ry },
                n * (cell_w as usize + 2) + m,
                rsh,
                PointU16 { x: 0, y: 0 },
                true,
            );
            let fc = rsh.2.get_rgba();
            let bc = None;
            f(&fc, &bc, s0, s1, s2, texidx, symidx);
        }
    }
}

#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
/// RustPixel Logo animation rendering with dynamic effects
/// 
/// This function renders the animated RustPixel logo during the startup sequence.
/// It provides a visually appealing introduction to the framework with dynamic
/// effects and proper centering across different screen resolutions.
/// 
/// ## Logo Animation Sequence
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                    Logo Animation Timeline                  │
/// │                                                             │
/// │  Stage 0 ────────────────────────────────► LOGO_FRAME      │
/// │    │                                            │           │
/// │    ▼                                            ▼           │
/// │  ┌─────────────────┐                    ┌─────────────────┐ │
/// │  │  Logo Display   │                    │  Start Game     │ │
/// │  │                 │                    │   Rendering     │ │
/// │  │  ┌───────────┐  │                    │                 │ │
/// │  │  │ ██████    │  │   Dynamic Effects:  │                 │ │
/// │  │  │ ██  ██    │  │   - Random colors   │                 │ │
/// │  │  │ ██████    │  │   - Centered pos    │                 │ │
/// │  │  │ ██  ██    │  │   - Smooth trans    │                 │ │
/// │  │  │ ██  ██    │  │   - Frame timing    │                 │ │
/// │  │  └───────────┘  │                    │                 │ │
/// │  └─────────────────┘                    └─────────────────┘ │
/// └─────────────────────────────────────────────────────────────┘
/// ```
/// 
/// ## Rendering Features
/// - **Centered Positioning**: Automatically centers on any screen size
/// - **Dynamic Colors**: Randomly generated color effects for visual appeal
/// - **Smooth Animation**: Frame-based timing for consistent display
/// - **High-DPI Support**: Proper scaling for different display densities
/// - **Cross-platform**: Works consistently across SDL, Winit, and Web modes
/// 
/// ## Logo Data Processing
/// The function processes the PIXEL_LOGO constant array where each character
/// is represented by 3 bytes: [symbol_id, texture_id, flags]. The logo is
/// dynamically positioned and colored based on the current animation stage.
/// 
/// # Parameters
/// - `srx`: Screen horizontal scaling ratio
/// - `sry`: Screen vertical scaling ratio  
/// - `spw`: Screen physical width in pixels
/// - `sph`: Screen physical height in pixels
/// - `rd`: Random number generator for color effects
/// - `stage`: Current animation stage (0 to LOGO_FRAME)
/// - `f`: Callback function to render each logo character
pub fn render_logo<F>(srx: f32, sry: f32, spw: u32, sph: u32, rd: &mut Rand, stage: u32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), ARect, ARect, usize, usize),
{
    let rx = srx * 1.0;
    let ry = sry * 1.0;
    for y in 0usize..PIXEL_LOGO_HEIGHT {
        for x in 0usize..PIXEL_LOGO_WIDTH {
            let sci = y * PIXEL_LOGO_WIDTH + x;
            let symw = PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
            let symh = PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;

            let (_s0, s1, mut s2, texidx, symidx) = render_helper(
                PIXEL_LOGO_WIDTH as u16,
                PointF32 { x: rx, y: ry },
                sci,
                &(
                    PIXEL_LOGO[sci * 3],
                    PIXEL_LOGO[sci * 3 + 2],
                    Color::Indexed(PIXEL_LOGO[sci * 3 + 1]),
                    Color::Reset,
                ),
                PointU16 {
                    x: spw as u16 / 2 - (PIXEL_LOGO_WIDTH as f32 / 2.0 * symw) as u16,
                    y: sph as u16 / 2 - (PIXEL_LOGO_HEIGHT as f32 / 2.0 * symh) as u16,
                },
                false,
            );
            let fc = Color::Indexed(PIXEL_LOGO[sci * 3 + 1]).get_rgba();

            let randadj = 12 - (rd.rand() % 24) as i32;
            let sg = LOGO_FRAME as u8 / 3;
            let r: u8;
            let g: u8;
            let b: u8;
            let a: u8;
            if stage <= sg as u32 {
                r = (stage as u8).saturating_mul(10);
                g = (stage as u8).saturating_mul(10);
                b = (stage as u8).saturating_mul(10);
                a = 255;
                s2.x += randadj;
            } else if stage <= sg as u32 * 2 {
                r = fc.0;
                g = fc.1;
                b = fc.2;
                a = 255;
            } else {
                let cc = (stage as u8 - sg * 2).saturating_mul(10);
                r = fc.0.saturating_sub(cc);
                g = fc.1.saturating_sub(cc);
                b = fc.2.saturating_sub(cc);
                a = 255;
            }
            f(&(r, g, b, a), s1, s2, texidx, symidx);
        }
    }
}

