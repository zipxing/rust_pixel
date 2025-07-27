// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Audio module provides sound playback functionality for RustPixel games.
//! 
//! This module offers cross-platform audio support through different backends:
//! - **Native**: Direct audio file playback via rodio
//! - **Web**: Browser-based audio through JavaScript interop

//! audio provides playing music and sound effect, reference
//! https://docs.rs/rodio

use crate::util::get_abs_path;
use std::fs::File;
use std::io::BufReader;

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
use rodio::{Decoder, OutputStreamBuilder, source::Source};

pub struct Audio {
    // Note: We cannot store the OutputStreamHandle here because it's not Send + Sync
    // This is a limitation of the current rodio design
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
    
    pub fn play_file(&self, fpath: &str, is_loop: bool) {
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        {
            let path = get_abs_path(fpath);
            
            // Create stream handle each time we play
            // Note: This will only play for a short time as the handle gets dropped
            // This is a limitation we need to work around in the future
            match OutputStreamBuilder::open_default_stream() {
                Ok(stream_handle) => {
                    match File::open(&path) {
                        Ok(file) => {
                            let buf_reader = BufReader::new(file);
                            match Decoder::try_from(buf_reader) {
                                Ok(source) => {
                                    let final_source = if is_loop {
                                        Box::new(source.repeat_infinite()) as Box<dyn Source<Item = f32> + Send>
                                    } else {
                                        Box::new(source) as Box<dyn Source<Item = f32> + Send>
                                    };
                                    
                                    stream_handle.mixer().add(final_source);
                                    log::info!("Audio file queued for playback: {}", path);
                                    log::warn!("Audio stream will stop shortly due to rodio API limitations");
                                    
                                    // TODO: Find a better way to keep the stream alive
                                    // The stream handle will be dropped here, stopping playback
                                }
                                Err(e) => {
                                    log::warn!("Failed to decode audio file '{}': {}", path, e);
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to open audio file '{}': {}", path, e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to initialize audio stream: {}", e);
                }
            }
        }
        #[cfg(any(target_os = "android", target_os = "ios", target_arch = "wasm32"))]
        {
            log::warn!("Audio playback not supported on this platform: {}", fpath);
        }
    }
}
