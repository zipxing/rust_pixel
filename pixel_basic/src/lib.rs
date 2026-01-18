//! # pixel_basic
//!
//! BASIC scripting support for rust_pixel game engine.
//!
//! This crate provides a BASIC interpreter with game-specific extensions
//! for creating retro-style games without writing Rust code.

// BASIC interpreter core (from BASIC-M6502.rs)
pub mod basic;

// Game integration modules (to be implemented)
// pub mod game_context;
// pub mod game_bridge;
// pub mod extensions;

// Re-export core types
pub use basic::{
    error::{BasicError, Result},
    token::Token,
    tokenizer::Tokenizer,
    ast::*,
    parser::Parser,
    runtime::Runtime,
    variables::{Variables, Value, Array},
    executor::{Executor, DataValue},
};
