// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! # Render Adapter Module
//!
//! This module defines the render adapter architecture for RustPixel, providing unified
//! rendering interfaces across different platforms and rendering backends.
//!
//! ## 🏗️ Architecture Overview (Post-WGPU Refactor)
//!
//! After the major WGPU refactor, RustPixel now uses a **direct concrete type architecture**
//! instead of trait objects, providing better performance and clearer code organization.
//!
//! ### Supported Rendering Backends
//!
//! #### TextMode
//! - **CrosstermAdapter**: Terminal text-mode rendering with crossterm
//!
//! #### GraphicsMode  
//! - **SdlAdapter**: Desktop platform based on SDL2 + OpenGL
//! - **WinitGlowAdapter**: Cross-platform OpenGL rendering (winit + glutin + glow)
//! - **WinitWgpuAdapter**: Modern GPU rendering (winit + wgpu)
//! - **WebAdapter**: WebGL-based browser rendering
//!
//! ### 🚀 Performance Improvements
//!
//! The refactor eliminated trait object overhead by:
//! - **Direct method calls** instead of dynamic dispatch
//! - **Compile-time polymorphism** via conditional compilation
//! - **Zero-cost abstractions** for better performance
//! - **Unified rendering interface** (`draw_all_graph`) across all backends
//!
//! ### 📁 Improved Code Organization
//!
//! ```text
//! src/render/adapter/
//! ├── mod.rs                    # This file - adapter definitions
//! ├── cross_adapter.rs          # Terminal rendering (crossterm)
//! ├── sdl_adapter.rs            # SDL2 + OpenGL desktop rendering  
//! ├── web_adapter.rs            # WebGL browser rendering
//! ├── winit_common.rs           # Shared winit utilities
//! ├── winit_glow_adapter.rs     # Winit + OpenGL rendering
//! ├── winit_wgpu_adapter.rs     # Winit + WGPU modern rendering
//! ├── gl/                       # OpenGL backend implementation
//! └── wgpu/                     # WGPU backend implementation
//! ```
//!
//! ## 🔄 Unified Rendering Pipeline
//!
//! All graphics adapters now share a common rendering flow:
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
//! │ ┌─────────────────────────────────────────────────────────┐ │
//! │ │            Unified Message Bus + Timer                  │ │
//! │ └─────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//!          │
//!          ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                Direct Adapter Dispatch                      │
//! │ ┌─────────────┬─────────────┬─────────────┬─────────────┐   │
//! │ │    SDL      │   Winit     │    Web      │ Crossterm   │   │
//! │ │   Adapter   │  Adapters   │  Adapter    │  Adapter    │   │
//! │ │             │ (Glow/WGPU) │             │             │   │
//! │ └─────────────┴─────────────┴─────────────┴─────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 🎨 Graphics Rendering Pipeline
//!
//! The graphics mode uses a sophisticated two-pass rendering system:
//!
//! ### Pass 1: Buffer to RenderCell Conversion
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   Graphics Mode Pass 1                      │
//! │                                                             │
//! │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐      │
//! │  │   Buffer    │    │   Sprites   │    │    Logo     │      │
//! │  │(Characters) │    │ (Objects)   │    │ (Startup)   │      │
//! │  └─────────────┘    └─────────────┘    └─────────────┘      │
//! │         │                 │                    │            │
//! │         └─────────────────┼────────────────────┘            │
//! │                           ▼                                 │
//! │              ┌─────────────────────┐                        │
//! │              │   RenderCell Array  │                        │
//! │              │  (GPU-ready Data)   │                        │
//! │              └─────────────────────┘                        │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Pass 2: GPU Rendering (Backend-Specific)
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   Graphics Mode Pass 2                      │
//! │                                                             │
//! │              ┌─────────────────────┐                        │
//! │              │   RenderCell Array  │                        │
//! │              └─────────────────────┘                        │
//! │                          │                                  │
//! │                          ▼                                  │
//! │          ┌─────────────────────────────────┐                │
//! │          │      Backend-Specific           │                │
//! │          │       GPU Rendering             │                │
//! │          │                                 │                │
//! │          │  ┌────────────────────────────┐ │                │
//! │          │  │    OpenGL (SDL/Winit)      │ │                │
//! │          │  │  - Vertex Arrays           │ │                │
//! │          │  │  - Shader Programs         │ │                │
//! │          │  │  - Texture Atlases         │ │                │
//! │          │  └────────────────────────────┘ │                │
//! │          │                                 │                │
//! │          │  ┌────────────────────────────┐ │                │
//! │          │  │      WGPU (Modern)         │ │                │
//! │          │  │  - Render Pipelines        │ │                │
//! │          │  │  - Command Buffers         │ │                │
//! │          │  │  - Bind Groups             │ │                │
//! │          │  └────────────────────────────┘ │                │
//! │          └─────────────────────────────────┘                │
//! │                          │                                  │
//! │                          ▼                                  │
//! │              ┌─────────────────────┐                        │
//! │              │     Final Frame     │                        │
//! │              │   (Swap Buffers)    │                        │
//! │              └─────────────────────┘                        │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 🎮 Advanced Features
//!
//! ### Render Textures (Off-screen Rendering)
//! All graphics adapters support render-to-texture for advanced effects:
//! - **Transition effects** between game states
//! - **Post-processing** and visual effects  
//! - **Multi-pass rendering** for complex scenes
//!
//! ### High-DPI Support
//! Automatic handling of Retina/High-DPI displays:
//! - **Logical vs Physical pixels** conversion
//! - **Scale factor** detection and adjustment
//! - **Crisp rendering** on all display types
//!
//! ### Cross-Platform Input
//! Unified input handling via `winit_common` module:
//! - **Window dragging** for borderless windows
//! - **Mouse and keyboard** event translation
//! - **Custom cursor** support
//!
//! ## 🔧 Configuration & Compilation
//!
//! Backend selection via cargo features:
//! ```toml
//! # Default: Terminal mode
//! rust_pixel = "0.1"
//!
//! # SDL desktop mode  
//! rust_pixel = { version = "0.1", features = ["sdl"] }
//!
//! # Winit + OpenGL mode
//! rust_pixel = { version = "0.1", features = ["winit"] }
//!
//! # Winit + WGPU mode (cutting-edge)
//! rust_pixel = { version = "0.1", features = ["wgpu"] }
//! ```

