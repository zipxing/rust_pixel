// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! **RustPixel** is a lightweight, glyph-based 2D game engine and rapid-prototyping toolkit.
//!
//! It provides a unified abstraction for both **terminal** and **graphic** rendering modes
//! (including WebAssembly), allowing developers to build pixel-style games, tools, and
//! simulation prototypes with minimal boilerplate.
//!
//! RustPixel supports **TUI both with and without a real terminal environment**, thanks to its
//! built-in glyph atlas and software-rendered text engine. This enables rich UI overlays on
//! top of game layers even in pure graphic mode.
//!
//! Designed for clarity, portability, and fast iteration, RustPixel is ideal for:
//! - Glyph / PETSCII / ASCII / emoji-based pixel games
//! - Terminal applications & hybrid TUI-over-graphics UIs
//! - Rapid prototyping of gameplay ideas
//! - Cross-platform rendering (Desktop, Web, Mobile, Mini-Game platforms)
//!
//! RustPixel's architecture emphasizes simplicity and composability, making it a practical
//! foundation for building both experimental and production-ready pixel-driven experiences.
//!
//! Rendering modes:
//! - Text mode: Runs in a terminal via `crossterm`, drawing with ASCII and Unicode/Emoji.
//! - Graphics mode (native): Uses `wgpu` or `SDL2`, rendering PETSCII and custom symbol sets.
//! - Graphics mode (web): Same core logic compiled to WASM, rendered via WebGL + JavaScript
//!
//! Core concepts:
//! - Cell: Smallest renderable unit (a character in text mode, or a fixed‑size glyph in graphics mode).
//! - Buffer: A collection of cells representing the screen state, with diff‑friendly updates.
//! - Panel/Sprite/Style: Higher‑level drawing abstractions that work uniformly across backends.
//!
//! Modules overview:
//! - `algorithm`, `event`, `util`: Always available; form the minimal runtime.
//! - `asset`, `audio`, `context`, `game`, `log`, `render`, `ui`: Enabled when not in `base` mode.
//!
//! Minimal build (base mode): Only `algorithm`, `event`, and `util` are compiled, reducing
//! dependencies for shipping as FFI or WASM libraries. This is ideal when you only need the
//! engine’s core data structures and event system.

use std::sync::OnceLock;

/// Global game configuration - initialized once at startup
///
/// This provides a single source of truth for game name and project path,
/// accessible from anywhere in the codebase without passing references.
#[derive(Debug, Clone)]
pub struct GameConfig {
    /// Game name identifier
    pub game_name: String,
    /// Project root path for asset loading
    pub project_path: String,
}

/// Global static game configuration
pub static GAME_CONFIG: OnceLock<GameConfig> = OnceLock::new();

// ============================================================================
// Unified Texture Asset Loading
// ============================================================================

/// Cached texture data loaded from symbols.png
///
/// This struct holds the raw pixel data after loading from disk but before
/// uploading to GPU. This allows early loading during init_pixel_assets()
/// while deferring GPU upload to adapter.init().
#[derive(Debug, Clone)]
pub struct PixelTextureData {
    /// Texture width in pixels
    pub width: u32,
    /// Texture height in pixels
    pub height: u32,
    /// Raw RGBA pixel data
    pub data: Vec<u8>,
}

/// Global cached texture data - loaded once during init_pixel_assets()
pub static PIXEL_TEXTURE_DATA: OnceLock<PixelTextureData> = OnceLock::new();

/// Get the cached texture data
///
/// # Panics
/// Panics if init_pixel_assets() was not called before this function.
pub fn get_pixel_texture_data() -> &'static PixelTextureData {
    PIXEL_TEXTURE_DATA.get().expect(
        "Texture data not loaded - call init_pixel_assets() first"
    )
}

/// Initialize the global game configuration
/// This should be called once at program startup before any other initialization.
pub fn init_game_config(game_name: &str, project_path: &str) {
    let _ = GAME_CONFIG.set(GameConfig {
        game_name: game_name.to_string(),
        project_path: project_path.to_string(),
    });
}

/// Get a reference to the global game configuration
/// If not initialized, returns a default config with empty game_name and "." as project_path.
/// This allows for gradual migration and testing scenarios.
pub fn get_game_config() -> &'static GameConfig {
    GAME_CONFIG.get_or_init(|| {
        // Default configuration for testing and backward compatibility
        GameConfig {
            game_name: String::new(),
            project_path: ".".to_string(),
        }
    })
}

