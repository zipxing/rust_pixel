// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! RustPixel is a 2D game engine & rapid prototyping tools supporting both text and graphics rendering modes.
//! It is suitable for creating 2D pixel-style games and developing terminal applications.
//! It is also a perfect choice for developing and debugging CPU-heavy core algorithms.
//! You can compile your core algorithms to FFI or WASM libraries, and use them with other gaming
//! frontends or backends.
//!
//! Text Mode: Built with crossterm, runs in the terminal, and uses ASCII & Unicode Emoji for drawing.
//! Graphical Mode (SDL2): Built with SDL2, using PETSCII & custom graphics symbols for rendering.
//! Graphical Mode (Web): Similar to the SDL2 mode, but the core logic is compiled into WASM and 
//! rendered using WebGL and JavaScript (refer to rust-pixel/web-template/pixel.js).
//!
//! In RustPixel, game scenes are rendered using individual Cell and managed by Buffer.
//!
//! Various modules asset, audio, event, game, log, render, algorithm, util are offered to ease
//! game development.
//!
//! We also provide a base mode in which only algorithm, event and util modules are compiled.
//! Base mode requires fewer dependencies and therefore it is a good fit for compiling to FFI
//! or WASM libraries.

/// framerate per second, set to moderate number to save CPUs
pub const GAME_FRAME: u32 = 60;
#[cfg(not(graphics_mode))]
pub const LOGO_FRAME: u32 = GAME_FRAME / 4 * 2;
#[cfg(graphics_mode)]
pub const LOGO_FRAME: u32 = GAME_FRAME / 4 * 5;

/// Re-export paste for use in macros
#[cfg(not(feature = "base"))]
pub use paste;

/// pixel_game macro for creating game applications
///
/// ## Architecture Design Philosophy
/// 
/// This macro generates a lib.rs + main.rs separation pattern for RustPixel applications.
/// While it might seem redundant to have both files, this design is essential for supporting
/// multiple deployment targets and maintaining a unified architecture across the framework.
///
/// ### Why Separate lib.rs and main.rs?
///
/// **1. WASM Support Requirements:**
/// - WASM compilation requires `crate-type = ["cdylib", "rlib"]` in Cargo.toml
/// - WASM cannot use `main()` function directly - it needs exported functions for JavaScript
/// - The generated `GameStruct` with methods like `new()`, `tick()`, `key_event()` can be 
///   exported to WASM and called from JavaScript/WebGL frontend
///
/// **2. Multi-Platform Deployment:**
/// - **Native Binary**: main.rs → calls `game_name::run()` from lib.rs
/// - **WASM Library**: JavaScript → calls `GameStruct::new()`, `tick()`, etc. from lib.rs  
/// - **Library Dependency**: Other projects can depend on the crate as a library
///
/// **3. Unified Architecture:**
/// - All RustPixel games follow the same pattern: Model + Render + Game struct
/// - Consistent interface across Terminal, SDL, and Web rendering backends
/// - Enables framework-wide optimizations and feature additions
///
/// **4. Code Organization Benefits:**
/// - Clear separation of concerns: lib.rs defines the API, main.rs provides entry point
/// - Facilitates testing (can test library functions without main())
/// - Enables conditional compilation for different platforms and features
///
/// ### What This Macro Generates:
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
/// pub fn init_game() -> {Name}Game { ... }
/// pub fn run() { ... }  // Called by main.rs
/// 
/// // WASM-specific exports
/// impl {Name}Game {
///     pub fn new() -> Self { ... }        // WASM constructor
///     pub fn tick(&mut self, dt: f32) { ... }      // WASM game loop
///     pub fn key_event(&mut self, ...) { ... }     // WASM input handling
/// }
/// ```
///
/// This design enables RustPixel applications to run seamlessly across:
/// - Terminal (crossterm backend)
/// - Desktop (SDL2 backend) 
/// - Web (WebGL backend via WASM)
/// - As library dependencies in other Rust projects
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
            #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
            pub struct [<$name Game>] {
                g: Game<[<$name Model>], [<$name Render>]>,
            }

            pub fn init_game() -> [<$name Game>] {
                let m = [<$name Model>]::new();
                let r = [<$name Render>]::new();
                let pp = get_project_path();
                println!("asset path : {:?}", pp);
                let mut g = Game::new(m, r, stringify!([<$name:lower>]), &pp);
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
                    if let Some(pe) = input_events_from_web(t, e, abase.gr.pixel_h, abase.gr.ratio_x, abase.gr.ratio_y) {
                        self.g.context.input_events.push(pe);
                    }
                }

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

/// disjoint-set data structure, astar
pub mod algorithm;

/// resource manager, supporting async load to better compatible with wasm mode
#[cfg(not(feature = "base"))]
pub mod asset;

/// processing input events, timer and other custom events
pub mod event;

/// comparison module for event system benchmarking
pub mod event_mutex;

/// common tools and data structures:
/// object pool, RNG, matrix, circle, dots
pub mod util;

/// calls audio module to play sounds
#[cfg(not(feature = "base"))]
pub mod audio;

/// public variables, including rendering adapter
#[cfg(not(feature = "base"))]
pub mod context;

/// integrates model and render, encapsulates the main loop
#[cfg(not(feature = "base"))]
pub mod game;

/// log
pub mod log;

/// Render module, it supports two rendering mode: text mode and graphics mode.
/// adapter: render adapter interface (crossterm, sdl, web).
/// cell: a base drawing unit i.e. a character.
/// buffer: a vector comprised of cells, managing screen buffer.
/// sprite: basic drawing component, encapsulating further the buffer.
/// style: define drawing attributes such as fore- and back-ground colors.
/// panel: drawing panel is compatible with text mode and graphics mode.
///
/// cell is a unicode character in text mode，
///
/// cell can be a fixed size dot matrix image, PETSCII char
/// or other custom images in graphics mode
///
/// It supports offsetting special sprite by pixels to enhance expressiveness
/// in graphics mode.
#[cfg(not(feature = "base"))]
pub mod render;