#![allow(unused_variables)]
#[cfg(any(
    feature = "sdl",
    feature = "winit",
    feature = "wgpu",
    target_arch = "wasm32"
))]
use crate::util::{ARect, PointI32};
use crate::{
    event::Event,
    render::{buffer::Buffer, sprite::Sprites},
    util::{Rand, Rect},
};

use std::any::Any;
use std::time::Duration;
// use log::info;

/// OpenGL rendering subsystem for winit, SDL and web modes
#[cfg(any(sdl_backend, winit_backend, wasm))]
pub mod gl;

/// WGPU rendering subsystem - modern GPU API for cross-platform rendering
#[cfg(feature = "wgpu")]
pub mod wgpu;

/// SDL adapter module - Desktop rendering backend based on SDL2
#[cfg(sdl_backend)]
pub mod sdl_adapter;

/// Web adapter module - WebGL-based browser rendering backend
#[cfg(wasm)]
pub mod web_adapter;

/// Winit common module - Shared code between winit_glow and winit_wgpu adapters
#[cfg(any(
    all(feature = "winit", not(feature = "wgpu"), not(target_arch = "wasm32")),
    all(feature = "wgpu", not(target_arch = "wasm32"))
))]
pub mod winit_common;

/// Winit + Glow adapter module - OpenGL backend with winit window management
#[cfg(winit_backend)]
pub mod winit_glow_adapter;

/// Winit + WGPU adapter module - Modern GPU backend with winit window management  
#[cfg(wgpu_backend)]
pub mod winit_wgpu_adapter;

/// Crossterm adapter module - Terminal-based text mode rendering
#[cfg(not(any(
    feature = "sdl",
    feature = "winit",
    feature = "wgpu",
    target_os = "android",
    target_os = "ios",
    target_arch = "wasm32"
)))]
pub mod cross_adapter;

// Re-export graph rendering functions and data structures
#[cfg(any(
    feature = "sdl",
    feature = "winit",
    feature = "wgpu",
    target_arch = "wasm32"
))]
pub use crate::render::graph::{
    generate_render_buffer, init_sym_height, init_sym_width, push_render_buffer, render_border,
    render_logo, render_main_buffer, render_pixel_sprites, Graph, RenderCell, PIXEL_LOGO,
    PIXEL_LOGO_HEIGHT, PIXEL_LOGO_WIDTH, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, PIXEL_TEXTURE_FILE,
};

