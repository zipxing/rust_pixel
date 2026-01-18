//! # pixel_basic
//!
//! BASIC scripting support for rust_pixel game engine.
//!
//! This crate provides a BASIC interpreter with game-specific extensions
//! for creating retro-style games without writing Rust code.

// BASIC interpreter core (from BASIC-M6502.rs)
pub mod basic;

// Game integration modules
pub mod game_context;
pub mod game_bridge;
// pub mod extensions;   // TODO: implement game extension functions

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

// Re-export game context
pub use game_context::{GameContext, NullGameContext};

// Re-export game bridge
pub use game_bridge::{GameBridge, ON_INIT_LINE, ON_TICK_LINE, ON_DRAW_LINE};
