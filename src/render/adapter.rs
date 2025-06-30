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
//! │              │    Crossterm        │                        │
//! │              │   (Terminal)        │                        │
//! │              └─────────────────────┘                        │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//! 
//! ### Graphics Mode Rendering (Pass 1: Buffer Generation)
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Graphics Rendering                       │
//! │  ┌─────────────┐  ┌─────────────┐                          │
//! │  │   Regular   │  │   Pixel     │                          │
//! │  │  Sprites    │  │  Sprites    │                          │
//! │  │ (Background)│  │(Foreground) │                          │
//! │  └─────────────┘  └─────────────┘                          │
//! │         │                │                                  │
//! │         ▼                │                                  │
//! │  ┌─────────────┐         │                                  │
//! │  │Main Buffer  │         │                                  │
//! │  │(Characters) │         │                                  │
//! │  └─────────────┘         │                                  │
//! │         │                │                                  │
//! │         └────────────────┼─────────────────┐                │
//! │                          ▼                 ▼                │
//! │              ┌─────────────────────────────────┐            │
//! │              │        RenderBuffer             │            │
//! │              │    (Vec<RenderCell>)            │            │
//! │              │                                 │            │
//! │              │  Each RenderCell contains:      │            │
//! │              │  - Colors (fg/bg)               │            │
//! │              │  - Texture symbol index         │            │
//! │              │  - Position & size              │            │
//! │              │  - Rotation angle & center      │            │
//! │              │  - Rotation angle & center      │            │
//! │              └─────────────────────────────────┘            │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//! 
//! ### Graphics Mode Rendering (Pass 2: OpenGL Pipeline)
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

/// SDL adapter module - Desktop rendering backend based on SDL2
#[cfg(all(feature = "sdl", not(target_arch = "wasm32")))]
pub mod sdl;

/// Web adapter module - WebGL-based browser rendering backend
#[cfg(target_arch = "wasm32")]
pub mod web;

/// Winit adapter module - Cross-platform rendering backend using winit + OpenGL  
#[cfg(all(feature = "winit", not(target_arch = "wasm32")))]
pub mod winit;

/// Crossterm adapter module - Terminal text-mode rendering backend
#[cfg(not(any(
    feature = "sdl",
    feature = "winit",
    target_os = "android",
    target_os = "ios",
    target_arch = "wasm32"
)))]
pub mod cross;

/// Symbol texture file path
/// 
/// The texture contains 8x8 blocks, each block contains 16x16 symbols,
/// for a total of 128x128 = 16,384 symbols arranged in a grid
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub const PIXEL_TEXTURE_FILE: &str = "assets/pix/symbols.png";

/// Calculate individual symbol width based on total texture width
/// 
/// Since the texture is a 128x128 symbol grid, 
/// individual symbol width = total width / 128
pub fn init_sym_width(width: u32) -> f32 {
    width as f32 / 128.0
}

/// Calculate individual symbol height based on total texture height
/// 
/// Since the texture is a 128x128 symbol grid,
/// individual symbol height = total height / 128  
pub fn init_sym_height(height: u32) -> f32 {
    height as f32 / 128.0
}

/// Global symbol width cache, initialized once at startup
pub static PIXEL_SYM_WIDTH: OnceLock<f32> = OnceLock::new();

/// Global symbol height cache, initialized once at startup
pub static PIXEL_SYM_HEIGHT: OnceLock<f32> = OnceLock::new();

/// RustPixel Logo display constants
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub const PIXEL_LOGO_WIDTH: usize = 27;   // Logo width in characters
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub const PIXEL_LOGO_HEIGHT: usize = 12;  // Logo height in characters

/// Logo data - each 3 bytes represent one character: [symbol_index, texture_index, color_index]
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
    0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 165, 0, 32, 165, 0, 160,
    214, 1, 103, 239, 1, 32, 242, 1, 32, 97, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15,
    0, 32, 15, 0, 32, 97, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32,
    15, 0, 32, 15, 0, 32, 15, 0, 32, 97, 0, 32, 165, 0, 32, 15, 1, 90, 214, 1, 47, 239, 1, 32, 0, 1, 32,
    15, 0, 32, 0, 1, 32, 0, 1, 32, 15, 0, 32, 15, 0, 32, 15, 0, 32, 0, 1, 32, 15, 0, 32, 0, 1, 32,
    0, 1, 32, 0, 1, 32, 0, 1, 32, 0, 1, 32, 0, 1, 32, 0, 1, 32, 15, 0, 32, 15, 1, 32, 15, 1, 32, 15,
    1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1, 32, 15, 1,
];

