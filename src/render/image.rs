// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Implements save/load of .pix and .esc files.
//!
//! ESC file stores the ASCII art images in terminal mode, 
//! saving ESC terminal sequences
//! and UTF-8 text. Run this command to check:
//!
//! $ cat assets/tetris/back.txt
//!
//! PIX file stores the art images in graphics mode, saving the cell sequence row by row.
//! Cell: char sym index, fore- and background colors (background color is used to mark texture in graphics mode).
//!
//! $ cat assets/snake/back.pix

use crate::render::buffer::Buffer;
use log::info;
use std::io::Error;

/// helping method to convert error msg
#[cfg(not(any(feature = "sdl", feature = "winit", feature = "wgpu", target_os = "android", target_os = "ios", target_arch = "wasm32")))]
pub fn to_error(error: Result<(), Error>) -> Result<(), String> {
    error.map_err(|e| e.to_string())
}

/// helping method to convert error msg
#[cfg(not(any(feature = "sdl", feature = "winit", feature = "wgpu", target_os = "android", target_os = "ios", target_arch = "wasm32")))]
pub fn io_error(error: Result<(), Error>) -> std::io::Result<()> {
    error.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
}

#[derive(Debug)]
pub enum DatFileError {
    Io(Error),
}

impl From<Error> for DatFileError {
    fn from(err: Error) -> DatFileError {
        DatFileError::Io(err)
    }
}

fn find_vaild_area(content: &Buffer) -> (u16, u16, u16, u16) {
    let width = content.area.width;
    let rowcnt = content.area.height;
    //crop effective data, removing blank
    let (mut x1, mut y1, mut x2, mut y2) = (u16::MAX, u16::MAX, 0, 0);
    for row in 0..rowcnt {
        for col in 0..width {
            let cell = &content.content[(row * width + col) as usize];
            if !cell.is_blank() {
                if col < x1 {
                    x1 = col;
                }
                if col > x2 {
                    x2 = col;
                }
                if row < y1 {
                    y1 = row;
                }
                if row > y2 {
                    y2 = row;
                }
            }
        }
    }
    info!("save_sdlfile:: x1{} x2{} y1{} y2{}", x1, x2, y1, y2);
    (x1, x2, y1, y2)
}

pub mod pix;
pub use pix::PixAsset;

pub mod esc;
pub use esc::EscAsset;

pub mod seq_frame;
pub use seq_frame::SeqFrameAsset;