/// Target frames per second for the main game loop. Keep this moderate to conserve CPU.
pub const GAME_FRAME: u32 = 60;

// ============================================================================
// Unified Asset Initialization Functions
// ============================================================================

/// Initialize all pixel assets: game config + texture + symbol_map
///
/// This function should be called once at program startup, BEFORE creating
/// Model/Render instances. It performs the following steps:
/// 1. Set global game configuration (game_name, project_path)
/// 2. Load symbols.png into memory and set PIXEL_SYM_WIDTH/HEIGHT
/// 3. Load and parse symbol_map.json
///
/// After calling this function, all resources are ready for use:
/// - `get_game_config()` returns the game configuration
/// - `get_pixel_texture_data()` returns the texture data
/// - `get_symbol_map()` returns the symbol mapping
///
/// # Arguments
/// * `game_name` - Game identifier
/// * `project_path` - Project root path for asset loading
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(String)` with error message on failure
///
/// # Example
/// ```ignore
/// init_pixel_assets("my_game", "/path/to/project")?;
/// // Now safe to create Model and Render
/// let model = MyModel::new();
/// let render = MyRender::new();
/// ```
#[cfg(all(graphics_mode, not(target_arch = "wasm32")))]
pub fn init_pixel_assets(game_name: &str, project_path: &str) -> Result<(), String> {
    use render::adapter::{init_sym_height, init_sym_width, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, PIXEL_TEXTURE_FILE};

    // 1. Set global game configuration
    init_game_config(game_name, project_path);

    // 2. Load texture file into memory
    let texture_path = format!(
        "{}{}{}",
        project_path,
        std::path::MAIN_SEPARATOR,
        PIXEL_TEXTURE_FILE
    );

    let img = image::open(&texture_path)
        .map_err(|e| format!("Failed to load texture '{}': {}", texture_path, e))?
        .to_rgba8();

    let width = img.width();
    let height = img.height();

    // 3. Set PIXEL_SYM_WIDTH/HEIGHT
    PIXEL_SYM_WIDTH
        .set(init_sym_width(width))
        .map_err(|_| "PIXEL_SYM_WIDTH already initialized".to_string())?;
    PIXEL_SYM_HEIGHT
        .set(init_sym_height(height))
        .map_err(|_| "PIXEL_SYM_HEIGHT already initialized".to_string())?;

    // 4. Cache texture data for later GPU upload
    PIXEL_TEXTURE_DATA
        .set(PixelTextureData {
            width,
            height,
            data: img.into_raw(),
        })
        .map_err(|_| "PIXEL_TEXTURE_DATA already initialized".to_string())?;

    // 5. Load symbol_map.json
    render::symbol_map::init_symbol_map_from_file()?;

    println!(
        "Pixel assets initialized: {}x{} texture, symbol_map loaded from {}",
        width, height, project_path
    );

    Ok(())
}