/// Pre-render Cell Structure (Graphics Mode Pass 1 Output)
/// 
/// This is the intermediate data structure used for OpenGL and WebGL rendering.
/// Each RenderCell represents one renderable element on screen, containing position,
/// size, color, texture information, etc.
/// 
/// Based on principle.md graphics mode design, RenderCell provides:
/// - Better rendering performance (GPU-friendly data format)
/// - Support for complex transformations (rotation, scaling)
/// - Unified rendering interface across platforms
/// 
/// ```text
/// Buffer/Sprites → RenderCell → OpenGL → Screen
/// ```
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct RenderCell {
    /// Foreground color (RGBA, 0.0-1.0 range)
    pub fcolor: (f32, f32, f32, f32),
    /// Background color (optional, RGBA, 0.0-1.0 range)
    pub bcolor: Option<(f32, f32, f32, f32)>,
    /// Texture symbol index (linear index in 128x128 symbol grid)
    pub texsym: usize,
    /// Render position X coordinate (pixel units)
    pub x: f32,
    /// Render position Y coordinate (pixel units)
    pub y: f32,
    /// Render width (pixel units)
    pub w: u32,
    /// Render height (pixel units)
    pub h: u32,
    /// Rotation angle (radians)
    pub angle: f32,
    /// Rotation center X coordinate
    pub cx: f32,
    /// Rotation center Y coordinate
    pub cy: f32,
}

/// Adapter Base Data Structure
/// 
/// Contains common base information and OpenGL rendering resources shared by all adapters.
/// Uses conditional compilation to ensure only necessary features are included on each platform.
pub struct AdapterBase {
    /// Game name identifier
    pub game_name: String,
    /// Project path (for asset loading)
    pub project_path: String,
    /// Window title
    pub title: String,
    /// Logical grid width (character count)
    pub cell_w: u16,
    /// Logical grid height (character count)
    pub cell_h: u16,
    /// Pixel width (actual render size)
    pub pixel_w: u32,
    /// Pixel height (actual render size)
    pub pixel_h: u32,
    /// X-axis scale ratio (symbol width scaling)
    pub ratio_x: f32,
    /// Y-axis scale ratio (symbol height scaling)
    pub ratio_y: f32,
    /// Random number generator (for logo animation, etc.)
    pub rd: Rand,
    
    // OpenGL rendering mode fields
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    /// Render flag: true=use render textures, false=direct rendering
    pub rflag: bool,
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    /// Render buffer storing RenderCells to be rendered
    pub rbuf: Vec<RenderCell>,
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    /// OpenGL context handle
    pub gl: Option<glow::Context>,
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    /// OpenGL pixel renderer, encapsulating shaders and rendering logic
    pub gl_pixel: Option<GlPixel>,
}

impl AdapterBase {
    /// Create new adapter base instance
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
            rflag: true,  // Enable render texture mode by default
            #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
            rbuf: vec![],
            #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
            gl: None,
            #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
            gl_pixel: None,
        }
    }
}

