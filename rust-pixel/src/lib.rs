// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024

//! RustPixel is a rust 2d mini-game engine.
//! It is ideal for fast prototyping of 2d pixel style games
//! it is also a perfect choice for developing and debugging CPU-heavy game core algorithms
//! You can compile your core algorithms to ffi or wasm libs, and used by other gaming
//! frontend or backend
//!
//! It supports two rendering mode: text mode and graphical mode.
//! Text mode runs in a terminal.
//! Graphical mode supports SDL running in a OS window or wasm on a webpage.
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

/// disjoint-set data structure, astar
pub mod algorithm;

/// resource manager, supporting async load to better compatible with wasm web mode
#[cfg(not(feature = "base"))]
pub mod asset;

/// processing input events, timer and other custom events
pub mod event;

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

/// Render module, it supports two rendering mode: text mode and graphical mode.
/// adapter: render adapter interface (crossterm, sdl, web).
/// cell: a bse drawing unit i.e. a character.
/// buffer: a vector comprised of cells, managing screen buffer.
/// sprite: basic drawing component, encapsulating further the buffer.
/// style: define drawing attributes such as fore- and back-ground colors.
/// panel: drawing panel is compatible with text mode and graphical mode.
///
/// cell is a unicode character in text mode，
///
/// cell can be a fixed size dot matrix image, PETSCII char
/// or other custom images in graphical mode
///
/// It supports offsetting special sprite by pixels to enhance expressiveness
/// in graphical mode.
#[cfg(not(feature = "base"))]
pub mod render;
