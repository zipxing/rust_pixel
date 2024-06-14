// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024

//! code for rendering

//! An adapter interface is defined here, to interface between different renders.
//! Currently, web, SDL and crossterm renders are supported.

pub mod adapter;

/// draw a base unit cell
pub mod cell;

/// Buffer is used to manage a set of Cell
pub mod buffer;

/// image, to read or write image files in pix or esc format
pub mod image;

/// sprite, basic drawing unit
pub mod sprite;

/// defines attributes like fore- or back-ground colors
pub mod style;

/// draw panel, compatible with both text mode (crossterm) and graphical mode (SDL&wasm)
pub mod panel;