/// Adapter Interface Definition
/// 
/// Defines the essential methods that all rendering adapters must implement.
/// Through this interface, game logic can be decoupled from specific rendering
/// backends, achieving "write once, run anywhere".
/// 
/// This follows the adapter pattern described in principle.md, where the same
/// game logic works with different rendering systems.
pub trait Adapter {
    /// Initialize the adapter
    /// 
    /// # Parameters
    /// - `w`: Logical width (character count)
    /// - `h`: Logical height (character count)  
    /// - `rx`: X-axis scale ratio
    /// - `ry`: Y-axis scale ratio
    /// - `s`: Window title
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, s: String);
    
    /// Reset adapter state
    fn reset(&mut self);
    
    /// Get mutable reference to base data structure
    fn get_base(&mut self) -> &mut AdapterBase;
    
    /// Poll input events
    /// 
    /// # Parameters
    /// - `timeout`: Event polling timeout
    /// - `ev`: Output event vector
    /// 
    /// # Returns
    /// Returns true if should exit
    fn poll_event(&mut self, timeout: Duration, ev: &mut Vec<Event>) -> bool;

    /// Render one frame to screen
    /// 
    /// This is the main rendering entry point that renders current buffer,
    /// sprites, etc. to the screen. Implements the rendering pipeline
    /// described in principle.md.
    /// 
    /// # Parameters
    /// - `current_buffer`: Current frame character buffer
    /// - `previous_buffer`: Previous frame character buffer (for transitions)
    /// - `pixel_sprites`: Pixel sprite list
    /// - `stage`: Render stage (for logo display, etc.)
    fn draw_all_to_screen(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String>;

    // Configuration methods with default implementations supporting method chaining

    /// Set logical grid size
    fn set_size(&mut self, w: u16, h: u16) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.cell_w = w;
        bs.cell_h = h;
        self
    }

    /// Get current logical grid size
    fn size(&mut self) -> Rect {
        let bs = self.get_base();
        Rect::new(0, 0, bs.cell_w, bs.cell_h)
    }

    /// Set X-axis scale ratio
    fn set_ratiox(&mut self, rx: f32) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.ratio_x = rx;
        self
    }

    /// Set Y-axis scale ratio
    fn set_ratioy(&mut self, ry: f32) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.ratio_y = ry;
        self
    }

    /// Calculate and set pixel size based on current settings
    /// 
    /// Pixel size = (logical size + border) * symbol size / scale ratio
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

    /// Set window title
    fn set_title(&mut self, s: String) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.title = s;
        self
    }

    // Methods that must be implemented by specific adapters

    /// Get render width of a single character
    fn cell_width(&self) -> f32;
    /// Get render height of a single character
    fn cell_height(&self) -> f32;
    /// Hide cursor
    fn hide_cursor(&mut self) -> Result<(), String>;
    /// Show cursor
    fn show_cursor(&mut self) -> Result<(), String>;
    /// Set cursor position
    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), String>;
    /// Get current cursor position
    fn get_cursor(&mut self) -> Result<(u16, u16), String>;

    /// OpenGL Rendering Main Pipeline (SDL, Winit and Web modes)
    /// 
    /// This is the core OpenGL rendering method that renders game data to screen.
    /// Uses double buffering and render textures for performance optimization.
    /// 
    /// Implements the graphics mode rendering pipeline from principle.md:
    /// 
    /// # Rendering Flow
    /// 1. Convert Buffer and Sprites to RenderCell array (Pass 1)
    /// 2. Decide rendering mode based on rflag:
    ///    - true: Use render textures (off-screen rendering)
    ///    - false: Direct rendering to buffer
    /// 3. Final composition to screen (Pass 2)
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    fn draw_all_graph(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) {
        // Step 1: Convert all render data to RenderCell array
        let rbuf =
            self.draw_all_to_render_buffer(current_buffer, previous_buffer, pixel_sprites, stage);
        
        // Debug code - can print buffer content
        // for y in 0..24 {
        //     let mut s = "".to_string();
        //     for x in 0..24 {
        //         let cc = &current_buffer.content[y * 24 + x];
        //         s.push_str(&format!("{}.{} ", cc.tex, cc.symbol));
        //     }
        //     info!("...{}", s);
        // }
        // info!("{:?} len={}", current_buffer.content.len(), rbuf.len());
        
        if self.get_base().rflag {
            // Render texture mode: render to texture first, then composite to screen
            // Render RenderCell array to render texture 2 (main buffer)
            self.draw_render_buffer_to_texture(&rbuf, 2, false);
            // Composite render textures 2 and 3 to screen
            self.draw_render_textures_to_screen();
        } else {
            // Direct render mode: save RenderCell to buffer, handled by specific adapter
            self.get_base().rbuf = rbuf;
            // info!("rbuf len...{}", self.get_base().rbuf.len());
        }
    }

    /// Set to buffer-only rendering mode
    /// 
    /// Disable render textures and save RenderCell directly to memory buffer.
    /// Used for scenarios that need render data for post-processing.
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    fn only_render_buffer(&mut self) {
        self.get_base().rflag = false;
    }

    /// Composite render textures to screen
    /// 
    /// This is the final step of the rendering pipeline, responsible for compositing
    /// multiple render textures to the final screen. Supports two render textures:
    /// - Texture 2: Main buffer (game content)
    /// - Texture 3: Transition effect layer
    /// 
    /// # Composition Order
    /// 1. Bind screen as render target
    /// 2. Draw main buffer texture (fullscreen)
    /// 3. Draw transition effect texture (specific region)
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    fn draw_render_textures_to_screen(&mut self) {
        let bs = self.get_base();

        if let (Some(pix), Some(gl)) = (&mut bs.gl_pixel, &mut bs.gl) {
            // Bind screen framebuffer as render target
            pix.bind_screen(gl);
            let c = GlColor::new(1.0, 1.0, 1.0, 1.0);

            // Draw render texture 2 (main buffer) to screen
            if !pix.get_render_texture_hidden(2) {
                let t = GlTransform::new();
                // Fullscreen draw [0.0, 0.0, 1.0, 1.0] represents entire screen area
                pix.draw_general2d(gl, 2, [0.0, 0.0, 1.0, 1.0], &t, &c);
            }

            // Draw render texture 3 (transition effect layer)
            if !pix.get_render_texture_hidden(3) {
                let pcw = pix.canvas_width as f32;
                let pch = pix.canvas_height as f32;
                let rx = bs.ratio_x;
                let ry = bs.ratio_y;
                // Calculate transition effect display area (40x25 character region)
                let pw = 40.0 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
                let ph = 25.0 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;

                // Set scale transform to fit display area
                let mut t2 = GlTransform::new();
                t2.scale(pw / pcw, ph / pch);
                // Draw to bottom-left corner of screen
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

    /// Render Buffer to specified render texture
    /// 
    /// A convenience method that converts Buffer to RenderCell then renders to texture.
    /// Mainly used for scenarios that need to render a single buffer separately.
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    fn draw_buffer_to_texture(&mut self, buf: &Buffer, rtidx: usize) {
        let rbuf = self.buffer_to_render_buffer(buf);
        // Can set red background for debugging
        // self.draw_render_buffer(&rbuf, rtidx, true);
        self.draw_render_buffer_to_texture(&rbuf, rtidx, false);
    }

    /// Render RenderCell array to specified render texture
    /// 
    /// This is the core off-screen rendering method that draws RenderCell arrays
    /// into OpenGL textures.
    /// 
    /// # Parameters
    /// - `rbuf`: RenderCell array
    /// - `rtidx`: Render texture index
    /// - `debug`: Whether to enable debug mode (red background)
    /// 
    /// # Rendering Flow
    /// 1. Bind target render texture
    /// 2. Set clear color
    /// 3. Clear texture content
    /// 4. Render RenderCell array
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    fn draw_render_buffer_to_texture(&mut self, rbuf: &[RenderCell], rtidx: usize, debug: bool) {
        let bs = self.get_base();
        let rx = bs.ratio_x;
        let ry = bs.ratio_y;
        if let (Some(pix), Some(gl)) = (&mut bs.gl_pixel, &mut bs.gl) {
            // Bind target render texture
            pix.bind_target(gl, rtidx);
            if debug {
                // Debug mode: set red background for identification
                pix.set_clear_color(GlColor::new(1.0, 0.0, 0.0, 1.0));
            } else {
                // Normal mode: black background
                pix.set_clear_color(GlColor::new(0.0, 0.0, 0.0, 1.0));
            }
            // Clear texture content
            pix.clear(gl);
            // Render RenderCell array to texture
            pix.render_rbuf(gl, rbuf, rx, ry);
        }
    }

    /// Convert Buffer to RenderCell array
    /// 
    /// This is the core data conversion method that transforms game logic's character
    /// buffer into GPU-friendly RenderCell format for subsequent OpenGL rendering.
    /// 
    /// # Conversion Process
    /// 1. Create empty RenderCell array
    /// 2. Define render callback function to convert each character to RenderCell
    /// 3. Call render_main_buffer for conversion
    /// 4. Return converted RenderCell array
    #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
    fn buffer_to_render_buffer(&mut self, cb: &Buffer) -> Vec<RenderCell> {
        let mut rbuf = vec![];
        let rx = self.get_base().ratio_x;
        let ry = self.get_base().ratio_y;
        let pz = PointI32 { x: 0, y: 0 };  // No rotation center
        
        // Define render callback function: convert each character to RenderCell
        let mut rfunc = |fc: &(u8, u8, u8, u8),      // Foreground color
                         bc: &Option<(u8, u8, u8, u8)>, // Background color
                         _s0: ARect,                    // Unused rect parameter
                         _s1: ARect,                    // Unused rect parameter
                         s2: ARect,                     // Target render rect
                         texidx: usize,                 // Texture index
                         symidx: usize| {               // Symbol index
            push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, 0.0, &pz);
        };
        
        // Render main buffer including border
        render_main_buffer(cb, cb.area.width, rx, ry, true, &mut rfunc);
        rbuf
    }

    /// Convert all render elements to RenderCell array
    /// 
    /// This is the core method of the rendering system, responsible for converting
    /// all visual elements of the game (Buffer, Sprites, etc.) into RenderCell arrays
    /// that can be directly processed by the GPU.
    /// 
    /// Implements Graphics Mode Pass 1 from principle.md:
    /// 
    /// # Render Layers (bottom to top)
    /// 1. **Logo Layer**: RustPixel Logo animation during startup
    /// 2. **Border Layer**: Window border (SDL and Winit modes only)
    /// 3. **Main Buffer Layer**: Main game content
    /// 4. **Sprite Layer**: Dynamic pixel sprite objects
    /// 
    /// # Parameters
    /// - `cb`: Current frame character buffer
    /// - `_pb`: Previous frame character buffer (unused currently)
    /// - `ps`: Pixel sprite list
    /// - `stage`: Render stage for controlling logo display
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
        let pz = PointI32 { x: 0, y: 0 };  // Default rotation center

        // Layer 1: Render Logo (startup stage only)
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
            return rbuf;  // Logo stage only renders logo, return directly
        }

        // Get basic render parameters
        let cw = self.get_base().cell_w;
        let ch = self.get_base().cell_h;
        let rx = self.get_base().ratio_x;
        let ry = self.get_base().ratio_y;
        
        // Define common render callback function
        let mut rfunc = |fc: &(u8, u8, u8, u8),      // Foreground color
                         bc: &Option<(u8, u8, u8, u8)>, // Background color
                         _s0: ARect,                    // Unused rect parameter
                         _s1: ARect,                    // Unused rect parameter
                         s2: ARect,                     // Target render rect
                         texidx: usize,                 // Texture index
                         symidx: usize| {               // Symbol index
            push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, 0.0, &pz);
        };

        // Layer 2: Render window border (SDL and Winit modes only)
        #[cfg(any(feature = "sdl", feature = "winit"))]
        render_border(cw, ch, rx, ry, &mut rfunc);

        // Layer 3: Render main buffer (game content)
        if stage > LOGO_FRAME {
            render_main_buffer(cb, width, rx, ry, false, &mut rfunc);
        }

        // Layer 4: Render pixel sprites (dynamic elements)
        if stage > LOGO_FRAME {
            for item in ps {
                // Only render visible pixel sprites
                if item.is_pixel && !item.is_hidden {
                    render_pixel_sprites(
                        item,
                        rx,
                        ry,
                        |fc, bc, _s0, _s1, s2, texidx, symidx, angle, ccp| {
                            // Sprites support rotation, use angle and ccp parameters
                            push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, angle, &ccp);
                        },
                    );
                }
            }
        }
        rbuf
    }

    /// Get Any type reference for downcasting
    fn as_any(&mut self) -> &mut dyn Any;
}