/// Initialize pixel assets for Web/WASM mode
///
/// This function is called from JavaScript after loading the texture image
/// and symbol_map.json. It performs the same initialization as `init_pixel_assets`
/// but receives data directly from JavaScript instead of loading from files.
///
/// # Arguments
/// * `game_name` - Game identifier
/// * `tex_w` - Texture width in pixels
/// * `tex_h` - Texture height in pixels
/// * `tex_data` - Raw RGBA pixel data from JavaScript
/// * `symbol_map_json` - Content of symbol_map.json
///
/// # Returns
/// * `true` on success
/// * `false` on failure (error logged to console)
///
/// # JavaScript Example
/// ```js
/// import init, {PixelGame, wasm_init_pixel_assets} from "./pkg/pixel.js";
/// await init();
///
/// // Load texture
/// const timg = new Image();
/// timg.src = "assets/pix/symbols.png";
/// await timg.decode();
/// const imgdata = ctx.getImageData(0, 0, timg.width, timg.height).data;
///
/// // Load symbol map
/// const symbolMapJson = await fetch("assets/pix/symbol_map.json").then(r => r.text());
///
/// // Initialize all assets at once
/// wasm_init_pixel_assets("my_game", timg.width, timg.height, imgdata, symbolMapJson);
///
/// // Now create the game
/// const sg = PixelGame.new();
/// ```
/// Internal implementation - called by the wrapper generated in pixel_game! macro
/// The macro generates a #[wasm_bindgen] wrapper that exports this function to JavaScript
#[cfg(target_arch = "wasm32")]
pub fn wasm_init_pixel_assets(
    game_name: &str,
    tex_w: u32,
    tex_h: u32,
    tex_data: &[u8],
    symbol_map_json: &str,
) -> bool {
    use render::adapter::{init_sym_height, init_sym_width, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH};

    // 1. Set game configuration (use "." as project_path for web mode)
    init_game_config(game_name, ".");

    // 2. Set PIXEL_SYM_WIDTH/HEIGHT
    if PIXEL_SYM_WIDTH.set(init_sym_width(tex_w)).is_err() {
        web_sys::console::warn_1(&"PIXEL_SYM_WIDTH already initialized".into());
    }
    if PIXEL_SYM_HEIGHT.set(init_sym_height(tex_h)).is_err() {
        web_sys::console::warn_1(&"PIXEL_SYM_HEIGHT already initialized".into());
    }

    // 3. Cache texture data
    if PIXEL_TEXTURE_DATA
        .set(PixelTextureData {
            width: tex_w,
            height: tex_h,
            data: tex_data.to_vec(),
        })
        .is_err()
    {
        web_sys::console::warn_1(&"PIXEL_TEXTURE_DATA already initialized".into());
    }

    // 4. Initialize symbol map
    match render::symbol_map::init_symbol_map_from_json(symbol_map_json) {
        Ok(()) => {
            web_sys::console::log_1(
                &format!(
                    "RUST: Pixel assets initialized: {}x{} texture, symbol_map loaded",
                    tex_w, tex_h
                )
                .into(),
            );
            true
        }
        Err(e) => {
            web_sys::console::error_1(&format!("RUST: Failed to init symbol map: {}", e).into());
            false
        }
    }
}

/// DEPRECATED: Use wasm_init_pixel_assets instead
///
/// Initialize the global symbol map from JSON string (standalone WASM function)
/// This must be called BEFORE creating a PixelGame instance in web mode.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn wasm_init_symbol_map(json: &str) -> bool {
    match render::symbol_map::init_symbol_map_from_json(json) {
        Ok(()) => {
            web_sys::console::log_1(&"RUST: Symbol map initialized from JSON".into());
            true
        }
        Err(e) => {
            web_sys::console::error_1(&format!("RUST: Failed to init symbol map: {}", e).into());
            false
        }
    }
}

#[cfg(not(graphics_mode))]
pub const LOGO_FRAME: u32 = GAME_FRAME / 4 * 2;

#[cfg(graphics_mode)]
pub const LOGO_FRAME: u32 = GAME_FRAME / 4 * 5;

/// Re‑export the `paste` crate so downstream crates can use it in macros generated by this crate.
#[cfg(not(feature = "base"))]
pub use paste;