/// Path to the symbols texture file
///
/// The symbols texture contains 8x8 blocks where each block contains 16x16 symbols,
/// totaling 128 × 128 symbols. This texture serves as a character atlas for rendering
/// text and symbols in graphics mode.
///
/// Layout:
/// ```text
/// ┌────────────────────────────────────────────────────────────┐
/// │                   Symbols Texture (128×128)                │
/// │  ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐         │
/// │  │Block│Block│Block│Block│Block│Block│Block│Block│         │
/// │  │ 0,0 │ 1,0 │ 2,0 │ 3,0 │ 4,0 │ 5,0 │ 6,0 │ 7,0 │         │
/// │  │16×16│16×16│16×16│16×16│16×16│16×16│16×16│16×16│         │
/// │  ├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤         │
/// │  │Block│Block│Block│Block│Block│Block│Block│Block│         │
/// │  │ 0,1 │ 1,1 │ 2,1 │ 3,1 │ 4,1 │ 5,1 │ 6,1 │ 7,1 │         │
/// │  │16×16│16×16│16×16│16×16│16×16│16×16│16×16│16×16│         │
/// │  ├─────┼─────┼─────┼─────┼─────┼─────┼─────┼─────┤         │
/// │  │  ⋮  │  ⋮   │  ⋮  │  ⋮  │  ⋮   │  ⋮  │  ⋮  │  ⋮  │         │
/// │  └─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┘         │
/// └────────────────────────────────────────────────────────────┘
/// ```
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

    /// Random number generator for effects and animations
    pub rd: Rand,

    /// Datas using by graph mode
    #[cfg(graphics_mode)]
    pub gr: Graph,
}