/// Push rendering data into RenderCell buffer
/// 
/// This is a core data conversion function that transforms game logic rendering parameters
/// into GPU-friendly RenderCell format. It handles color conversion, texture coordinate
/// calculation, and rotation angle processing.
/// 
/// Based on principle.md graphics mode design, this function bridges the gap between
/// game logic data and OpenGL rendering format.
/// 
/// # Parameters
/// - `rbuf`: Target RenderCell buffer
/// - `fc`: Foreground color (RGBA, 0-255 range)
/// - `bgc`: Background color (optional, RGBA, 0-255 range)
/// - `texidx`: Texture index (0-63)
/// - `symidx`: Symbol index (0-255)
/// - `s`: Render rectangle area
/// - `angle`: Rotation angle (degrees)
/// - `ccp`: Rotation center point
/// 
/// # Texture Coordinate Calculation
/// RustPixel uses an 8x8 texture block layout, each block contains 16x16 symbols:
/// ```text
/// ┌─────────────────────────────────────────────────┐
/// │ Texture Layout (128x128 symbols total)         │
/// │                                                 │
/// │  Block(0,0)  Block(1,0)  ...  Block(7,0)      │
/// │  16x16 syms  16x16 syms       16x16 syms       │
/// │                                                 │
/// │  Block(0,1)  Block(1,1)  ...  Block(7,1)      │
/// │  16x16 syms  16x16 syms       16x16 syms       │
/// │                                                 │
/// │     ...         ...      ...     ...           │
/// │                                                 │
/// │  Block(0,7)  Block(1,7)  ...  Block(7,7)      │
/// │  16x16 syms  16x16 syms       16x16 syms       │
/// └─────────────────────────────────────────────────┘
/// ```
/// - Total texture blocks: 64 (8x8)
/// - Symbols per block: 256 (16x16)
/// - Total symbols: 16384 (128x128)
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
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
        // Convert foreground color from 0-255 range to 0.0-1.0 range
        fcolor: (
            fc.0 as f32 / 255.0,
            fc.1 as f32 / 255.0,
            fc.2 as f32 / 255.0,
            fc.3 as f32 / 255.0,
        ),
        ..Default::default()
    };
    
    // Set background color (if any)
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
    
    // Calculate symbol coordinates in texture
    // Texture layout: 8x8 blocks, each block 16x16 symbols
    let x = symidx as u32 % 16u32 + (texidx as u32 % 8u32) * 16u32;
    let y = symidx as u32 / 16u32 + (texidx as u32 / 8u32) * 16u32;
    // Convert to linear index
    wc.texsym = (y * 16u32 * 8u32 + x) as usize;
    
    // Set render position (add border offset)
    wc.x = s.x as f32 + PIXEL_SYM_WIDTH.get().expect("lazylock init");
    wc.y = s.y as f32 + PIXEL_SYM_HEIGHT.get().expect("lazylock init");
    wc.w = s.w;
    wc.h = s.h;
    
    // Handle rotation angle
    if angle == 0.0 {
        wc.angle = angle as f32;
    } else {
        // Convert angle from degrees to radians and normalize to [0, 2π] range
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
    
    // Set rotation center point
    wc.cx = ccp.x as f32;
    wc.cy = ccp.y as f32;
    rbuf.push(wc);
}

