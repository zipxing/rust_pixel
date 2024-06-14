// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024


//! audio provides playing music and sound effect, reference
//! https://docs.rs/rodio


use crate::util::get_abs_path;
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
use rodio::{source::Source, Decoder, OutputStream, OutputStreamHandle};
use std::fs::File;
use std::io::BufReader;

pub struct Audio {
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    _out: OutputStream,
    #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
    handle: OutputStreamHandle,
}

impl Audio {
    pub fn new() -> Self {
        #[cfg(any(target_os = "android", target_os = "ios", target_arch = "wasm32"))]
        {
            Self {}
        }
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        {
            let (s, h) = OutputStream::try_default().unwrap();
            Self { _out: s, handle: h }
        }
    }
    #[allow(unused)]
    pub fn play_file(&self, fpath: &str, is_loop: bool) {
        let fpstr = get_abs_path(fpath);
        let file = BufReader::new(File::open(fpstr).unwrap());
        #[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
        {
            if is_loop {
                let source = Decoder::new(file).unwrap().repeat_infinite();
                self.handle.play_raw(source.convert_samples()).unwrap();
            } else {
                let source = Decoder::new(file).unwrap();
                self.handle.play_raw(source.convert_samples()).unwrap();
            };
        }
    }
}
