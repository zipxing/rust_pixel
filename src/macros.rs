// RustPixel - Application Macro Module
// copyright zipxing@hotmail.com 2022～2025

//! Macro for scaffolding RustPixel applications.
//!
//! This module provides the `app!` macro which generates the boilerplate code
//! needed for a RustPixel application to run across multiple platforms.

/// Internal macro containing shared application scaffolding code.
///
/// This is used by `app!` to avoid duplicating the Game struct, init_game,
/// WASM exports, and run function across different render module strategies.
#[cfg(not(feature = "base"))]
#[macro_export]
macro_rules! app_body {
    ($name:ident) => {
        use rust_pixel::game::Game;
        use rust_pixel::util::{get_project_path, is_fullscreen_requested, is_fullscreen_fit_requested};

        #[cfg(wgpu_web_backend)]
        use rust_pixel::render::adapter::wgpu_web_adapter::{input_events_from_web, WgpuWebAdapter};
        use wasm_bindgen::prelude::*;
        #[cfg(wgpu_web_backend)]
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

            /// Pass app-specific text data from JavaScript before game creation.
            /// Use URL parameter `?data=assets/demo.md` to specify the data file.
            #[cfg(target_arch = "wasm32")]
            #[wasm_bindgen]
            pub fn wasm_set_app_data(data: &str) {
                rust_pixel::set_wasm_app_data(data);
            }

            #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
            pub struct [<$name Game>] {
                g: Game<[<$name Model>], [<$name Render>]>,
            }

            pub fn init_game() -> [<$name Game>] {
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"[init_game] start".into());

                let pp = get_project_path();
                let fullscreen = is_fullscreen_requested();
                let fullscreen_fit = is_fullscreen_fit_requested();
                println!("asset path : {:?}, fullscreen: {}, fullscreen_fit: {}", pp, fullscreen, fullscreen_fit);

                // Initialize assets based on mode:
                // - Graphics mode (native): load texture + symbol_map via init_pixel_assets
                // - Terminal mode: only set game config (no texture needed)
                // - WASM mode: JS already called wasm_init_pixel_assets before this
                #[cfg(all(graphics_mode, not(target_arch = "wasm32")))]
                {
                    rust_pixel::init_pixel_assets(stringify!([<$name:lower>]), &pp, fullscreen, fullscreen_fit)
                        .expect("Failed to initialize pixel assets");
                }

                #[cfg(not(graphics_mode))]
                {
                    // Terminal mode: only need game config, no texture
                    rust_pixel::init_game_config(stringify!([<$name:lower>]), &pp, fullscreen, fullscreen_fit);
                }

                #[cfg(target_arch = "wasm32")]
                {
                    // WASM mode: JS should have already called wasm_init_pixel_assets
                    // Just set game config if not already set
                    web_sys::console::log_1(&"[init_game] calling init_game_config for wasm".into());
                    rust_pixel::init_game_config(stringify!([<$name:lower>]), &pp, true, false);
                }

                // Now create Model and Render (they can safely use symbol_map functions)
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"[init_game] creating Model...".into());
                let m = [<$name Model>]::new();

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"[init_game] creating Render...".into());
                let r = [<$name Render>]::new();

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"[init_game] creating Game...".into());
                let mut g = Game::new(m, r);

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"[init_game] calling g.init()...".into());
                g.init();

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&"[init_game] done!".into());
                [<$name Game>] { g }
            }

            #[cfg(wgpu_web_backend)]
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
                        .downcast_ref::<WgpuWebAdapter>()
                        .unwrap()
                        .base;
                    if let Some(pe) = input_events_from_web(t, e, abase.gr.pixel_h, abase.gr.ratio_x, abase.gr.ratio_y, abase.gr.use_tui_height) {
                        self.g.context.input_events.push(pe);
                    }
                }

                /// Initialize WGPU renderer using pre-cached texture data
                ///
                /// Call this AFTER wasm_init_pixel_assets() to initialize the WGPU renderer
                /// using the cached texture data. Uses WebGPU if available, falls back to WebGL2.
                ///
                /// # JavaScript Example
                /// ```js
                /// wasm_init_pixel_assets("my_game", tex_w, tex_h, imgdata, symbolMapJson);
                /// const sg = PixelGame.new();
                /// await sg.init_from_cache();  // Initialize WGPU using cached texture (async!)
                /// ```
                pub async fn init_from_cache(&mut self) {
                    let wa = self
                        .g
                        .context
                        .adapter
                        .as_any()
                        .downcast_mut::<WgpuWebAdapter>()
                        .unwrap();

                    wa.init_wgpu_from_cache_async().await;
                    info!("RUST: WGPU Web initialized from cached texture data");
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

                /// Get the actual canvas size (pixel dimensions) for rendering
                ///
                /// Returns the width and height that should be used for the HTML canvas
                /// to match the WGPU surface size exactly, avoiding scaling artifacts.
                ///
                /// # Returns
                /// A JavaScript array [width, height] in pixels
                pub fn get_canvas_size(&self) -> Vec<u32> {
                    let (w, h) = self.g.context.adapter.get_canvas_size();
                    vec![w, h]
                }
            }

            pub fn run() {
                let mut g = init_game().g;
                g.run().unwrap();
                g.render.scene.reset(&mut g.context);
            }
        }
    };
}

/// Macro `app!` to scaffold a RustPixel application entry.
///
/// ## Usage
///
/// ### Dual-file mode (separate render_terminal.rs + render_graphics.rs):
/// ```rust,ignore
/// use rust_pixel::app;
/// app!(MyGame);
/// ```
///
/// ### Unified mode (single render.rs with cfg for minor differences):
/// ```rust,ignore
/// use rust_pixel::app;
/// app!(MyGame, unified);
/// ```
#[cfg(not(feature = "base"))]
#[macro_export]
macro_rules! app {
    // Unified render mode: single render.rs file
    ($name:ident, unified) => {
        mod model;
        mod render;
        use crate::{model::*, render::*};
        $crate::app_body!($name);
    };
    // Dual-file render mode (default, backward compatible)
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
        $crate::app_body!($name);
    };
}