/// Render Helper Function
/// 
/// A utility function that calculates rendering rectangles and texture coordinates
/// for different types of rendering elements. This function bridges the gap between
/// logical character positions and physical pixel positions.
/// 
/// # Parameters
/// - `cell_w`: Cell width
/// - `r`: Scale ratio point
/// - `i`: Character index
/// - `sh`: Character information (symbol, texture, fg/bg colors)
/// - `p`: Position offset
/// - `is_border`: Whether this is border rendering
/// 
/// # Returns
/// Tuple containing:
/// - Background symbol rectangle in texture
/// - Symbol rectangle in texture  
/// - Destination rectangle in render texture
/// - Texture ID
/// - Symbol ID
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

/// Render Pixel Sprites
/// 
/// Renders pixel sprites from Sprites objects into RenderCells. Sprites support
/// complex transformation effects including rotation, scaling, and color changes.
/// 
/// This implements the Pixel Sprites rendering layer from principle.md graphics mode,
/// where pixel sprites are rendered separately from regular sprites and support
/// pixel-level movement and transparency.
/// 
/// # Key Features
/// - **Pixel-level precision**: Unlike regular sprites constrained to character grid
/// - **Transformation support**: Rotation, scaling, alpha blending
/// - **Render ordering**: Sprites sorted by render weight for proper layering
/// - **Transparency**: Alpha channel support for translucent effects
/// 
/// # Parameters
/// - `pixel_spt`: Pixel sprite object
/// - `rx`: X-axis scale ratio
/// - `ry`: Y-axis scale ratio
/// - `f`: Render callback function that receives render parameters
/// 
/// # Callback Function Parameters
/// - Foreground color (RGBA)
/// - Background color (optional RGBA)
/// - Background rect, symbol rect, destination rect
/// - Texture index, symbol index
/// - Rotation angle, rotation center point
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub fn render_pixel_sprites<F>(pixel_spt: &mut Sprites, rx: f32, ry: f32, mut f: F)
where
    // rgba, back rgba, back rect, sym rect, dst rect, tex, sym, angle, center point
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
    // Sort sprites by render weight for proper layering
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

        // Process each cell in the sprite
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
            
            // Calculate rotation center point relative to sprite center
            let ccp = PointI32 {
                x: ((pw as f32 / 2.0 - x as f32) * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx) as i32,
                y: ((ph as f32 / 2.0 - y as f32) * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry) as i32,
            };
            
            // Apply sprite alpha to foreground color
            let mut fc = sh.2.get_rgba();
            fc.3 = s.alpha;
            
            // Handle background color with alpha
            let bc;
            if sh.3 != Color::Reset {
                let mut brgba = sh.3.get_rgba();
                brgba.3 = s.alpha;
                bc = Some(brgba);
            } else {
                bc = None;
            }
            
            // Call render callback with all parameters
            f(&fc, &bc, s0, s1, s2, texidx, symidx, s.angle, ccp);
        }
    }
}

