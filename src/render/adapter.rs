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

#[cfg(any(feature = "sdl", feature = "wgpu", feature = "winit", target_arch = "wasm32"))]
use crate::render::pixel_renderer::{PixelRenderer, RenderContext, UnifiedColor, UnifiedTransform};

// Import GlPixelRenderer only when OpenGL backends are available
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
use crate::render::adapter::gl::pixel::GlPixelRenderer;


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

/// Legacy Winit adapter module - Mixed OpenGL/WGPU implementation (deprecated)
#[cfg(all(any(feature = "winit", feature = "wgpu"), not(target_arch = "wasm32")))]
pub mod winit;

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

        // Legacy Winit adapter (for backward compatibility)
        #[cfg(all(any(feature = "winit", feature = "wgpu"), not(target_arch = "wasm32")))]
        {
            use crate::render::adapter::winit::WinitAdapter;
            if let Some(winit_adapter) = self.as_any().downcast_mut::<WinitAdapter>() {
                winit_adapter.draw_buffer_to_texture(buf, rtidx);
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
        // Try the unified approach first - works for OpenGL cases
        // Get ratios first to avoid borrowing conflicts
        let ratio_x;
        let ratio_y;
        {
            let base = self.get_base();
            ratio_x = base.gr.ratio_x;
            ratio_y = base.gr.ratio_y;
        }
        
        // Use unified PixelRenderer interface - no more downcast needed!
        if let Some(pixel_renderer) = self.get_base().gr.pixel_renderer.as_mut() {
            // Set clear color for debug mode
            let clear_color = if debug {
                UnifiedColor::new(1.0, 0.0, 0.0, 1.0) // Red for debug
            } else {
                UnifiedColor::black() // Black for normal
            };
            pixel_renderer.set_clear_color(&clear_color);
            
            // Call unified rendering method - works for both OpenGL and WGPU
            if let Err(e) = pixel_renderer.render_symbols_to_texture(rbuf, rtidx, ratio_x, ratio_y) {
                eprintln!("PixelRenderer render_symbols_to_texture error: {}", e);
            }
            return;
        }
        
        // Fallback to old method for WGPU complex cases that need special CommandEncoder handling
        #[cfg(feature = "wgpu")]
        {
            use crate::render::adapter::winit::WinitAdapter;

            if let Some(winit_adapter) = self.as_any().downcast_mut::<WinitAdapter>() {
                // Check if WGPU components are available
                if winit_adapter.wgpu_pixel_renderer.is_some() {
                    // WGPU mode - use specialized implementation for CommandEncoder management
                    if let Err(e) = draw_render_buffer_to_texture_unified_wgpu(winit_adapter, rbuf, rtidx, debug) {
                        eprintln!("WGPU draw_render_buffer_to_texture error: {}", e);
                    }
                    return;
                }
            }
        }

        // No OpenGL fallback needed - unified interface handles all cases above
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
        // First check if we're in WGPU mode and handle it accordingly
        #[cfg(feature = "wgpu")]
        {
            use crate::render::adapter::winit::WinitAdapter;

            if let Some(winit_adapter) = self.as_any().downcast_mut::<WinitAdapter>() {
                // Check if WGPU components are available
                if winit_adapter.wgpu_pixel_renderer.is_some() {
                    // WGPU mode - use unified implementation
                    if let Err(e) = draw_render_textures_to_screen_unified_wgpu(winit_adapter) {
                        eprintln!("WGPU draw_render_textures_to_screen error: {}", e);
                    }
                    return;
                }
            }
        }

        // OpenGL mode implementation using unified PixelRenderer interface
        #[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
        {
            draw_render_textures_to_screen_unified_opengl(self);
        }
    }

    fn as_any(&mut self) -> &mut dyn Any;


}

/// Unified WGPU implementation for render texture to screen composition
#[cfg(feature = "wgpu")]
fn draw_render_textures_to_screen_unified_wgpu(
    winit_adapter: &mut crate::render::adapter::winit::WinitAdapter,
) -> Result<(), String> {
    use ::wgpu::{
        Color, CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment,
        RenderPassDescriptor, StoreOp, TextureViewDescriptor,
    };

    let (device, queue, surface, pixel_renderer) = {
        let device = winit_adapter
            .wgpu_device
            .as_ref()
            .ok_or("WGPU device not initialized")?;
        let queue = winit_adapter
            .wgpu_queue
            .as_ref()
            .ok_or("WGPU queue not initialized")?;
        let surface = winit_adapter
            .wgpu_surface
            .as_ref()
            .ok_or("WGPU surface not initialized")?;
        let pixel_renderer = winit_adapter
            .wgpu_pixel_renderer
            .as_mut()
            .ok_or("WGPU pixel renderer not initialized")?;
        (device, queue, surface, pixel_renderer)
    };

    // Get current surface texture
    let output = surface
        .get_current_texture()
        .map_err(|e| format!("Failed to acquire next swap chain texture: {}", e))?;

    let view = output
        .texture
        .create_view(&TextureViewDescriptor::default());

    // Bind screen as render target
    pixel_renderer.bind_screen();

    // Create command encoder for screen composition
    let mut screen_encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Screen Composition Encoder"),
    });

    // Clear screen
    {
        let _clear_pass = screen_encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Clear Screen Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::BLACK),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
    }

    // Call WGPU-specific rendering logic
    draw_render_textures_unified_wgpu(
        pixel_renderer,
        device,
        queue,
        &mut screen_encoder,
        &view,
        winit_adapter.base.gr.ratio_x,
        winit_adapter.base.gr.ratio_y,
    )?;

    // Submit commands and present frame
    queue.submit(std::iter::once(screen_encoder.finish()));
    output.present();

    Ok(())
}

