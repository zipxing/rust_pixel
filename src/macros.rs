// RustPixel - Application Macro Module
// copyright zipxing@hotmail.com 2022～2025

//! Macro for scaffolding RustPixel applications.
//!
//! This module provides the `app!` macro which generates the boilerplate code
//! needed for a RustPixel application to run across multiple platforms.

/// Macro `app!` to scaffold a RustPixel application entry.
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
/// ```rust,ignore
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
///
/// ## Usage
///
/// In your app's `lib.rs`:
/// ```rust,ignore
/// use rust_pixel::app;
/// app!(MyGame);
/// ```
#[cfg(not(feature = "base"))]
#[macro_export]
macro_rules! app {
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

