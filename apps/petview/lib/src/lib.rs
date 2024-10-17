//
// implement core algorithm...
//

#![allow(dead_code)]
use rust_pixel::util::Rand;

pub struct PetviewData {
    pub rand: Rand,
    pub pool: Vec<u8>,
    pub index: usize,
}

impl PetviewData {
    pub fn new() -> Self {
        let mut rd = Rand::new();
        rd.srand_now();
        Self {
            rand: rd,
            pool: vec![],
            index: 0,
        }
    }

    pub fn shuffle(&mut self) {
        self.pool.clear();
        for i in 1..=52u8 {
            self.pool.push(i);
        }
        self.rand.shuffle(&mut self.pool);
        // println!("shuffle ok...");
    }

    pub fn next(&mut self) -> u8 {
        let ret;
        if self.pool.len() > 0 {
            ret = self.pool[self.index];
            self.index = (self.index + 1) % 52;
        } else {
            ret = 0;
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    #[test]
    fn it_works() {
        // let result = PetviewData::new();
    }
}