/// Unified OpenGL implementation for render texture to screen composition
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
fn draw_render_textures_to_screen_unified_opengl(adapter: &mut dyn Adapter) {
    let ratio_x;
    let ratio_y;

    // Get all needed data first to avoid borrowing conflicts
    {
        let bs = adapter.get_base();
        ratio_x = bs.gr.ratio_x;
        ratio_y = bs.gr.ratio_y;
    }

    // Get physical size for retina displays if needed (only when winit/wgpu is available)
    #[cfg(all(any(feature = "winit", feature = "wgpu"), not(target_arch = "wasm32")))]
    let physical_size = {
        use crate::render::adapter::winit::WinitAdapter;
        if let Some(winit_adapter) = adapter.as_any().downcast_mut::<WinitAdapter>()
        {
            winit_adapter.window.as_ref().map(|w| w.inner_size())
        } else {
            None
        }
    };

    let bs = adapter.get_base();
    if let Some(pixel_renderer) = &mut bs.gr.pixel_renderer {
        // Handle Retina displays for Winit adapter
        #[cfg(all(any(feature = "winit", feature = "wgpu"), not(target_arch = "wasm32")))]
        {
            if let Some(size) = physical_size {
                // For GlPixelRenderer, we need to access the OpenGL context directly
                if let Some(gl_pixel_renderer) = pixel_renderer.as_any().downcast_mut::<GlPixelRenderer>() {
                    unsafe {
                        use glow::HasContext;
                        let gl = gl_pixel_renderer.get_gl();
                        gl.viewport(0, 0, size.width as i32, size.height as i32);
                    }
                }
            }
        }

        // Bind to screen framebuffer
        pixel_renderer.bind_render_target(None);

        // Layer 1: Draw render_texture 2 (main game content)
        if !pixel_renderer.get_render_texture_hidden(2) {
            let transform = UnifiedTransform::new();
            let color = UnifiedColor::white();
            if let Err(e) = pixel_renderer.render_texture_to_screen(2, [0.0, 0.0, 1.0, 1.0], &transform, &color) {
                eprintln!("Error rendering texture 2 to screen: {}", e);
            }
        }

        // Layer 2: Draw render_texture 3 (transition effects and overlays)
        if !pixel_renderer.get_render_texture_hidden(3) {
            // Calculate proper scaling for high-DPI displays
            let (canvas_width, canvas_height) = pixel_renderer.get_canvas_size();
            let pcw = canvas_width as f32;
            let pch = canvas_height as f32;

            // Calculate scaled dimensions for transition layer
            let pw = 40.0f32 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / ratio_x;
            let ph = 25.0f32 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ratio_y;

            // Create unified transform with proper scaling
            let mut transform = UnifiedTransform::new();
            transform.scale(pw / pcw, ph / pch);

            // Calculate viewport based on graphics API (OpenGL bottom-left origin)
            let viewport = [0.0 / pcw, (pch - ph) / pch, pw / pcw, ph / pch];

            let color = UnifiedColor::white();
            if let Err(e) = pixel_renderer.render_texture_to_screen(3, viewport, &transform, &color) {
                eprintln!("Error rendering texture 3 to screen: {}", e);
            }
        }
    }
}

