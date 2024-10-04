use crate::monster::Monster;
use crate::{BH, BW};
use rust_pixel::util::{
    objpool::{GObj, GameObjPool},
    PointU16, Rand,
};

#[derive(Default)]
pub struct Tower {
    pub ttype: u8,
    pub pos: PointU16,
    pub range: i16,
    pub interval: i16,
    pub cd: i16,
    pub target: Option<usize>,
}

impl GObj for Tower {
    fn new() -> Tower {
        Default::default()
    }

    fn reset(&mut self, ttype: u8, ps: &Vec<u32>) {
        self.ttype = ttype;
        if ttype == 0 {
            self.range = 100;
            self.interval = 2;
        } else if ttype == 1 {
            self.range = 100;
            self.interval = 4;
        } else {
            // laser tower...
            self.range = 100;
            self.interval = 4;
        }
        self.cd = 0;
        self.pos = PointU16 {
            x: ps[0] as u16,
            y: ps[1] as u16,
        };
        self.target = None;
    }
}

impl Tower {
    pub fn set_in_grid(&self, grid: &mut [Vec<u8>]) {
        let x = self.pos.x as usize * BW;
        let y = self.pos.y as usize * BH;
        for i in 0..BW {
            for j in 0..BH {
                grid[y + i][x + j] = 2;
            }
        }
    }

    pub fn update(&mut self, ms: &mut GameObjPool<Monster>, ctx: &mut Rand) -> Vec<usize> {
        let mut vr: Vec<usize> = vec![];
        self.cd += 1;
        if self.cd > self.interval {
            self.cd = 0;
            if let Some(index) = self.target {
                if !ms.pool[index].active {
                    self.target = None;
                } else {
                    vr.push(index);
                }
            }
            if self.target.is_none() {
                let iv: Vec<_> = ms.pool.iter().filter(|m| m.active).collect();
                if !iv.is_empty() {
                    let tid = iv[ctx.rand() as usize % iv.len()].id;
                    self.target = Some(tid);
                    vr.push(tid);
                }
            }
        }
        vr
    }
}