impl AdapterBase {
    pub fn new(gn: &str, project_path: &str) -> Self {
        Self {
            game_name: gn.to_string(),
            project_path: project_path.to_string(),
            title: "".to_string(),
            cell_w: 0,
            cell_h: 0,
            rd: Rand::new(),
            #[cfg(any(
                feature = "sdl",
                feature = "winit",
                feature = "wgpu",
                target_arch = "wasm32"
            ))]
            gr: Graph::new(),
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
/// │                    Game Application                         │
/// │                                                             │
/// │  1. init() ──────────► Initialize renderer                  │
/// │  2. Loop:                                                   │
/// │     ├── poll_event() ─► Handle input events                 │
/// │     ├── (game logic) ──► Update game state                  │
/// │     └── draw_all() ──► Render frame                         │
/// │  3. (cleanup) ────────► Automatic cleanup on drop           │
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
    fn draw_all(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String>;

    /// Post draw process
    fn post_draw(&mut self);

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

    fn set_title(&mut self, s: String) -> &mut Self
    where
        Self: Sized,
    {
        let bs = self.get_base();
        bs.title = s;
        self
    }

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
    /// │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐      │
    /// │  │   Buffer    │    │   Sprites   │    │    Logo     │      │
    /// │  │             │    │             │    │             │      │
    /// │  └─────────────┘    └─────────────┘    └─────────────┘      │
    /// │         │                 │                    │            │
    /// │         └─────────────────┼────────────────────┘            │
    /// │                           ▼                                 │
    /// │                ┌───────────────────────┐                    │
    /// │                │ generate_render_buffer│                    │
    /// │                └───────────────────────┘                    │
    /// │                           │                                 │
    /// │                           ▼                                 │
    /// │                ┌─────────────────────┐                      │
    /// │                │Vec<RenderCell> rbuf │                      │
    /// │                └─────────────────────┘                      │
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
    #[cfg(graphics_mode)]
    fn draw_all_graph(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) {
        // Pass 1: Convert game data (buffer + sprites) to GPU-ready format
        let rbuf = generate_render_buffer(
            current_buffer,
            previous_buffer,
            pixel_sprites,
            stage,
            self.get_base(),
        );

        // Pass 2: Render to screen or buffer based on mode
        if self.get_base().gr.rflag {
            // Both OpenGL and WGPU use the same unified rendering pipeline
            // 1. Draw RenderCell array to render_texture 2 (main scene)
            self.draw_render_buffer_to_texture(&rbuf, 2, false);
            // 2. Composite render_texture 2 & 3 to screen (final output)
            self.draw_render_textures_to_screen();
        } else {
            // Buffered mode: Store render data for external access
            // Used by FFI interfaces and WASM exports to access raw render data
            self.get_base().gr.rbuf = rbuf;
        }
    }

    // draw buffer to render texture - unified for both OpenGL and WGPU
    #[cfg(graphics_mode)]
    fn draw_buffer_to_texture(&mut self, buf: &Buffer, rtidx: usize) {
        // Convert buffer to render buffer first
        let rbuf = self.buffer_to_render_buffer(buf);

        // Then draw render buffer to texture
        self.draw_render_buffer_to_texture(&rbuf, rtidx, false);
    }

    /// Graphics mode render buffer to texture - abstract method
    ///
    /// Each graphics adapter must implement this method to render RenderCell data
    /// to the specified render texture. This method is only available in graphics modes.
    ///
    /// # Parameters  
    /// - `rbuf`: Array of RenderCell data (GPU-ready format)
    /// - `rtidx`: Target render texture index (typically 2 for main scene, 3 for transitions)
    /// - `debug`: Enable debug mode rendering (colored backgrounds for debugging)
    #[cfg(graphics_mode)]
    fn draw_render_buffer_to_texture(&mut self, rbuf: &[RenderCell], rtidx: usize, debug: bool);

    // buffer to render buffer - unified for both OpenGL and WGPU
    #[cfg(graphics_mode)]
    fn buffer_to_render_buffer(&mut self, cb: &Buffer) -> Vec<RenderCell> {
        let mut rbuf = vec![];
        let rx = self.get_base().gr.ratio_x;
        let ry = self.get_base().gr.ratio_y;
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

        render_main_buffer(cb, cb.area.width, rx, ry, false, &mut rfunc);

        rbuf
    }

    #[cfg(graphics_mode)]
    fn only_render_buffer(&mut self) {
        self.get_base().gr.rflag = false;
    }

    /// Render texture composition to screen - final rendering stage
    ///
    /// This method performs the final composite rendering step, combining multiple
    /// render textures into the final screen output. It handles layer composition,
    /// scaling for different display ratios, and transition effects.
    ///
    /// ## Unified Implementation for Both WGPU and OpenGL
    ///
    /// Both WGPU and OpenGL modes now use the same rendering logic through the
    /// unified PixelRenderer interface. The main differences are:
    /// - OpenGL uses `RenderContext::OpenGL`
    /// - WGPU uses `RenderContext::Wgpu` with additional surface management
    ///
    /// ## Rendering Order and Layers
    /// ```text
    /// ┌─────────────────────────────────────────────────────────────┐
    /// │                    Screen Composition                       │
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
    #[cfg(graphics_mode)]
    fn draw_render_textures_to_screen(&mut self);

    fn as_any(&mut self) -> &mut dyn Any;

    /// Advanced rendering methods for special effects (petview, transitions, etc.)
    /// These methods provide unified high-level interfaces for graphics modes.

    /// Set render texture visibility
    ///
    /// Controls whether a specific render texture is visible in the final composition.
    /// This is used for advanced effects like transitions and overlays.
    ///
    /// # Parameters
    /// - `texture_index`: Render texture index (0-3, typically 2=main, 3=effects)
    /// - `visible`: Whether the texture should be visible
    #[cfg(graphics_mode)]
    fn set_render_texture_visible(&mut self, texture_index: usize, visible: bool) {
        // Default implementation for graphics modes
        // Each adapter can override this with optimized implementations
    }

    /// Render a simple transition effect
    ///
    /// Performs a basic transition rendering to the specified render texture.
    /// This is used for fade-in/fade-out effects and simple transitions.
    ///
    /// # Parameters
    /// - `target_texture`: Target render texture index
    #[cfg(graphics_mode)]
    fn render_simple_transition(&mut self, target_texture: usize) {
        // Default implementation - no effect
        // Graphics adapters should override this
    }

    /// Render an advanced transition effect with parameters
    ///
    /// Performs complex transition rendering with customizable effects and progress.
    /// Supports various shader-based transition effects like dissolve, wipe, etc.
    ///
    /// # Parameters
    /// - `target_texture`: Target render texture index
    /// - `effect_type`: Transition effect type (0=dissolve, 1=wipe, etc.)
    /// - `progress`: Transition progress from 0.0 to 1.0
    #[cfg(graphics_mode)]
    fn render_advanced_transition(
        &mut self,
        target_texture: usize,
        effect_type: usize,
        progress: f32,
    ) {
        // Default implementation - fallback to simple transition
        self.render_simple_transition(target_texture);
    }

    /// Get canvas size for advanced rendering calculations
    ///
    /// Returns the actual canvas/viewport size for coordinate calculations.
    /// Used by applications that need to perform custom rendering calculations.
    ///
    /// # Returns
    /// (width, height) tuple in pixels
    #[cfg(graphics_mode)]
    fn get_canvas_size(&self) -> (u32, u32) {
        let base = unsafe { &*(self as *const Self as *const AdapterBase) };
        (base.gr.pixel_w as u32, base.gr.pixel_h as u32)
    }

    /// Setup buffer transition rendering
    ///
    /// Prepares the rendering pipeline for complex buffer transition effects.
    /// This method handles adapter-specific setup for advanced image processing
    /// like distortion effects, noise generation, and multi-pass rendering.
    ///
    /// # Parameters
    /// - `target_texture`: Target render texture index for transition effects
    #[cfg(graphics_mode)]
    fn setup_buffer_transition(&mut self, target_texture: usize) {
        // Default implementation - no special setup needed
        // Graphics adapters can override this with optimized implementations
    }
}
