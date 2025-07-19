// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! # Render Adapter Module
//!
//! This module defines the render adapter architecture for RustPixel, providing unified
//! rendering interfaces across different platforms and rendering backends.
//!
//! ## Supported Rendering Backends
//! ### TextMode
//! - **Crossterm**: Terminal text-mode rendering
//!
//! ### GraphMode
//! - **SDL**: Desktop platform based on SDL2 library
//! - **Winit**: Cross-platform window management with OpenGL(winit + glow)
//! - **Wgpu**: Cross-platform window management with Wgpu(winit + wgpu)
//! - **Web**: WebGL-based browser rendering(webgl)
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

// UnifiedColor and UnifiedTransform imports removed - each adapter now handles rendering directly

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

/// Winit common module - Shared code between winit_glow and winit_wgpu adapters
#[cfg(any(
    all(feature = "winit", not(feature = "wgpu"), not(target_arch = "wasm32")),
    all(feature = "wgpu", not(target_arch = "wasm32"))
))]
pub mod winit_common;

/// Winit + Glow adapter module - OpenGL backend with winit window management
#[cfg(all(feature = "winit", not(feature = "wgpu"), not(target_arch = "wasm32")))]
pub mod winit_glow;

/// Winit + WGPU adapter module - Modern GPU backend with winit window management  
#[cfg(all(feature = "wgpu", not(target_arch = "wasm32")))]
pub mod winit_wgpu;

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

    /// Datas using by graph mode
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
    ) where
        Self: Sized,
    {
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
    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
    fn draw_buffer_to_texture(&mut self, buf: &Buffer, rtidx: usize)
    where
        Self: Sized,
    {
        // Convert buffer to render buffer first
        let rbuf = self.buffer_to_render_buffer(buf);
        
        // Then draw render buffer to texture
        self.draw_render_buffer_to_texture(&rbuf, rtidx, false);
    }

    /// Trait object compatible wrapper for draw_buffer_to_texture
    ///
    /// This method can be called on trait objects and internally handles
    /// the downcasting to call the sized implementation.
    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
    fn draw_buffer_to_texture_dyn(&mut self, buf: &Buffer, rtidx: usize) {
        // Handle each adapter type explicitly with proper feature detection
        
        // Winit + Glow adapter (OpenGL backend)
        #[cfg(all(feature = "winit", not(feature = "wgpu"), not(target_arch = "wasm32")))]
        {
            use crate::render::adapter::winit_glow::WinitGlowAdapter;
            if let Some(winit_glow_adapter) = self.as_any().downcast_mut::<WinitGlowAdapter>() {
                winit_glow_adapter.draw_buffer_to_texture(buf, rtidx);
                return;
            }
        }

        // Winit + WGPU adapter (modern GPU backend)
        #[cfg(all(feature = "wgpu", not(target_arch = "wasm32")))]
        {
            use crate::render::adapter::winit_wgpu::WinitWgpuAdapter;
            if let Some(winit_wgpu_adapter) = self.as_any().downcast_mut::<WinitWgpuAdapter>() {
                winit_wgpu_adapter.draw_buffer_to_texture(buf, rtidx);
                return;
            }
        }



        // SDL adapter
        #[cfg(all(feature = "sdl", not(target_arch = "wasm32")))]
        {
            use crate::render::adapter::sdl::SdlAdapter;
            if let Some(sdl_adapter) = self.as_any().downcast_mut::<SdlAdapter>() {
                sdl_adapter.draw_buffer_to_texture(buf, rtidx);
                return;
            }
        }

        // Web adapter
        #[cfg(target_arch = "wasm32")]
        {
            use crate::render::adapter::web::WebAdapter;
            if let Some(web_adapter) = self.as_any().downcast_mut::<WebAdapter>() {
                web_adapter.draw_buffer_to_texture(buf, rtidx);
                return;
            }
        }

        // If we reach here, none of the specific adapter types matched
        eprintln!("Warning: draw_buffer_to_texture_dyn called on unknown adapter type");
    }

    // draw render buffer to render texture - unified for both OpenGL and WGPU
    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
    fn draw_render_buffer_to_texture(&mut self, rbuf: &[RenderCell], rtidx: usize, debug: bool) 
    where
        Self: Sized,
    {
        // All adapters now implement their own direct rendering methods
        // This default implementation should never be called since each adapter
        // overrides this method with their own specialized implementation:
        // - WinitGlowAdapter: Uses direct gl_pixel_renderer
        // - WinitWgpuAdapter: Uses direct wgpu_pixel_renderer  
        // - SdlAdapter: Uses direct gl_pixel_renderer
        // - WebAdapter: Uses direct gl_pixel_renderer
        eprintln!("Warning: draw_render_buffer_to_texture called on adapter that hasn't implemented direct rendering");
    }

    // buffer to render buffer - unified for both OpenGL and WGPU
    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
    fn buffer_to_render_buffer(&mut self, cb: &Buffer) -> Vec<RenderCell>
    where
        Self: Sized,
    {
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
    #[cfg(any(
        feature = "sdl",
        feature = "winit",
        feature = "wgpu",
        target_arch = "wasm32"
    ))]
    fn draw_render_textures_to_screen(&mut self)
    where
        Self: Sized,
    {
        // All modes now handled by their respective specialized adapters:
        // - WinitGlowAdapter: Direct OpenGL rendering
        // - WinitWgpuAdapter: Direct WGPU rendering  
        // - SdlAdapter: Direct OpenGL rendering
        // - WebAdapter: Direct OpenGL rendering
        // This unified approach is no longer needed
    }

    fn as_any(&mut self) -> &mut dyn Any;


}



// draw_render_textures_to_screen_unified_opengl function removed
// All adapters now implement their own specialized draw_render_textures_to_screen methods







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
