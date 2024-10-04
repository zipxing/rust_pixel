// RustPixel
// copyright zipxing@hotmail.com 2022~2024

use rand::seq::SliceRandom;
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro256StarStar,
};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

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

    pub fn rand64(&mut self) -> u64 {
        self.rng.next_u64()
    }

    pub fn rand(&mut self) -> u32 {
        self.rng.next_u64() as u32
    }

    pub fn gen_range(&mut self, min: f64, max: f64) -> f64 {
        if min > max {
            return 0.0;
        }
        let u1 = (min * 1000.0) as u64;
        let u2 = (max * 1000.0) as u64;
        (u1 + (self.rng.next_u64() % (u2 - u1 + 1))) as f64 / 1000.0
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
impl Default for RandLCG {
    fn default() -> Self {
        Self::new()
    }
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
        let ret = if self.count % 2 == 0 {
            //MS LCG
            self.random_next = self.random_next.wrapping_mul(214013).wrapping_add(2531011);
            self.random_next &= 0x7FFFFFFF;
            (self.random_next >> 16) & 0x7FFF
        } else {
            //BSD random LCG...
            self.random_next = self
                .random_next
                .wrapping_mul(1103515245)
                .wrapping_add(12345);
            self.random_next &= 0x7FFFFFFF;
            self.random_next
        };
        self.count += 1;
        ret
    }

    pub fn shuffle<T: Copy>(&mut self, v: &mut [T]) {
        let vl = v.len();
        if vl < 2 {
            return;
        }
        for i in 0..vl - 1 {
            v.swap(self.rand() as usize % vl, i);
        }
    }
}
