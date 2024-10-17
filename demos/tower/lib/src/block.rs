use crate::{BH, BW};
use rust_pixel::util::{objpool::GObj, PointU16};

#[derive(Default)]
pub struct Block {
    pub btype: u8,
    pub pos: PointU16,
}

impl GObj for Block {
    fn new() -> Block {
        Default::default()
    }

    fn reset(&mut self, btype: u8, ps: &[u32]) {
        self.btype = btype;
        self.pos = PointU16 {
            x: ps[0] as u16,
            y: ps[1] as u16,
        };
    }
}

impl Block {
    pub fn set_in_grid(&self, grid: &mut [Vec<u8>]) {
        let x = self.pos.x as usize * BW;
        let y = self.pos.y as usize * BH;
        for i in 0..BW {
            for j in 0..BH {
                grid[y + i][x + j] = 1;
            }
        }
    }
}
