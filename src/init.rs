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
//! init_layered_pixel_assets("my_game", "/path/to/project", WindowMode::Window)?;
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
// Window Mode
// ============================================================================

/// Window display mode for the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowMode {
    /// Normal windowed mode
    #[default]
    Window,
    /// Fullscreen stretched to fill the entire screen
    Fullscreen,
    /// Fullscreen with preserved aspect ratio (letterboxing with black borders)
    FullscreenFit,
}

impl WindowMode {
    /// Returns true if the application should run in any fullscreen mode
    pub fn is_fullscreen(&self) -> bool {
        matches!(self, WindowMode::Fullscreen | WindowMode::FullscreenFit)
    }

    /// Returns true if letterboxing (aspect ratio preservation) is enabled
    pub fn is_fit(&self) -> bool {
        matches!(self, WindowMode::FullscreenFit)
    }
}

/// High-level runtime mode for an application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunMode {
    /// Existing 2D/TUI runtime.
    #[default]
    TwoD,
    /// New voxel-oriented 3D runtime.
    ThreeD,
}

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
    /// Window display mode
    pub window_mode: WindowMode,
    /// High-level render/runtime mode
    pub run_mode: RunMode,
}

/// Global static game configuration
pub static GAME_CONFIG: OnceLock<GameConfig> = OnceLock::new();

/// Initialize the global game configuration
///
/// This should be called once at program startup before any other initialization.
pub fn init_game_config(game_name: &str, project_path: &str, window_mode: WindowMode) {
    let _ = GAME_CONFIG.set(GameConfig {
        game_name: game_name.to_string(),
        project_path: project_path.to_string(),
        window_mode,
        run_mode: crate::util::parse_run_mode(),
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
            window_mode: WindowMode::Window,
            run_mode: RunMode::TwoD,
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

/// Cached layer data (temporary, cleared after GPU upload to save memory).
/// Uses thread_local since the game loop is single-threaded.
use std::cell::RefCell;
thread_local! {
    static PIXEL_LAYER_DATA: RefCell<Option<PixelLayerData>> = const { RefCell::new(None) };
}

/// Access the cached layer data via closure.
/// Returns None if not yet initialized or already cleared.
pub fn with_pixel_layer_data<R>(f: impl FnOnce(&PixelLayerData) -> R) -> Option<R> {
    PIXEL_LAYER_DATA.with(|cell| {
        cell.borrow().as_ref().map(f)
    })
}

/// Release the cached layer data to free CPU memory after GPU upload.
pub fn clear_pixel_layer_data() {
    PIXEL_LAYER_DATA.with(|cell| {
        if let Some(data) = cell.borrow_mut().take() {
            let total_bytes: usize = data.layers.iter().map(|l| l.len()).sum();
            log::info!(
                "Cleared PIXEL_LAYER_DATA: freed ~{:.1} MB of CPU memory",
                total_bytes as f64 / (1024.0 * 1024.0)
            );
        }
    });
}

// ============================================================================
// Shared initialization logic
// ============================================================================

/// Internal shared initialization: symbol map + PIXEL_SYM sizes + layer data cache.
///
/// Both native and WASM paths call this after preparing the symbol map JSON
/// and raw layer data.
#[cfg(any(feature = "wgpu", target_arch = "wasm32"))]
fn init_pixel_assets_inner(
    layer_size: u32,
    layers: Vec<Vec<u8>>,
) -> Result<(), String> {
    use crate::render::adapter::{PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH};

    let lmap = crate::render::symbol_map::get_layered_symbol_map()
        .ok_or("Layered symbol map not initialized")?;

    // Set PIXEL_SYM_WIDTH/HEIGHT from symbol map's cell_pixel_size
    let cell_px = lmap.cell_pixel_size as f32;
    let _ = PIXEL_SYM_WIDTH.set(cell_px);
    let _ = PIXEL_SYM_HEIGHT.set(cell_px);

    let layer_count = layers.len();

    // Cache layer data
    PIXEL_LAYER_DATA.with(|cell| {
        *cell.borrow_mut() = Some(PixelLayerData { layer_size, layers });
    });

    log::info!(
        "Layered pixel assets initialized: {} layers ({}x{}), {} symbols",
        layer_count, layer_size, layer_size, lmap.symbol_count()
    );

    Ok(())
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
#[cfg(feature = "wgpu")]
pub fn init_layered_pixel_assets(
    game_name: &str,
    project_path: &str,
    window_mode: WindowMode,
) -> Result<(), String> {
    use crate::render::symbol_map::init_layered_symbol_map_from_file;

    // 1. Set global game configuration
    init_game_config(game_name, project_path, window_mode);

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

    // 3. Load layered_symbol_map.json
    let json_path = format!("{}/layered_symbol_map.json", pix_base);
    init_layered_symbol_map_from_file(&json_path)?;

    let lmap = crate::render::symbol_map::get_layered_symbol_map()
        .ok_or("Layered symbol map not initialized")?;

    // 4. Load layer PNGs
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

    // 5. Shared init: set sym sizes + cache layers
    init_pixel_assets_inner(layer_size, layers)?;

    log::info!("Pix base: {}", pix_base);

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
    // 1. Set game configuration (WASM always fullscreen)
    init_game_config(game_name, ".", WindowMode::Fullscreen);

    // 2. Initialize layered symbol map
    if let Err(e) = crate::render::symbol_map::init_layered_symbol_map_from_json(symbol_map_json) {
        log::error!("Failed to init symbol map: {}", e);
        return false;
    }

    // 3. Split concatenated layer data into individual layers
    let bytes_per_layer = (layer_size * layer_size * 4) as usize;
    let mut layers = Vec::with_capacity(layer_count as usize);
    for i in 0..layer_count as usize {
        let start = i * bytes_per_layer;
        let end = start + bytes_per_layer;
        if end > layer_data.len() {
            log::error!("Layer data too short for layer {}", i);
            return false;
        }
        layers.push(layer_data[start..end].to_vec());
    }

    // 4. Shared init: set sym sizes + cache layers
    match init_pixel_assets_inner(layer_size, layers) {
        Ok(()) => true,
        Err(e) => {
            log::error!("Failed to init pixel assets: {}", e);
            false
        }
    }
}
