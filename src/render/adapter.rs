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
//! │ ┌─────────────────────────────────────────────────────────┐ │
//! │ │              Message Bus + Timer                        │ │
//! │ └─────────────────────────────────────────────────────────┘ │
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
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
//! │  │   Layer 1   │  │   Layer 2   │  │   Layer N   │          │
//! │  │ (Sprites)   │  │ (Sprites)   │  │ (Sprites)   │          │
//! │  └─────────────┘  └─────────────┘  └─────────────┘          │
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
//! │  ┌─────────────────────┐    ┌─────────────────────┐         │
//! │  │  Symbols Shader     │    │  Transition Shader  │         │
//! │  │ (Instanced Render)  │    │   (Effect Mixing)   │         │
//! │  └─────────────────────┘    └─────────────────────┘         │
//! │             │                           │                   │
//! │             ▼                           ▼                   │
//! │  ┌─────────────────────┐    ┌─────────────────────┐         │
//! │  │   Render Texture    │    │   Render Texture    │         │
//! │  │      (Main)         │    │    (Transition)     │         │
//! │  └─────────────────────┘    └─────────────────────┘         │
//! │             │                           │                   │
//! │             └─────────────┬─────────────┘                   │
//! │                           ▼                                 │
//! │                ┌─────────────────────┐                      │
//! │                │  General2D Shader   │                      │
//! │                │  (Final Composite)  │                      │
//! │                └─────────────────────┘                      │
//! │                           │                                 │
//! │                           ▼                                 │
//! │                ┌─────────────────────┐                      │
//! │                │       Screen        │                      │
//! │                │    (Framebuffer)    │                      │
//! │                └─────────────────────┘                      │
//! └─────────────────────────────────────────────────────────────┘
//! ```

#![allow(unused_variables)]
use crate::{
    event::Event,
    render::{buffer::Buffer, sprite::Sprites},
    util::{Rand, Rect},
};
#[cfg(any(
    feature = "sdl",
    feature = "winit",
    feature = "wgpu",
    target_arch = "wasm32"
))]
use crate::{
    util::{ARect, PointI32},
    LOGO_FRAME,
};

#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
use crate::render::adapter::gl::{color::GlColor, transform::GlTransform};
use std::any::Any;
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

/// Winit adapter module - Cross-platform window management with OpenGL/WGPU
#[cfg(all(any(feature = "winit", feature = "wgpu"), not(target_arch = "wasm32")))]
pub mod winit;