/// Macro `pixel_game` to scaffold a RustPixel application entry.
///
/// ## Architecture
///
/// This macro implements the conventional `lib.rs` + `main.rs` split used by RustPixel apps.
/// Although the split may look redundant, it is crucial for cross‑platform deployment and a
/// consistent framework architecture.
///
/// ### Why split `lib.rs` and `main.rs`?
///
/// - WASM builds require `crate-type = ["cdylib", "rlib"]` and do not use `main()`; instead,
///   exported functions are called from JavaScript. The generated `{Name}Game` methods such as
///   `new()`, `tick()`, and `key_event()` can be exported for the Web frontend.
/// - Multi‑platform deployment:
///   - Native binary: `main.rs` calls `{crate_name}::run()` from `lib.rs`.
///   - WASM library: JavaScript calls `{Name}Game::new()`, `tick()`, etc., from `lib.rs`.
///   - Library dependency: Other crates can depend on the library API in `lib.rs`.
/// - Unified architecture: All RustPixel games follow the same Model + Render + Game pattern,
///   enabling consistent interfaces across terminal, SDL2, and Web backends.
/// - Code organization: `lib.rs` defines the public API and integration points; `main.rs`
///   provides the native entry point. This improves testability and conditional compilation.
///
/// ### What does the macro generate?
///
/// ```rust
/// // Module structure
/// mod model;           // Game logic and state
/// mod render_terminal; // Terminal-mode rendering (text-based)
/// mod render_graphics; // Graphics-mode rendering (SDL/Web)
///
/// // Generated structs and functions
/// pub struct {Name}Game {
///     g: Game<{Name}Model, {Name}Render>,
/// }
///
/// pub fn init_game() -> {Name}Game { /* ... */ }
/// pub fn run() { /* ... */ } // Called by main.rs
///
/// // WASM-specific exports
/// impl {Name}Game {
///     pub fn new() -> Self { /* ... */ }          // WASM constructor
///     pub fn tick(&mut self, dt: f32) { /* ... */ } // WASM game loop
///     pub fn key_event(&mut self, /* ... */) { /* ... */ } // WASM input
/// }
/// ```
///
/// With this single macro, your game can run as:
/// - Terminal app (crossterm backend)
/// - Desktop app (opengl or wgpu backend)
/// - Web app (WebGL via WASM)
/// - A library embedded in other Rust projects
#[cfg(not(feature = "base"))]
#[macro_export]
macro_rules! pixel_game {
    ($name:ident) => {
        mod model;
        #[cfg(not(graphics_mode))]
        mod render_terminal;
        #[cfg(graphics_mode)]
        mod render_graphics;

        #[cfg(not(graphics_mode))]
        use crate::{model::*, render_terminal::*};
        #[cfg(graphics_mode)]
        use crate::{model::*, render_graphics::*};
        use rust_pixel::game::Game;
        use rust_pixel::util::get_project_path;

        #[cfg(wasm)]
        use rust_pixel::render::adapter::web_adapter::{input_events_from_web, WebAdapter};
        use wasm_bindgen::prelude::*;
        #[cfg(wasm)]
        use wasm_bindgen_futures::js_sys;
        #[cfg(wasm)]
        use log::info;

        rust_pixel::paste::paste! {
            // Re-export wasm_init_pixel_assets with wasm_bindgen attribute
            // This wrapper is needed because wasm-bindgen only exports from the current crate
            #[cfg(target_arch = "wasm32")]
            #[wasm_bindgen]
            pub fn wasm_init_pixel_assets(
                game_name: &str,
                tex_w: u32,
                tex_h: u32,
                tex_data: &[u8],
                symbol_map_json: &str,
            ) -> bool {
                rust_pixel::wasm_init_pixel_assets(game_name, tex_w, tex_h, tex_data, symbol_map_json)
            }
            #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
            pub struct [<$name Game>] {
                g: Game<[<$name Model>], [<$name Render>]>,
            }

            pub fn init_game() -> [<$name Game>] {
                let pp = get_project_path();
                println!("asset path : {:?}", pp);

                // Initialize assets based on mode:
                // - Graphics mode (native): load texture + symbol_map via init_pixel_assets
                // - Terminal mode: only set game config (no texture needed)
                // - WASM mode: JS already called wasm_init_pixel_assets before this
                #[cfg(all(graphics_mode, not(target_arch = "wasm32")))]
                {
                    rust_pixel::init_pixel_assets(stringify!([<$name:lower>]), &pp)
                        .expect("Failed to initialize pixel assets");
                }

                #[cfg(not(graphics_mode))]
                {
                    // Terminal mode: only need game config, no texture
                    rust_pixel::init_game_config(stringify!([<$name:lower>]), &pp);
                }

                #[cfg(target_arch = "wasm32")]
                {
                    // WASM mode: JS should have already called wasm_init_pixel_assets
                    // Just set game config if not already set
                    rust_pixel::init_game_config(stringify!([<$name:lower>]), &pp);
                }

                // Now create Model and Render (they can safely use symbol_map functions)
                let m = [<$name Model>]::new();
                let r = [<$name Render>]::new();
                let mut g = Game::new(m, r);
                g.init();
                [<$name Game>] { g }
            }

            #[cfg(wasm)]
            #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
            impl [<$name Game>] {
                pub fn new() -> Self {
                    init_game()
                }

                pub fn tick(&mut self, dt: f32) {
                    self.g.on_tick(dt);
                }

                pub fn key_event(&mut self, t: u8, e: web_sys::Event) {
                    let abase = &self
                        .g
                        .context
                        .adapter
                        .as_any()
                        .downcast_ref::<WebAdapter>()
                        .unwrap()
                        .base;
                    if let Some(pe) = input_events_from_web(t, e, abase.gr.pixel_h, abase.gr.ratio_x, abase.gr.ratio_y, abase.gr.use_tui_height) {
                        self.g.context.input_events.push(pe);
                    }
                }

                /// Initialize WebGL renderer using pre-cached texture data
                ///
                /// Call this AFTER wasm_init_pixel_assets() to initialize the WebGL renderer
                /// using the cached texture data. This is the preferred approach.
                ///
                /// # JavaScript Example
                /// ```js
                /// wasm_init_pixel_assets("my_game", tex_w, tex_h, imgdata, symbolMapJson);
                /// const sg = PixelGame.new();
                /// sg.init_from_cache();  // Initialize WebGL using cached texture
                /// ```
                pub fn init_from_cache(&mut self) {
                    let wa = &mut self
                        .g
                        .context
                        .adapter
                        .as_any()
                        .downcast_mut::<WebAdapter>()
                        .unwrap();

                    wa.init_glpix_from_cache();
                    info!("RUST: WebGL initialized from cached texture data");
                }

                /// DEPRECATED: Use wasm_init_pixel_assets() + init_from_cache() instead
                pub fn upload_imgdata(&mut self, w: i32, h: i32, d: &js_sys::Uint8ClampedArray) {
                    let length = d.length() as usize;
                    let mut pixels = vec![0u8; length];
                    d.copy_to(&mut pixels);
                    info!("RUST...pixels.len={}", pixels.len());

                    let wa = &mut self
                        .g
                        .context
                        .adapter
                        .as_any()
                        .downcast_mut::<WebAdapter>()
                        .unwrap();

                    wa.init_glpix(w, h, &pixels);
                }

                pub fn on_asset_loaded(&mut self, url: &str, data: &[u8]) {
                    self.g.context.asset_manager.set_data(url, data);
                }

                pub fn get_ratiox(&mut self) -> f32 {
                    self.g.context.adapter.get_base().gr.ratio_x
                }

                pub fn get_ratioy(&mut self) -> f32 {
                    self.g.context.adapter.get_base().gr.ratio_y
                }
            }

            pub fn run() {
                let mut g = init_game().g;
                g.run().unwrap();
                g.render.panel.reset(&mut g.context);
            }
        }
    };
}

