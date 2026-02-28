// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! # Render Adapter Module
//!
//! This module defines the render adapter architecture for RustPixel, providing unified
//! rendering interfaces across different platforms and rendering backends.
//!
//! ### Supported Rendering Backends
//!
//! #### TextMode
//! - **CrosstermAdapter**: Terminal text-mode rendering with crossterm
//!
//! #### GraphicsMode
//! - **WinitWgpuAdapter**: Modern GPU rendering (winit + wgpu) for desktop
//! - **WgpuWebAdapter**: WGPU-based browser rendering (WebGPU/WebGL2 fallback) for web
//!
//! ```text
//! src/render/adapter.rs         # This file - adapter definitions
//! src/render/adapter/
//! ├── cross_adapter.rs          # Terminal rendering (crossterm)
//! ├── wgpu_web_adapter.rs       # WGPU browser rendering (WebGPU + WebGL2 fallback)
//! ├── winit_common.rs           # Shared winit utilities
//! ├── winit_wgpu_adapter.rs     # Winit + WGPU modern rendering
//! └── wgpu/                     # WGPU backend implementation (shared)
//! ```
//!
//! ## 🔄 Unified Rendering Pipeline
//!
//! All rendering adapters now share a common rendering flow:
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
//! │ ┌─────────────┬─────────────┬─────────────┐                 │
//! │ │   Winit     │    Web      │ Crossterm   │                 │
//! │ │   WGPU      │  Adapter    │  Adapter    │                 │
//! │ │  Adapter    │             │             │                 │
//! │ └─────────────┴─────────────┴─────────────┘                 │
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
//! │          │  │    WebGL2 (Web/glow)       │ │                │
//! │          │  │  - Vertex Arrays           │ │                │
//! │          │  │  - Shader Programs         │ │                │
//! │          │  │  - Texture Atlases         │ │                │
//! │          │  └────────────────────────────┘ │                │
//! │          │                                 │                │
//! │          │  ┌────────────────────────────┐ │                │
//! │          │  │  WGPU (Desktop + WebGPU)   │ │                │
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
//! - **Mouse and keyboard** event translation with TUI height support
//! - **Accurate coordinate conversion** accounting for TUI double-height characters
//! - **Custom cursor** support
//!

#![allow(unused_variables)]
#[cfg(graphics_mode)]
use crate::render::graph::render_buffer_to_cells;
use crate::{
    event::Event,
    render::{buffer::Buffer, sprite::Layer},
    util::{Rand, Rect},
};

use std::any::Any;
use std::time::Duration;
// use log::info;

// Re-export RT types from graph module for backward compatibility
#[cfg(graphics_mode)]
pub use crate::render::graph::{BlendMode, RtComposite, RtConfig, RtSize};

/// WGPU rendering subsystem - modern GPU API for cross-platform rendering
/// Used by both desktop (via winit) and web (via wasm) adapters
#[cfg(any(wgpu_backend, wgpu_web_backend))]
pub mod wgpu;

/// WGPU Web adapter module - WGPU browser rendering with WebGPU/WebGL2 fallback
#[cfg(wgpu_web_backend)]
pub mod wgpu_web_adapter;

/// Winit common module - Shared code for winit_wgpu adapter
#[cfg(wgpu_backend)]
pub mod winit_common;

/// Winit + WGPU adapter module - Modern GPU backend with winit window management
#[cfg(wgpu_backend)]
pub mod winit_wgpu_adapter;

/// Crossterm adapter module - Terminal-based text mode rendering
#[cfg(cross_backend)]
pub mod cross_adapter;

// Re-export graph rendering functions and data structures
#[cfg(graphics_mode)]
pub use crate::render::graph::{
    generate_render_buffer,
    get_ratio_x,
    get_ratio_y,
    init_sym_height,
    init_sym_width,
    push_render_buffer,
    render_logo,
    Graph,
    RenderCell,
    PIXEL_LOGO_HEIGHT,
    PIXEL_LOGO_WIDTH,
    PIXEL_RATIO_X,
    PIXEL_RATIO_Y,
    PIXEL_SYM_HEIGHT,
    PIXEL_SYM_WIDTH,
    PIXEL_TEXTURE_FILE,
};