/// WGPU-specific rendering logic for screen composition
///
/// This function handles WGPU screen composition using direct calls to WgpuPixelRender
/// methods since WGPU requires external CommandEncoder management.
#[cfg(feature = "wgpu")]
fn draw_render_textures_unified_wgpu(
    wgpu_pixel_renderer: &mut crate::render::adapter::wgpu::pixel::WgpuPixelRender,
    device: &::wgpu::Device,
    queue: &::wgpu::Queue,
    encoder: &mut ::wgpu::CommandEncoder,
    view: &::wgpu::TextureView,
    ratio_x: f32,
    ratio_y: f32,
) -> Result<(), String> {
    let unified_color = UnifiedColor::white();

    // Layer 1: Draw render_texture 2 (main game content)
    if !wgpu_pixel_renderer.get_render_texture_hidden(2) {
        let unified_transform = UnifiedTransform::new();
        wgpu_pixel_renderer.render_texture_to_screen_impl(
            device,
            queue,
            encoder,
            view,
            2,
            [0.0, 0.0, 1.0, 1.0], // Full-screen quad
            &unified_transform,
            &unified_color,
        )?;
    }

    // Layer 2: Draw render_texture 3 (transition effects and overlays)
    if !wgpu_pixel_renderer.get_render_texture_hidden(3) {
        // Calculate proper scaling for high-DPI displays
        let (canvas_width, canvas_height) = wgpu_pixel_renderer.get_canvas_size();
        let pcw = canvas_width as f32;
        let pch = canvas_height as f32;

        // Calculate scaled dimensions for transition layer
        let pw = 40.0f32 * PIXEL_SYM_WIDTH.get().expect("lazylock init") / ratio_x;
        let ph = 25.0f32 * PIXEL_SYM_HEIGHT.get().expect("lazylock init") / ratio_y;

        // Create unified transform with proper scaling
        let mut unified_transform = UnifiedTransform::new();
        unified_transform.scale(pw / pcw, ph / pch);

        // WGPU Y-axis: top-left origin
        let viewport = [0.0 / pcw, 0.0 / pch, pw / pcw, ph / pch];

        wgpu_pixel_renderer.render_texture_to_screen_impl(
            device,
            queue,
            encoder,
            view,
            3,
            viewport,
            &unified_transform,
            &unified_color,
        )?;
    }

    Ok(())
}

/// Unified WGPU implementation for render buffer to texture composition
#[cfg(feature = "wgpu")]
fn draw_render_buffer_to_texture_unified_wgpu(
    winit_adapter: &mut crate::render::adapter::winit::WinitAdapter,
    rbuf: &[RenderCell],
    rtidx: usize,
    debug: bool,
) -> Result<(), String> {
    use ::wgpu::CommandEncoderDescriptor;

    let (device, queue, pixel_renderer) = {
        let device = winit_adapter
            .wgpu_device
            .as_ref()
            .ok_or("WGPU device not initialized")?;
        let queue = winit_adapter
            .wgpu_queue
            .as_ref()
            .ok_or("WGPU queue not initialized")?;
        let pixel_renderer = winit_adapter
            .wgpu_pixel_renderer
            .as_mut()
            .ok_or("WGPU pixel renderer not initialized")?;
        (device, queue, pixel_renderer)
    };

    // Create command encoder for render to texture
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some(&format!("Render Buffer to RT{} Encoder", rtidx)),
    });

    // Get ratios for rendering
    let ratio_x = winit_adapter.base.gr.ratio_x;
    let ratio_y = winit_adapter.base.gr.ratio_y;

    // Use WGPU-specific methods directly instead of unified interface
    // Set clear color for debug mode
    let clear_color = if debug {
        UnifiedColor::new(1.0, 0.0, 0.0, 1.0) // Red for debug
    } else {
        UnifiedColor::black() // Black for normal
    };
    pixel_renderer.set_clear_color(clear_color);

    // Bind target render texture
    pixel_renderer.bind_target(rtidx);

    // Render RenderCell data to current target using WGPU-specific method
    pixel_renderer.render_rbuf(device, queue, rbuf, ratio_x, ratio_y);

    // Execute rendering to current target (render texture)
    pixel_renderer.render_to_current_target(&mut encoder, None)?;

    // Submit commands
    queue.submit(std::iter::once(encoder.finish()));

    Ok(())
}

// Removed draw_render_buffer_to_texture_unified_opengl - unified interface handles both OpenGL and WGPU

/// Unified rendering logic for render buffer to texture - both WGPU and OpenGL modes
///
/// This method contains the core rendering logic that is shared between
/// WGPU and OpenGL implementations. It renders RenderCell data to a render texture
/// using the unified PixelRenderer interface with simplified parameters.
#[cfg(any(
    feature = "sdl",
    feature = "winit",
    feature = "wgpu",
    target_arch = "wasm32"
))]
fn draw_render_buffer_to_texture_unified(
    pixel_renderer: &mut dyn PixelRenderer,
    rbuf: &[RenderCell],
    rtidx: usize,
    debug: bool,
    ratio_x: f32,
    ratio_y: f32,
) -> Result<(), String> {
    // Set clear color using unified interface
    let clear_color = if debug {
        UnifiedColor::new(1.0, 0.0, 0.0, 1.0) // Red for debug
    } else {
        UnifiedColor::black() // Black for normal
    };
    pixel_renderer.set_clear_color(&clear_color);

    // Render symbols to texture using simplified unified interface
    // Each implementation manages its own rendering context internally
    pixel_renderer.render_symbols_to_texture(rbuf, rtidx, ratio_x, ratio_y)?;

    Ok(())
}

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