/// Algorithms and data structures used by demos and utilities (e.g., disjoint‑set/union‑find,
/// A* pathfinding).
pub mod algorithm;

/// Resource/asset manager with optional asynchronous loading for better compatibility with WASM.
#[cfg(not(feature = "base"))]
pub mod asset;

/// Event system for input, timers, and custom user events.
pub mod event;

// /// Alternative event implementation used for benchmarking and mutex‑based comparisons.
// pub mod event_mutex;

/// Common utilities and data structures such as object pools, RNG, matrices, circles, and dots.
pub mod util;

/// Audio playback utilities and abstractions.
#[cfg(not(feature = "base"))]
pub mod audio;

/// Runtime context, including the active rendering adapter and other shared state.
#[cfg(not(feature = "base"))]
pub mod context;

/// Game orchestration: integrates model and renderer, encapsulating the main loop.
#[cfg(not(feature = "base"))]
pub mod game;

/// Logging facilities tailored for demos and examples.
pub mod log;

/// Rendering subsystem supporting both text and graphics modes.
///
/// Components:
/// - Adapter: Rendering adapter interface (crossterm; winit + glow/wgpu; SDL + glow; Web).
/// - Cell: Base drawing unit (character in text mode; glyph/small bitmap in graphics mode).
/// - Buffer: Screen buffer built from cells, with efficient updates.
/// - Sprite: Higher‑level drawing primitive built on top of buffers.
/// - Style: Foreground/background colors and other attributes.
/// - Panel: Unified drawing surface that works in both modes.
///
/// In text mode a cell is a Unicode character. In graphics mode a cell can be a fixed‑size dot
/// matrix image, a PETSCII character, or a custom texture. Graphics mode also supports per‑sprite
/// pixel offsets to improve expressiveness.
#[cfg(not(feature = "base"))]
pub mod render;

/// UI framework for building character‑based interfaces, including widgets, layouts, events,
/// and themes for rapid development.
#[cfg(not(feature = "base"))]
pub mod ui;

