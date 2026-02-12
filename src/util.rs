// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Utilities for random, rect, PointU16...
//! and a simple object pool: objpool.rs
//! Some primitive algorithms: shape.rs

use serde::{Deserialize, Serialize};
use std::{
    cmp::{max, min},
    env,
    ffi::OsString,
    fs::read_dir,
    io::{self, ErrorKind},
    path::{Path, PathBuf, MAIN_SEPARATOR},
};
use keyframe_derive::CanTween;

pub mod objpool;
pub mod shape;
mod particle;
pub use particle::*;
mod rand;
pub use rand::*;

/// Intelligently determines the project path based on command line arguments and environment variables.
///
/// This function provides a smart way to resolve the working directory for RustPixel applications
/// by checking multiple sources in order of priority:
///
/// # Return Value
/// Returns a `String` representing the project path, determined by the following logic:
///
/// 1. **No arguments**: If no command line arguments are provided, tries to use the 
///    `CARGO_MANIFEST_DIR` environment variable (set when running with `cargo run`).
///    Falls back to "." (current directory) if the environment variable is not available.
///
/// 2. **One argument**: Uses the first command line argument as the project path.
///    This is typically used when the path is explicitly provided by tools like `cargo-pixel`.
///
/// 3. **Multiple arguments**: Falls back to "." (current directory) when there are
///    multiple arguments, assuming the path specification is unclear.
///
/// # Examples
/// ```
/// // When run with: cargo run
/// // Returns: /path/to/project (from CARGO_MANIFEST_DIR)
///
/// // When run with: my_app /home/user/projects/game
/// // Returns: "/home/user/projects/game"
///
/// // When run with: my_app arg1 arg2 arg3
/// // Returns: "."
/// ```
///
/// # Use Cases
/// - Standalone executables that need to find their working directory
/// - Tools launched by `cargo-pixel` that receive the project path as an argument
/// - Development scenarios where `CARGO_MANIFEST_DIR` provides the correct path
pub fn get_project_path() -> String {
    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        1 => {
            if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
                manifest_dir.to_string()
            } else {
                ".".to_string()
            }
        }
        2 => {
            args[1].to_string()
        }
        _ => {
            args[1].to_string()
        }
    }
}

/// get flag_file path...
pub fn get_project_root(flag_file: &str) -> io::Result<PathBuf> {
    let path = env::current_dir()?;
    let path_ancestors = path.as_path().ancestors();

    for p in path_ancestors {
        let has_cargo = read_dir(p)?
            .any(|p| p.unwrap().file_name() == *flag_file);
        if has_cargo {
            return Ok(PathBuf::from(p));
        }
    }
    Err(io::Error::new(
        ErrorKind::NotFound,
        "Ran out of places to find flag_file",
    ))
}

/// Gets the absolute path of the root of RustPixel. In fact, it looks for where Cargo.lock locates
/// When deploying, users can run the CMD: 
/// cargo install --bin city --path games/city --root ~/PIXEL
/// and then put a Cargo.lock file and assets folder at bin/xxx
/// By doing this, binary executables can locate pixel_root_path and have access to resources under
/// the assets folder
pub fn get_pixel_root_path() -> String {
    match get_project_root("Cargo.lock") {
        Ok(p) => {
            let s = format!("{:?}", p);
            s[1..s.len() - 1].to_string()
        }
        Err(_e) => {
            ".".to_string()
        }
    }
}

pub fn get_abs_path(fpath: &str) -> String {
    if Path::new(fpath).is_relative() {
        format!("{}{}{}", get_pixel_root_path(), MAIN_SEPARATOR, fpath)
    } else {
        fpath.to_string()
    }
}

pub fn get_file_name(fpath: &str) -> String {
    Path::new(fpath)
        .file_name()
        .unwrap_or(&OsString::from(""))
        .to_str()
        .unwrap_or("")
        .to_string()
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Dir {
    Up,
    RightUp,
    Right,
    RightDown,
    Down,
    LeftDown,
    Left,
    LeftUp,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct PointI32 {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct PointU16 {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct PointI16 {
    pub x: i16,
    pub y: i16,
}

#[derive(CanTween, Debug, Clone, Copy, PartialEq, Default)]
pub struct PointF32 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct ARect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Rect {
        let max_area = u16::MAX;
        let (clipped_width, clipped_height) =
            if u32::from(width) * u32::from(height) > u32::from(max_area) {
                let aspect_ratio = f64::from(width) / f64::from(height);
                let max_area_f = f64::from(max_area);
                let height_f = (max_area_f / aspect_ratio).sqrt();
                let width_f = height_f * aspect_ratio;
                (width_f as u16, height_f as u16)
            } else {
                (width, height)
            };
        Rect {
            x,
            y,
            width: clipped_width,
            height: clipped_height,
        }
    }

    pub fn area(self) -> u16 {
        self.width * self.height
    }

    pub fn left(self) -> u16 {
        self.x
    }

    pub fn right(self) -> u16 {
        self.x.saturating_add(self.width)
    }

    pub fn top(self) -> u16 {
        self.y
    }

    pub fn bottom(self) -> u16 {
        self.y.saturating_add(self.height)
    }

    pub fn union(self, other: Rect) -> Rect {
        let x1 = min(self.x, other.x);
        let y1 = min(self.y, other.y);
        let x2 = max(self.x + self.width, other.x + other.width);
        let y2 = max(self.y + self.height, other.y + other.height);
        Rect {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }

    pub fn intersection(self, other: Rect) -> Rect {
        let x1 = max(self.x, other.x);
        let y1 = max(self.y, other.y);
        let x2 = min(self.x + self.width, other.x + other.width);
        let y2 = min(self.y + self.height, other.y + other.height);
        Rect {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }

    pub fn intersects(self, other: Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }
}