/// Render Main Buffer
/// 
/// Renders the game's character buffer into RenderCells. This is the most fundamental
/// rendering function responsible for displaying the main text content of the game.
/// 
/// Implements the Main Buffer rendering from principle.md text mode, where regular
/// sprites are merged into the main buffer for background elements.
/// 
/// # Key Features
/// - **Character-based rendering**: Processes text characters and symbols
/// - **Color support**: Handles foreground and background colors
/// - **Texture mapping**: Maps characters to texture symbols
/// - **Border support**: Optional border offset for window frames
/// 
/// # Parameters
/// - `buf`: Character buffer to render
/// - `width`: Buffer width in characters
/// - `rx`: X-axis scale ratio
/// - `ry`: Y-axis scale ratio
/// - `border`: Whether to include border offset
/// - `f`: Render callback function
/// 
/// # Rendering Process
/// Iterates through each character cell in the buffer, extracts its color,
/// texture, and symbol information, then converts to rendering parameters
/// via the callback function.
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub fn render_main_buffer<F>(buf: &Buffer, width: u16, rx: f32, ry: f32, border: bool, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, ARect, ARect, usize, usize),
{
    for (i, cell) in buf.content.iter().enumerate() {
        // Extract symbol index, texture index, foreground and background colors
        let sh = cell.get_cell_info();
        let (s0, s1, s2, texidx, symidx) = render_helper(
            width,
            PointF32 { x: rx, y: ry },
            i,
            &sh,
            PointU16 { x: 0, y: 0 },
            border,
        );
        
        // Convert colors to RGBA format
        let fc = sh.2.get_rgba();
        let bc = if sh.3 != Color::Reset {
            Some(sh.3.get_rgba())
        } else {
            None
        };
        
        // Call render callback
        f(&fc, &bc, s0, s1, s2, texidx, symidx);
    }
}

