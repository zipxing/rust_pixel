// RustPixel - Asset Initialization Module
// copyright zipxing@hotmail.com 2022～2025

//! Unified asset initialization for RustPixel applications.
//!
//! This module provides functions and types for initializing game configuration
//! and loading texture/symbol assets. It supports both native (graphics mode)
//! and WebAssembly builds with a consistent API.
//!
//! # Initialization Flow
//!
//! ## Native Graphics Mode
//! ```ignore
//! init_pixel_assets("my_game", "/path/to/project")?;
//! // Now safe to create Model and Render
//! ```
//!
//! ## WASM Mode (from JavaScript)
//! ```js
//! wasm_init_pixel_assets("my_game", tex_w, tex_h, imgdata, symbolMapJson);
//! const game = PixelGame.new();
//! game.init_from_cache();
//! ```

use std::sync::OnceLock;

// ============================================================================
// Game Configuration
// ============================================================================

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

/// Initialize the global game configuration
///
/// This should be called once at program startup before any other initialization.
pub fn init_game_config(game_name: &str, project_path: &str) {
    let _ = GAME_CONFIG.set(GameConfig {
        game_name: game_name.to_string(),
        project_path: project_path.to_string(),
    });
}

/// Get a reference to the global game configuration
///
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

// ============================================================================
// App Data Cache (for passing text data from JS to WASM at startup)
// ============================================================================

/// App-specific text data passed from JavaScript before game creation.
///
/// Use `?data=assets/demo.md` URL parameter in the browser to load data.
pub static WASM_APP_DATA: OnceLock<String> = OnceLock::new();

/// Store app data (called from JavaScript before game creation)
pub fn set_wasm_app_data(data: &str) {
    let _ = WASM_APP_DATA.set(data.to_string());
}

/// Retrieve app data passed from JavaScript, if any
pub fn get_wasm_app_data() -> Option<&'static str> {
    WASM_APP_DATA.get().map(|s| s.as_str())
}

// ============================================================================
// Texture Data Cache
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
    PIXEL_TEXTURE_DATA
        .get()
        .expect("Texture data not loaded - call init_pixel_assets() first")
}

// ============================================================================
// Native Graphics Mode Initialization
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
    use crate::render::adapter::{
        init_sym_height, init_sym_width, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, PIXEL_TEXTURE_FILE,
    };

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
    crate::render::symbol_map::init_symbol_map_from_file()?;

    println!(
        "Pixel assets initialized: {}x{} texture, symbol_map loaded from {}",
        width, height, project_path
    );

    Ok(())
}

// ============================================================================
// WASM Mode Initialization
// ============================================================================

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
#[cfg(target_arch = "wasm32")]
pub fn wasm_init_pixel_assets(
    game_name: &str,
    tex_w: u32,
    tex_h: u32,
    tex_data: &[u8],
    symbol_map_json: &str,
) -> bool {
    use crate::render::adapter::{init_sym_height, init_sym_width, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH};

    // 1. Set game configuration (use "." as project_path for web mode)
    init_game_config(game_name, ".");

    // 2. Set PIXEL_SYM_WIDTH/HEIGHT
    let sym_w = init_sym_width(tex_w);
    let sym_h = init_sym_height(tex_h);
    web_sys::console::log_1(
        &format!(
            "RUST: tex_w={}, tex_h={}, sym_width={}, sym_height={}",
            tex_w, tex_h, sym_w, sym_h
        )
        .into(),
    );
    if PIXEL_SYM_WIDTH.set(sym_w).is_err() {
        web_sys::console::warn_1(&"PIXEL_SYM_WIDTH already initialized".into());
    }
    if PIXEL_SYM_HEIGHT.set(sym_h).is_err() {
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
    match crate::render::symbol_map::init_symbol_map_from_json(symbol_map_json) {
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

