// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Audio module provides sound playback functionality for RustPixel games.
//!
//! This module offers cross-platform audio support through different backends:
//! - **Native**: Direct audio file playback via rodio
//! - **Web**: Browser-based audio through JavaScript interop

//! audio provides playing music and sound effect, reference
//! https://docs.rs/rodio

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
use crate::util::get_project_path;

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
use rodio::{Decoder, OutputStreamBuilder, Source};
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
use std::fs::File;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
use std::io::BufReader;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
use std::sync::{Arc, Mutex, OnceLock};
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
use std::thread;

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
type AudioStreamHandle = Box<dyn std::any::Any + Send>;

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
static GLOBAL_AUDIO_HANDLE: OnceLock<Arc<Mutex<Option<AudioStreamHandle>>>> = OnceLock::new();

#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
fn get_or_create_audio_handle() -> Arc<Mutex<Option<AudioStreamHandle>>> {
    GLOBAL_AUDIO_HANDLE
        .get_or_init(|| Arc::new(Mutex::new(None)))
        .clone()
}

pub struct Audio {}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}

impl Audio {
    pub fn new() -> Self {
        Self {}
    }

    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    pub fn play_file(&self, fpath: &str, is_loop: bool) {
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        {
            let project_path = get_project_path();
            let path = format!("{}/assets/{}", project_path, fpath);
            log::info!("Attempting to play audio file: {}", path);

            let audio_handle = get_or_create_audio_handle();
            let path_clone = path.clone();

            // Spawn a thread to handle audio playback
            thread::spawn(move || {
                match OutputStreamBuilder::open_default_stream() {
                    Ok(stream_handle) => {
                        // Store the handle to keep it alive
                        {
                            let mut handle_guard = audio_handle.lock().unwrap();
                            *handle_guard = Some(Box::new(()) as AudioStreamHandle);
                        }

                        match File::open(&path_clone) {
                            Ok(file) => {
                                match Decoder::try_from(BufReader::new(file)) {
                                    Ok(source) => {
                                        let final_source = if is_loop {
                                            Box::new(source.repeat_infinite())
                                                as Box<dyn Source<Item = f32> + Send>
                                        } else {
                                            Box::new(source) as Box<dyn Source<Item = f32> + Send>
                                        };

                                        stream_handle.mixer().add(final_source);
                                        log::info!("Audio file started playing: {}", path_clone);

                                        // Keep the thread alive to maintain the audio stream
                                        if is_loop {
                                            // For looping audio, keep the thread alive indefinitely
                                            loop {
                                                thread::sleep(std::time::Duration::from_secs(1));
                                            }
                                        } else {
                                            // For non-looping audio, estimate duration and sleep
                                            thread::sleep(std::time::Duration::from_secs(10));
                                        }
                                    }
                                    Err(e) => log::warn!(
                                        "Failed to decode audio file '{}': {}",
                                        path_clone,
                                        e
                                    ),
                                }
                            }
                            Err(e) => {
                                log::warn!("Failed to open audio file '{}': {}", path_clone, e)
                            }
                        }
                    }
                    Err(e) => log::warn!("Failed to open audio stream: {}", e),
                }
            });
        }

        #[cfg(any(target_os = "android", target_os = "ios", target_arch = "wasm32"))]
        {
            log::info!("Audio playback not supported on this platform: {}", fpath);
        }
    }
}