/// Render Window Border
/// 
/// Draws decorative window borders for SDL and Winit modes. The border uses
/// specific symbols to create a frame around the game content, providing
/// better visual experience and platform consistency.
/// 
/// # Border Design
/// ```text
/// ┌─────────────────────────────────────────────────┐
/// │ Border Layout (character-based frame)          │
/// │                                                 │
/// │  ┌─────────────────────────────────────┐  X    │
/// │  │                                     │       │
/// │  │         Game Content Area           │       │
/// │  │                                     │       │
/// │  └─────────────────────────────────────┘       │
/// │                                                 │
/// │  Corner chars: ┌ ┐ └ ┘  Border chars: ─ │     │
/// │  Close button: X (top-right corner)            │
/// └─────────────────────────────────────────────────┘
/// ```
/// 
/// # Parameters
/// - `cell_w`: Window width in characters
/// - `cell_h`: Window height in characters
/// - `rx`: X-axis scale ratio
/// - `ry`: Y-axis scale ratio
/// - `f`: Render callback function
/// 
/// # Border Components
/// - **Top border**: Horizontal lines with close button
/// - **Side borders**: Vertical lines
/// - **Corners**: Special corner characters
/// - **Close button**: 'X' symbol in top-right corner
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub fn render_border<F>(cell_w: u16, cell_h: u16, rx: f32, ry: f32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), &Option<(u8, u8, u8, u8)>, ARect, ARect, ARect, usize, usize),
{
    // Define border character styles
    let sh_top = (102u8, 1u8, Color::Indexed(7), Color::Reset);    // Top border character
    let sh_other = (24u8, 2u8, Color::Indexed(7), Color::Reset);   // Other border characters
    let sh_close = (214u8, 1u8, Color::Indexed(7), Color::Reset);  // Close button character

    // Render border frame
    for n in 0..cell_h as usize + 2 {
        for m in 0..cell_w as usize + 2 {
            // Skip interior cells (only draw border)
            if n != 0 && n != cell_h as usize + 1 && m != 0 && m != cell_w as usize + 1 {
                continue;
            }
            
            // Select appropriate character style
            let rsh;
            if n == 0 {
                if m as u16 <= cell_w {
                    rsh = &sh_top;     // Top border
                } else {
                    rsh = &sh_close;   // Close button
                }
            } else {
                rsh = &sh_other;       // Side borders
            }
            
            // Calculate rendering parameters
            let (s0, s1, s2, texidx, symidx) = render_helper(
                cell_w + 2,
                PointF32 { x: rx, y: ry },
                n * (cell_w as usize + 2) + m,
                rsh,
                PointU16 { x: 0, y: 0 },
                true,
            );
            
            // Render border character
            let fc = rsh.2.get_rgba();
            let bc = None;  // No background color for border
            f(&fc, &bc, s0, s1, s2, texidx, symidx);
        }
    }
}