/// Adapter base data structure containing shared information and OpenGL resources
///
/// AdapterBase holds common data and GPU resources shared across all graphics
/// mode adapters (WinitWgpu, WgpuWeb). This design follows the principle of separation
/// of concerns while avoiding code duplication.
///
/// ## Architecture Role
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                  Adapter Interface                          │
/// │  ┌─────────────┬─────────────┬─────────────┐                │
/// │  │    Winit    │     Web     │  Crossterm  │                │
/// │  │   WGPU      │   Adapter   │   Adapter   │                │
/// │  │   Adapter   │      │      │      │      │                │
/// │  │  ┌───▼───┐  │  ┌───▼───┐  │      │      │                │
/// │  │  │ Base  │  │  │ Base  │  │     N/A     │                │
/// │  │  │ Data  │  │  │ Data  │  │ (Terminal)  │                │
/// │  │  └───────┘  │  └───────┘  │             │                │
/// │  └─────────────┴─────────────┴─────────────┘                │
/// └─────────────────────────────────────────────────────────────┘
/// ```
/// Base data structure shared by all rendering adapters
///
/// Note: `game_name` and `project_path` are now stored in the global `GAME_CONFIG`.
/// Use `rust_pixel::get_game_config()` to access them from anywhere.
pub struct AdapterBase {
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

impl Default for AdapterBase {
    fn default() -> Self {
        Self::new()
    }
}

impl AdapterBase {
    pub fn new() -> Self {
        Self {
            title: "".to_string(),
            cell_w: 0,
            cell_h: 0,
            rd: Rand::new(),
            #[cfg(graphics_mode)]
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
/// - **WinitWgpuAdapter**: Desktop rendering with WGPU (Vulkan/Metal/DX12)
/// - **WgpuWebAdapter**: Browser rendering with WGPU (WebGPU/WebGL2 fallback)
/// - **CrosstermAdapter**: Terminal text mode rendering
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
        layers: &mut Vec<Layer>,
        stage: u32,
    ) -> Result<(), String>;

    /// Post draw process
    fn post_draw(&mut self);

    /// Main rendering pipeline with double buffering and render textures
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
    /// │  │   texture(rbuf, 2)      │        │   base.rbuf for     │ │
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
    /// - **Render Texture 2**: Main game content (characters, sprites)
    /// - **Render Texture 3**: Transition effects and overlays
    /// - **Screen Buffer**: Final composite output (uses OS window decoration)
    ///
    /// ## Rendering Modes
    /// - **rflag=true**: Normal rendering directly to screen
    /// - **rflag=false**: Buffered mode - stores render data for external access (FFI/WASM)
    #[cfg(graphics_mode)]
    fn draw_all_graph(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        layers: &mut Vec<Layer>,
        stage: u32,
    ) {
        // Pass 1: Convert game data (buffer + layers) to GPU-ready format
        let rbuf = generate_render_buffer(
            current_buffer,
            previous_buffer,
            layers,
            stage,
            self.get_base(),
        );

        // Pass 2: Render to RT2 or buffer based on mode
        if self.get_base().gr.rflag {
            // Draw RenderCell array to render_texture 2 (main scene)
            // Note: present_default() is called separately by Scene::draw()
            // This allows apps to customize the present stage
            self.rbuf2rt(&rbuf, 2, false);
        } else {
            // Buffered mode: Store render data for external access
            // Used by FFI interfaces and WASM exports to access raw render data
            self.get_base().gr.rbuf = rbuf;
        }
    }

    // draw buffer to render texture - unified for both OpenGL and WGPU
    #[cfg(graphics_mode)]
    fn buf2rt(&mut self, buf: &Buffer, rtidx: usize) {
        let mut rbuf = vec![];
        // Use default transformation (no scale, no rotation, full opacity)
        self.buf2rbuf(buf, &mut rbuf, false, 255, 1.0, 1.0, 0.0);
        // Then draw render buffer to texture
        self.rbuf2rt(&rbuf, rtidx, false);
    }

    // ========================================================================
    // RENDERING PRIMITIVES
    // ========================================================================
    //
    // These 4 primitives are the foundation of the entire rendering pipeline.
    // All other rendering methods are combinations of these primitives.
    //
    // ┌─────────────────────────────────────────────────────────────────────┐
    // │  Primitive 1: buf2rbuf  - Buffer → RenderBuffer (with transforms)  │
    // │  Primitive 2: rbuf2rt   - RenderBuffer → RenderTexture             │
    // │  Primitive 3: blend_rts - RT₁ + RT₂ → RT₃ (shader blend)           │
    // │  Primitive 4: present   - RT → Screen                              │
    // └─────────────────────────────────────────────────────────────────────┘
    //

    /// PRIMITIVE 1: buf2rbuf - Buffer → RenderBuffer with full transformation
    ///
    /// The most fundamental rendering primitive. Converts a Buffer's content
    /// to RenderCell format and appends it to the render buffer.
    /// Supports full transformation: alpha, scale, and rotation.
    ///
    /// # Parameters
    /// - `buffer`: Source buffer (read-only, contains position in area.x/y)
    /// - `rbuf`: Target render buffer to append to (mutable)
    /// - `use_tui`: Use TUI characters (16×32) if true, Sprite (8×8) if false
    /// - `alpha`: Overall transparency (0=transparent, 255=opaque)
    /// - `scale_x`, `scale_y`: Overall scale factors (1.0 = no scaling)
    /// - `angle`: Overall rotation angle in degrees (0.0 = no rotation)
    #[cfg(graphics_mode)]
    fn buf2rbuf(
        &mut self,
        buffer: &Buffer,
        rbuf: &mut Vec<RenderCell>,
        use_tui: bool,
        alpha: u8,
        scale_x: f32,
        scale_y: f32,
        angle: f64,
    ) {
        let rx = self.get_base().gr.ratio_x;
        let ry = self.get_base().gr.ratio_y;

        render_buffer_to_cells(
            buffer,
            rx,
            ry,
            use_tui,
            alpha,
            scale_x,
            scale_y,
            angle,
            |fc, bc, s2, texidx, symidx, angle, ccp, modifier| {
                push_render_buffer(rbuf, fc, bc, texidx, symidx, s2, angle, &ccp, modifier);
            },
        );
    }

    /// PRIMITIVE 2: rbuf2rt - RenderBuffer → RenderTexture
    ///
    /// Second stage: Takes GPU-ready RenderCell array and renders it to
    /// a specified render texture.
    ///
    /// # Parameters
    /// - `rbuf`: Array of RenderCell data (from buf2rbuf)
    /// - `rt`: Target render texture index (0-3)
    /// - `debug`: Enable debug mode (colored backgrounds for debugging)
    #[cfg(graphics_mode)]
    fn rbuf2rt(&mut self, rbuf: &[RenderCell], rtidx: usize, debug: bool);

    // Primitives 3 (blend_rts) and 4 (present) are defined above in the RT section

    #[cfg(graphics_mode)]
    fn only_render_buffer(&mut self) {
        self.get_base().gr.rflag = false;
    }

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
    fn set_rt_visible(&mut self, texture_index: usize, visible: bool);

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
    fn setup_buffer_transition(&mut self, target_texture: usize);

    /// Copy one render texture to another
    ///
    /// Efficiently copies the contents of one render texture to another without
    /// going through the full shader pipeline. This is much faster than using
    /// a transition shader with progress=1.0 for static display purposes.
    ///
    /// # Parameters
    /// - `src_index`: Source render texture index (0-3)
    /// - `dst_index`: Destination render texture index (0-3)
    ///
    /// # Use Cases
    /// - Displaying static transition results without shader overhead
    /// - Preparing render textures for subsequent operations
    /// - Swapping/copying render texture contents
    #[cfg(graphics_mode)]
    fn copy_rt(&mut self, src_index: usize, dst_index: usize);

    // ========================================================================
    // New RT API - Unified RenderTexture management
    // ========================================================================

    /// Configure a render texture
    ///
    /// Sets up RT with specified configuration (size strategy, etc.)
    /// Call this during initialization for custom RT configurations.
    ///
    /// # Parameters
    /// - `rt`: RT index (0-3)
    /// - `config`: RT configuration
    #[cfg(graphics_mode)]
    fn configure_rt(&mut self, rt: usize, config: RtConfig) {
        // Default implementation - store config for later use
        // Graphics adapters can override with optimized implementations
    }

    /// Resize a render texture to specific dimensions
    ///
    /// Manually resize an RT. Only effective for RTs configured with Fixed size.
    ///
    /// # Parameters
    /// - `rt`: RT index (0-3)
    /// - `width`: New width in pixels
    /// - `height`: New height in pixels
    #[cfg(graphics_mode)]
    fn resize_rt(&mut self, rt: usize, width: u32, height: u32) {
        // Default implementation - no-op
        // Graphics adapters should override with actual resize logic
    }

    /// Clear a render texture
    ///
    /// Clears the specified RT to transparent black.
    ///
    /// # Parameters
    /// - `rt`: RT index (0-3)
    #[cfg(graphics_mode)]
    fn clear_rt(&mut self, rt: usize) {
        // Default implementation - no-op
        // Graphics adapters should override
    }

    /// Blend two RTs with effect and render to target RT
    ///
    /// GPU shader-based transition effect.
    ///
    /// # Parameters
    /// - `src1`: Source RT 1 index
    /// - `src2`: Source RT 2 index
    /// - `target`: Target RT index
    /// - `effect`: Effect type (0=Mosaic, 1=Heart, etc.)
    /// - `progress`: Transition progress (0.0-1.0)
    #[cfg(graphics_mode)]
    fn blend_rts(&mut self, src1: usize, src2: usize, target: usize, effect: usize, progress: f32);

    /// Present RT composite chain to screen
    ///
    /// This is the new unified method to composite RTs to screen.
    /// Replaces the old draw_render_textures_to_screen() with flexible RT chain.
    ///
    /// # Parameters
    /// - `composites`: Array of RtComposite items to render in order
    ///
    /// # Example
    /// ```ignore
    /// // Simple: just render RT2
    /// adapter.present(&[RtComposite::fullscreen(2)]);
    ///
    /// // Complex: RT3 first, then RT2 overlay
    /// adapter.present(&[
    ///     RtComposite::fullscreen(3),
    ///     RtComposite::fullscreen(2).alpha(200),
    /// ]);
    /// ```
    #[cfg(graphics_mode)]
    fn present(&mut self, composites: &[RtComposite]);

    /// Present with default settings (RT2 fullscreen)
    ///
    /// Convenience method for simple cases - just renders RT2 to screen.
    /// This maintains backward compatibility with Scene.draw().
    #[cfg(graphics_mode)]
    fn present_default(&mut self) {
        self.present(&[RtComposite::fullscreen(2)]);
    }

    /// Set CAS (Contrast Adaptive Sharpening) intensity
    ///
    /// Applied during the final RT-to-screen composition pass (Stage 4).
    /// Useful for improving text clarity on high-DPI displays when
    /// the texture atlas resolution causes slight edge blur.
    ///
    /// # Parameters
    /// - `sharpness`: 0.0 = off, 0.5 = moderate, 1.0 = maximum
    #[cfg(graphics_mode)]
    fn set_sharpness(&mut self, _sharpness: f32) {
        // Default no-op; WGPU adapters override this
    }

    /// Set whether MSDF/SDF rendering is enabled for TUI/CJK regions.
    ///
    /// When enabled, TUI and CJK symbols use MSDF distance field rendering
    /// for crisp edges at any scale. When disabled, all symbols use bitmap
    /// rendering (for legacy 4096 textures).
    ///
    /// By default, this is auto-detected from texture size:
    /// 8192+ = MSDF enabled, 4096 = bitmap only.
    ///
    /// # Parameters
    /// - `enabled`: true to enable MSDF rendering, false for bitmap-only
    #[cfg(graphics_mode)]
    fn set_msdf_enabled(&mut self, _enabled: bool) {
        // Default no-op; WGPU adapters override this
    }

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
}
