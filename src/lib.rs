// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! RustPixel is a 2D game engine & rapid prototyping tools supporting both text and graphics rendering modes.
//! It is suitable for creating 2D pixel-style games and developing terminal applications.
//! it is also a perfect choice for developing and debugging CPU-heavy core algorithms
//! You can compile your core algorithms to ffi or wasm libs, and used by other gaming
//! frontend or backend
//!
//! Text Mode: Built with crossterm, runs in the terminal, and uses ASCII & Unicode Emoji for drawing.
//! Graphical Mode (SDL2): Built with SDL2, using PETSCII & custom graphics symbols for rendering.
//! Graphical Mode (Web): Similar to the SDL2 mode, but the core logic is compiled into WASM and 
//! rendered using WebGL and JavaScript (refer to rust-pixel/web-template/pixel.js).
//!
//! In RustPixel, game scenes are rendered using individual Cell and managed by Buffer
//!
//! Various modules asset, audio, event, game, log, render, algorithm, util are offered to ease
//! game development
//!
//! We also provide a base mode in which only algorithm, event and util modules are compiled.
//! Base mode requires fewer dependencies and therefore it is a good fit for compiling to ffi
//! or wasm libs.

/// framerate per second, set to moderate number to save CPUs
pub const GAME_FRAME: u32 = 60;
#[cfg(not(any(feature = "sdl", feature = "winit", target_arch = "wasm32")))]
pub const LOGO_FRAME: u32 = GAME_FRAME / 4 * 2;
#[cfg(any(feature = "sdl", feature = "winit", target_arch = "wasm32"))]
pub const LOGO_FRAME: u32 = GAME_FRAME / 4 * 5;

/// Re-export paste for use in macros
#[cfg(not(feature = "base"))]
pub use paste;

/// pixel_game macro for creating game applications
#[cfg(not(feature = "base"))]
#[macro_export]
macro_rules! pixel_game {
    ($name:ident) => {
        mod model;
        #[cfg(not(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32")))]
        mod render_terminal;
        #[cfg(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32"))]
        mod render_graphics;

        #[cfg(not(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32")))]
        use crate::{model::*, render_terminal::*};
        #[cfg(any(feature = "sdl", feature = "winit", feature = "wgpu", target_arch = "wasm32"))]
        use crate::{model::*, render_graphics::*};
        use rust_pixel::game::Game;
        use rust_pixel::util::get_project_path;

        #[cfg(target_arch = "wasm32")]
        use rust_pixel::render::adapter::web::{input_events_from_web, WebAdapter};
        use wasm_bindgen::prelude::*;
        #[cfg(target_arch = "wasm32")]
        use wasm_bindgen_futures::js_sys;
        #[cfg(target_arch = "wasm32")]
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
                info!("asset path : {:?}", pp);
                let mut g = Game::new(m, r, stringify!([<$name:lower>]), &pp);
                g.init();
                [<$name Game>] { g }
            }

            #[cfg(target_arch = "wasm32")]
            #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
            impl [<$name Game>] {
                pub fn new() -> Self {
                    info!("hahahahahhahahaha....");
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