/// Render RustPixel Logo
/// 
/// Displays the animated RustPixel logo during program startup. The logo uses
/// predefined character data and implements dynamic display effects through
/// random effects and stage control.
/// 
/// # Logo Animation Design
/// ```text
/// ┌─────────────────────────────────────────────────┐
/// │ Logo Animation Stages                           │
/// │                                                 │
/// │  Stage 1 (0 - 33%):   Gradual character reveal │
/// │  ┌─────────────────┐   - Characters fade in    │
/// │  │ R u s t P i x e │   - Random position jitter│
/// │  │ l               │   - Increasing brightness │
/// │  └─────────────────┘                           │
/// │                                                 │
/// │  Stage 2 (33% - 66%): Full color display       │
/// │  ┌─────────────────┐   - Stable positioning    │
/// │  │ R u s t P i x e │   - Full color saturation │
/// │  │ l               │   - No movement           │
/// │  └─────────────────┘                           │
/// │                                                 │
/// │  Stage 3 (66% - 100%): Fade out               │
/// │  ┌─────────────────┐   - Colors fade to black │
/// │  │ R u s t P i x e │   - Smooth transition     │
/// │  │ l               │   - Prepare for main game │
/// │  └─────────────────┘                           │
/// └─────────────────────────────────────────────────┘
/// ```
/// 
/// # Parameters
/// - `srx`: X-axis scale ratio
/// - `sry`: Y-axis scale ratio
/// - `spw`: Pixel width of display area
/// - `sph`: Pixel height of display area
/// - `rd`: Random number generator for effects
/// - `stage`: Current animation stage (0 to LOGO_FRAME)
/// - `f`: Render callback function
/// 
/// # Animation Effects
/// - **Stage 1**: Progressive character appearance with random jitter
/// - **Stage 2**: Stable full-color logo display
/// - **Stage 3**: Gradual fade-out transition
/// 
/// The logo is centered on screen and scales appropriately with the display resolution.
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub fn render_logo<F>(srx: f32, sry: f32, spw: u32, sph: u32, rd: &mut Rand, stage: u32, mut f: F)
where
    F: FnMut(&(u8, u8, u8, u8), ARect, ARect, usize, usize),
{
    let rx = srx * 1.0;
    let ry = sry * 1.0;
    
    // Process each character in the logo
    for y in 0usize..PIXEL_LOGO_HEIGHT {
        for x in 0usize..PIXEL_LOGO_WIDTH {
            let sci = y * PIXEL_LOGO_WIDTH + x;
            let symw = PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
            let symh = PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;

            // Calculate rendering parameters for this character
            let (_s0, s1, mut s2, texidx, symidx) = render_helper(
                PIXEL_LOGO_WIDTH as u16,
                PointF32 { x: rx, y: ry },
                sci,
                &(
                    PIXEL_LOGO[sci * 3],     // Symbol ID
                    PIXEL_LOGO[sci * 3 + 2], // Texture ID
                    Color::Indexed(PIXEL_LOGO[sci * 3 + 1]), // Color
                    Color::Reset,
                ),
                PointU16 {
                    // Center the logo on screen
                    x: spw as u16 / 2 - (PIXEL_LOGO_WIDTH as f32 / 2.0 * symw) as u16,
                    y: sph as u16 / 2 - (PIXEL_LOGO_HEIGHT as f32 / 2.0 * symh) as u16,
                },
                false,
            );
            
            // Get base color from logo data
            let fc = Color::Indexed(PIXEL_LOGO[sci * 3 + 1]).get_rgba();

            // Calculate animation effects based on stage
            let randadj = 12 - (rd.rand() % 24) as i32;  // Random position jitter
            let sg = LOGO_FRAME as u8 / 3;  // Stage duration
            let r: u8;
            let g: u8;
            let b: u8;
            let a: u8;
            
            if stage <= sg as u32 {
                // Stage 1: Gradual fade-in with jitter
                r = (stage as u8).saturating_mul(10);
                g = (stage as u8).saturating_mul(10);
                b = (stage as u8).saturating_mul(10);
                a = 255;
                s2.x += randadj;  // Add random jitter effect
            } else if stage <= sg as u32 * 2 {
                // Stage 2: Full color display
                r = fc.0;
                g = fc.1;
                b = fc.2;
                a = 255;
            } else {
                // Stage 3: Fade out
                let cc = (stage as u8 - sg * 2).saturating_mul(10);
                r = fc.0.saturating_sub(cc);
                g = fc.1.saturating_sub(cc);
                b = fc.2.saturating_sub(cc);
                a = 255;
            }
            
            // Render the character with calculated color
            f(&(r, g, b, a), s1, s2, texidx, symidx);
        }
    }
}

