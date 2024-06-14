use crate::{BH, BW};
use rust_pixel::util::{objpool::GObj, Point};

#[derive(Default)]
pub struct Block {
    pub btype: u8,
    pub pos: Point,
}

impl GObj for Block {
    fn new(btype: u8, ps: &Vec<Point>) -> Block {
        let mut b = Block {
            ..Default::default()
        };
        b.reset(btype, ps);
        b
    }

    fn reset(&mut self, btype: u8, ps: &Vec<Point>) {
        self.btype = btype;
        self.pos = ps[0];
    }
}

impl Block {
    pub fn set_in_grid(&self, grid: &mut Vec<Vec<u8>>) {
        let x = self.pos.x as usize * BW;
        let y = self.pos.y as usize * BH;
        for i in 0..BW {
            for j in 0..BH {
                grid[y + i][x + j] = 1;
            }
        }
    }
}
