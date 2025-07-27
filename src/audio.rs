// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Audio module provides sound playback functionality for RustPixel games.
//! 
//! This module offers cross-platform audio support through different backends:
//! - **Native**: Direct audio file playback
//! - **Web**: Browser-based audio through JavaScript interop


//! audio provides playing music and sound effect, reference
//! https://docs.rs/rodio


use crate::util::get_abs_path;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
use rodio::{source::Source, Decoder, OutputStream};
use std::fs::File;
use std::io::BufReader;

pub struct Audio {
    // Audio functionality temporarily disabled due to rodio API changes
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}

impl Audio {
    pub fn new() -> Self {
        Self {}
    }
    
    #[allow(unused)]
    pub fn play_file(&self, _fpath: &str, _is_loop: bool) {
        // Audio playback is temporarily disabled due to rodio API changes
        log::warn!("Audio playback temporarily disabled due to rodio API changes");
    }
}
