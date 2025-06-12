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
#[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
pub const LOGO_FRAME: u32 = GAME_FRAME / 4 * 2;
#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
pub const LOGO_FRAME: u32 = GAME_FRAME / 4 * 5;

/// proc macro for pixel_game!
pub use pixel_macro;

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