/// Crossterm adapter module - Terminal-based text mode rendering
#[cfg(not(any(
    feature = "sdl",
    feature = "winit",
    feature = "wgpu",
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

    /// Render flag controlling immediate vs buffered rendering
    ///
    /// - true: Direct rendering to screen (normal mode)
    /// - false: Buffered rendering for external access (used for FFI/WASM)
    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
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
        #[cfg(any(
            feature = "sdl",
            feature = "winit",
            feature = "wgpu",
            target_arch = "wasm32"
        ))]
        {
            let bs = self.get_base();
            bs.gr.ratio_x = rx;
        }
        self
    }

    fn set_ratioy(&mut self, ry: f32) -> &mut Self
    where
        Self: Sized,
    {
        #[cfg(any(
            feature = "sdl",
            feature = "winit",
            feature = "wgpu",
            target_arch = "wasm32"
        ))]
        {
            let bs = self.get_base();
            bs.gr.ratio_y = ry;
        }
        self
    }

    fn set_pixel_size(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        #[cfg(any(
            feature = "sdl",
            feature = "winit",
            feature = "wgpu",
            target_arch = "wasm32"
        ))]
        {
            let bs = self.get_base();
            bs.gr.pixel_w = ((bs.cell_w + 2) as f32 * PIXEL_SYM_WIDTH.get().expect("lazylock init")
                / bs.gr.ratio_x) as u32;
            bs.gr.pixel_h = ((bs.cell_h + 2) as f32 * PIXEL_SYM_HEIGHT.get().expect("lazylock init")
                / bs.gr.ratio_y) as u32;
        }
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
    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
    fn draw_all_graph(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) {
        // Pass 1: Convert game data (buffer + sprites) to GPU-ready format
        let rbuf =
            self.generate_render_buffer(current_buffer, previous_buffer, pixel_sprites, stage);

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

    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
    fn only_render_buffer(&mut self) {
        self.get_base().gr.rflag = false;
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
    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
    fn draw_render_textures_to_screen(&mut self) {
        #[cfg(feature = "wgpu")]
        {
            use crate::render::adapter::winit::WinitAdapter;

            if let Some(winit_adapter) = self.as_any().downcast_mut::<WinitAdapter>() {
                if let Err(e) = winit_adapter.draw_render_textures_to_screen_wgpu() {
                    eprintln!("WGPU draw_render_textures_to_screen error: {}", e);
                }
                return;
            }
        }

        #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
        {
            // OpenGL mode implementation
            let bs = self.get_base();

            if let (Some(pix), Some(gl)) = (&mut bs.gr.gl_pixel, &mut bs.gr.gl) {
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
                    let pcw = pix.canvas_width as f32; // Physical canvas width
                    let pch = pix.canvas_height as f32; // Physical canvas height
                    let rx = bs.gr.ratio_x; // Horizontal scaling ratio
                    let ry = bs.gr.ratio_y; // Vertical scaling ratio

                    // Calculate scaled dimensions for transition layer
                    // Use actual game area dimensions instead of hardcoded 40x25
                    // let pw = bs.cell_w as f32 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
                    // let ph = bs.cell_h as f32 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;
                    let pw = 40.0f32 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / rx;
                    let ph = 25.0f32 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ry;

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
    }

    // draw buffer to render texture - unified for both OpenGL and WGPU
    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
    fn draw_buffer_to_texture(&mut self, buf: &Buffer, rtidx: usize) {
        #[cfg(feature = "wgpu")]
        {
            use crate::render::adapter::wgpu::WgpuRender;
            use crate::render::adapter::winit::WinitAdapter;

            // First, convert buffer to render buffer to avoid borrowing conflicts
            let rbuf = self.buffer_to_render_buffer(buf);

            if let Some(winit_adapter) = self.as_any().downcast_mut::<WinitAdapter>() {
                if let (Some(wgpu_pixel_renderer), Some(device), Some(queue)) = (
                    &mut winit_adapter.wgpu_pixel_renderer,
                    &winit_adapter.wgpu_device,
                    &winit_adapter.wgpu_queue,
                ) {
                    // Prepare all render data first
                    wgpu_pixel_renderer.prepare_draw(device, queue); // Upload base vertices, indices, uniforms
                    wgpu_pixel_renderer.prepare_draw_with_render_cells(device, queue, &rbuf); // Upload instance data

                    // Create command encoder for rendering to texture
                    let mut encoder =
                        device.create_command_encoder(&::wgpu::CommandEncoderDescriptor {
                            label: Some("Buffer to Texture Encoder"),
                        });

                    // Render to the specified render texture
                    {
                        let render_pass_result =
                            wgpu_pixel_renderer.begin_render_to_texture(&mut encoder, rtidx);
                        if let Ok(mut render_pass) = render_pass_result {
                            // begin_render_to_texture already sets up the pipeline, buffers, and bind groups
                            // Just perform the instanced draw call
                            render_pass.draw_indexed(
                                0..6,
                                0,
                                0..wgpu_pixel_renderer.get_instance_count(),
                            );
                        }
                        // render_pass is automatically dropped here
                    }

                    // Now we can safely finish the encoder
                    queue.submit(std::iter::once(encoder.finish()));
                    return;
                }
            }
        }

        #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
        {
            let rbuf = self.buffer_to_render_buffer(buf);
            // For debug...
            // self.draw_render_buffer(&rbuf, rtidx, true);
            self.draw_render_buffer_to_texture(&rbuf, rtidx, false);
        }
    }

    // draw render buffer to render texture - unified for both OpenGL and WGPU
    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
    fn draw_render_buffer_to_texture(&mut self, rbuf: &[RenderCell], rtidx: usize, debug: bool) {
        #[cfg(feature = "wgpu")]
        {
            use crate::render::adapter::winit::WinitAdapter;

            if let Some(winit_adapter) = self.as_any().downcast_mut::<WinitAdapter>() {
                if let Err(e) = winit_adapter.draw_render_buffer_to_texture_wgpu(rbuf, rtidx, debug)
                {
                    eprintln!("WGPU draw_render_buffer_to_texture error: {}", e);
                }
                return;
            }
        }

        #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
        {
            // OpenGL mode implementation
            let bs = self.get_base();
            let rx = bs.gr.ratio_x;
            let ry = bs.gr.ratio_y;
            if let (Some(pix), Some(gl)) = (&mut bs.gr.gl_pixel, &mut bs.gr.gl) {
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
    }

    // buffer to render buffer - unified for both OpenGL and WGPU
    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
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
        render_main_buffer(cb, cb.area.width, rx, ry, true, &mut rfunc);
        rbuf
    }

    // merge main buffer & pixel sprites to render buffer...
    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
    fn generate_render_buffer(
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
                self.get_base().gr.ratio_x,
                self.get_base().gr.ratio_y,
                self.get_base().gr.pixel_w,
                self.get_base().gr.pixel_h,
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
        let rx = self.get_base().gr.ratio_x;
        let ry = self.get_base().gr.ratio_y;
        let mut rfunc = |fc: &(u8, u8, u8, u8),
                         bc: &Option<(u8, u8, u8, u8)>,
                         _s0: ARect,
                         _s1: ARect,
                         s2: ARect,
                         texidx: usize,
                         symidx: usize| {
            push_render_buffer(&mut rbuf, fc, bc, texidx, symidx, s2, 0.0, &pz);
        };

        // render windows border, for sdl, winit and wgpu mode
        // #[cfg(any(feature = "sdl", feature = "winit", feature = "wgpu"))]
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

// Re-export graph rendering functions and data structures
#[cfg(any(
    feature = "sdl",
    feature = "winit",
    feature = "wgpu",
    target_arch = "wasm32"
))]
pub use crate::render::graph::{
    Graph, init_sym_height, init_sym_width, push_render_buffer, render_border, render_logo,
    render_main_buffer, render_pixel_sprites, RenderCell, PIXEL_LOGO, PIXEL_LOGO_HEIGHT,
    PIXEL_LOGO_WIDTH, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, PIXEL_TEXTURE_FILE,
};

