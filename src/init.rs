// RustPixel - Asset Initialization Module
// copyright zipxing@hotmail.com 2022～2026

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
//! init_layered_pixel_assets("my_game", "/path/to/project", false, false)?;
//! // Now safe to create Model and Render
//! ```
//!
//! ## WASM Mode (from JavaScript)
//! ```js
//! wasm_init_pixel_assets("my_game", layerSize, layerCount, layerData, symbolMapJson);
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
    /// Whether to start in fullscreen mode
    pub fullscreen: bool,
    /// Whether to preserve aspect ratio in fullscreen (letterboxing with black borders)
    pub fullscreen_fit: bool,
}

/// Global static game configuration
pub static GAME_CONFIG: OnceLock<GameConfig> = OnceLock::new();

/// Initialize the global game configuration
///
/// This should be called once at program startup before any other initialization.
pub fn init_game_config(game_name: &str, project_path: &str, fullscreen: bool, fullscreen_fit: bool) {
    let _ = GAME_CONFIG.set(GameConfig {
        game_name: game_name.to_string(),
        project_path: project_path.to_string(),
        fullscreen,
        fullscreen_fit,
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
            fullscreen: false,
            fullscreen_fit: false,
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
// Layer Data Cache (Texture2DArray mode)
// ============================================================================

/// Cached layer data for Texture2DArray mode.
///
/// Each layer is a square RGBA image loaded from `layers/layer_N.png`.
/// All layers have the same dimensions (layer_size × layer_size).
#[derive(Debug, Clone)]
pub struct PixelLayerData {
    /// Layer size in pixels (all layers are square)
    pub layer_size: u32,
    /// Raw RGBA data for each layer
    pub layers: Vec<Vec<u8>>,
}

/// Global cached layer data
pub static PIXEL_LAYER_DATA: OnceLock<PixelLayerData> = OnceLock::new();

/// Get the cached layer data, if loaded
pub fn get_pixel_layer_data() -> Option<&'static PixelLayerData> {
    PIXEL_LAYER_DATA.get()
}

// ============================================================================
// Native Graphics Mode Initialization
// ============================================================================

/// Initialize layered pixel assets: game config + layer PNGs + layered_symbol_map.json
///
/// This is the only initialization path for graphics mode. It loads the
/// Texture2DArray layers and the layered symbol map.
///
/// After calling this function:
/// - `get_game_config()` returns the game configuration
/// - `get_pixel_layer_data()` returns the layer images
/// - `get_layered_symbol_map()` returns the layered symbol mapping
#[cfg(all(graphics_mode, not(target_arch = "wasm32")))]
pub fn init_layered_pixel_assets(
    game_name: &str,
    project_path: &str,
    fullscreen: bool,
    fullscreen_fit: bool,
) -> Result<(), String> {
    use crate::render::adapter::{PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH};
    use crate::render::symbol_map::init_layered_symbol_map_from_file;

    // 1. Set global game configuration
    init_game_config(game_name, project_path, fullscreen, fullscreen_fit);

    // 2. Find pix directory with fallback:
    //    - First try: {project_path}/assets/pix/
    //    - Fallback:  ./assets/pix/ (project root)
    let pix_base = {
        let app_pix = format!("{}{}assets/pix", project_path, std::path::MAIN_SEPARATOR);
        let app_json = format!("{}/layered_symbol_map.json", app_pix);
        if std::path::Path::new(&app_json).exists() {
            app_pix
        } else {
            let root_pix = "assets/pix".to_string();
            let root_json = format!("{}/layered_symbol_map.json", root_pix);
            if std::path::Path::new(&root_json).exists() {
                root_pix
            } else {
                return Err(format!(
                    "Cannot find layered_symbol_map.json in '{}' or '{}'",
                    app_pix, root_pix
                ));
            }
        }
    };

    // 3. Load layered_symbol_map.json (must be before setting PIXEL_SYM_WIDTH/HEIGHT)
    let json_path = format!("{}/layered_symbol_map.json", pix_base);
    init_layered_symbol_map_from_file(&json_path)?;

    let lmap = crate::render::symbol_map::get_layered_symbol_map()
        .ok_or("Layered symbol map not initialized")?;

    // 4. Set PIXEL_SYM_WIDTH/HEIGHT from symbol map's cell_pixel_size
    let cell_px = lmap.cell_pixel_size as f32;
    let _ = PIXEL_SYM_WIDTH.set(cell_px);
    let _ = PIXEL_SYM_HEIGHT.set(cell_px);

    // 5. Load layer PNGs from the same pix_base directory
    let layer_size = lmap.layer_size;
    let mut layers = Vec::with_capacity(lmap.layer_files.len());

    for layer_file in &lmap.layer_files {
        let layer_path = format!("{}/{}", pix_base, layer_file);
        let img = image::open(&layer_path)
            .map_err(|e| format!("Failed to load layer '{}': {}", layer_path, e))?
            .to_rgba8();

        if img.width() != layer_size || img.height() != layer_size {
            return Err(format!(
                "Layer {} size {}x{} != expected {}x{}",
                layer_file,
                img.width(),
                img.height(),
                layer_size,
                layer_size
            ));
        }

        layers.push(img.into_raw());
    }

    let layer_count = layers.len();

    // 6. Cache layer data
    PIXEL_LAYER_DATA
        .set(PixelLayerData { layer_size, layers })
        .map_err(|_| "PIXEL_LAYER_DATA already initialized".to_string())?;

    println!(
        "Layered pixel assets initialized: {} layers ({}x{}), {} symbols from {}",
        layer_count, layer_size, layer_size, lmap.symbol_count(), pix_base
    );

    Ok(())
}

// ============================================================================
// WASM Mode Initialization
// ============================================================================

/// Initialize pixel assets for Web/WASM mode
///
/// This function is called from JavaScript after loading the layer images
/// and layered_symbol_map.json. It initializes the layered symbol map
/// and caches the layer data for GPU upload.
///
/// # Arguments
/// * `game_name` - Game identifier
/// * `layer_size` - Size of each square layer in pixels
/// * `layer_count` - Number of layers
/// * `layer_data` - Concatenated raw RGBA pixel data for all layers
/// * `symbol_map_json` - Content of layered_symbol_map.json
///
/// # Returns
/// * `true` on success
/// * `false` on failure (error logged to console)
#[cfg(target_arch = "wasm32")]
pub fn wasm_init_pixel_assets(
    game_name: &str,
    layer_size: u32,
    layer_count: u32,
    layer_data: &[u8],
    symbol_map_json: &str,
) -> bool {
    use crate::render::adapter::{PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH};

    // 1. Set game configuration
    init_game_config(game_name, ".", true, false);

    // 2. Initialize layered symbol map (must be before setting PIXEL_SYM_WIDTH/HEIGHT)
    if let Err(e) = crate::render::symbol_map::init_layered_symbol_map_from_json(symbol_map_json) {
        web_sys::console::error_1(&format!("RUST: Failed to init symbol map: {}", e).into());
        return false;
    }

    let lmap = match crate::render::symbol_map::get_layered_symbol_map() {
        Some(m) => m,
        None => {
            web_sys::console::error_1(&"RUST: Layered symbol map not initialized".into());
            return false;
        }
    };

    // 3. Set PIXEL_SYM_WIDTH/HEIGHT from symbol map's cell_pixel_size
    let cell_px = lmap.cell_pixel_size as f32;
    web_sys::console::log_1(
        &format!(
            "RUST: layer_size={}, layer_count={}, cell_pixel_size={}",
            layer_size, layer_count, lmap.cell_pixel_size
        )
        .into(),
    );
    if PIXEL_SYM_WIDTH.set(cell_px).is_err() {
        web_sys::console::warn_1(&"PIXEL_SYM_WIDTH already initialized".into());
    }
    if PIXEL_SYM_HEIGHT.set(cell_px).is_err() {
        web_sys::console::warn_1(&"PIXEL_SYM_HEIGHT already initialized".into());
    }

    // 4. Cache layer data
    let bytes_per_layer = (layer_size * layer_size * 4) as usize;
    let mut layers = Vec::with_capacity(layer_count as usize);
    for i in 0..layer_count as usize {
        let start = i * bytes_per_layer;
        let end = start + bytes_per_layer;
        if end > layer_data.len() {
            web_sys::console::error_1(&format!("RUST: Layer data too short for layer {}", i).into());
            return false;
        }
        layers.push(layer_data[start..end].to_vec());
    }

    if PIXEL_LAYER_DATA
        .set(PixelLayerData { layer_size, layers })
        .is_err()
    {
        web_sys::console::warn_1(&"PIXEL_LAYER_DATA already initialized".into());
    }

    web_sys::console::log_1(
        &format!(
            "RUST: Layered pixel assets initialized: {} layers ({}x{}), {} symbols",
            layer_count, layer_size, layer_size, lmap.symbol_count()
        )
        .into(),
    );
    true
}
