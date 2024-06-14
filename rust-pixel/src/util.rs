// RustPixel
// copyright zhouxin@tuyoogame.com 2022~2024

//! Utils of random rect point...
//! and a simple object pool: objpool.rs
//! some primitive algorithm: shape.rs

use rand::seq::SliceRandom;
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro256StarStar,
};
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
    cmp::{max, min},
    env,
    ffi::OsString,
    fs::read_dir,
    io::{self, ErrorKind},
    path::{Path, PathBuf, MAIN_SEPARATOR},
};
#[cfg(target_arch = "wasm32")]
use web_sys::js_sys;
pub mod objpool;
pub mod shape;

/// 获取flag_file所在的路径
pub fn get_project_root(flag_file: &str) -> io::Result<PathBuf> {
    let path = env::current_dir()?;
    let mut path_ancestors = path.as_path().ancestors();

    while let Some(p) = path_ancestors.next() {
        let has_cargo = read_dir(p)?
            .into_iter()
            .any(|p| p.unwrap().file_name() == OsString::from(flag_file));
        if has_cargo {
            return Ok(PathBuf::from(p));
        }
    }
    Err(io::Error::new(
        ErrorKind::NotFound,
        "Ran out of places to find Cargo.toml",
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
            return s[1..s.len() - 1].to_string();
        }
        Err(_e) => {
            return ".".to_string();
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

/// RCG
pub struct Rand {
    rng: Xoshiro256StarStar,
}

impl Default for Rand {
    fn default() -> Self {
        Rand::new()
    }
}

/// 封装Xoshiro256**随机数生成器
impl Rand {
    pub fn new() -> Self {
        Self {
            rng: Xoshiro256StarStar::seed_from_u64(0),
        }
    }

    pub fn srand(&mut self, seed: u64) {
        self.rng = Xoshiro256StarStar::seed_from_u64(seed);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn srand_now(&mut self) {
        let seed: u64 = js_sys::Date::now() as u64;
        self.srand(seed);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn srand_now(&mut self) {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let seed = since_the_epoch.as_millis();
        self.srand(seed as u64);
    }

    pub fn rand(&mut self) -> u32 {
        let r = self.rng.next_u64() as u32;
        //info!("rand use xoshiro...{}", r);
        r
    }

    pub fn shuffle<T: Copy>(&mut self, v: &mut Vec<T>) {
        v.shuffle(&mut self.rng);
    }
}

/// 封装LCG随机数生成器, 随机效果不好
/// 但是在跟其他语言系统交互时，如果其他系统没有xoshiro实现
/// 为了获得一致的随机数序列可以采用
pub struct RandLCG {
    random_next: u64,
    count: u64,
}
impl RandLCG {
    pub fn new() -> Self {
        Self {
            random_next: 0u64,
            count: 0u64,
        }
    }

    pub fn srand(&mut self, seed: u64) {
        self.random_next = seed;
    }

    #[cfg(target_arch = "wasm32")]
    pub fn srand_now(&mut self) {
        let seed: u64 = js_sys::Date::now() as u64;
        self.srand(seed);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn srand_now(&mut self) {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let seed = since_the_epoch.as_secs();
        self.srand(seed);
    }

    pub fn rand(&mut self) -> u64 {
        let ret;
        if self.count % 2 == 0 {
            //MS LCG
            self.random_next = self.random_next.wrapping_mul(214013).wrapping_add(2531011);
            self.random_next &= 0x7FFFFFFF;
            ret = (self.random_next >> 16) & 0x7FFF
        } else {
            //BSD random LCG...
            self.random_next = self
                .random_next
                .wrapping_mul(1103515245)
                .wrapping_add(12345);
            self.random_next &= 0x7FFFFFFF;
            ret = self.random_next
        }
        self.count += 1;
        ret
    }

    pub fn shuffle<T: Copy>(&mut self, v: &mut Vec<T>) {
        let vl = v.len();
        if vl < 2 {
            return;
        }
        for i in 0..vl - 1 {
            v.swap(self.rand() as usize % vl, i);
        }
    }
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
pub struct APoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct ARect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub struct IPoint {
    pub x: i16,
    pub y: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FPoint {
    pub x: f32,
    pub y: f32,
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
        let max_area = u16::max_value();
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
