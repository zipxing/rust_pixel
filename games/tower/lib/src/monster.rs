use crate::{check_passable, TOWERH, TOWERW};
// use log::info;
use std::collections::{HashMap, HashSet};
use rust_pixel::{
    algorithm::astar::{a_star, PointUsize},
    util::{objpool::GObj, PointF32, PointU16, Rand},
};

#[derive(Default)]
pub struct Monster {
    pub mtype: u8,
    pub life: i32,
    pub max_life: i32,
    pub speed: i16,
    pub fspeed: PointF32,
    pub pos: PointU16,
    pub next_pos: PointU16,
    pub pixel_pos: PointF32,
    pub interval: i16,
    pub cd: i16,
    pub path: Vec<PointUsize>,
}

impl GObj for Monster {
    fn new() -> Monster {
        Default::default()
    }

    fn reset(&mut self, mtype: u8, ps: &Vec<u32>) {
        self.mtype = mtype;
        if mtype == 1 {
            self.life = 5800;
            self.speed = 2;
        } else {
            self.life = 500;
            self.speed = 3;
        }
        self.max_life = self.life;
        self.fspeed = PointF32 { x: 0.0, y: 0.0 };
        self.pos = PointU16 { x: 0, y: 0 };
        self.next_pos = PointU16 { x: 0, y: 0 };
        self.pixel_pos = PointF32 {
            x: ps[0] as f32,
            y: ps[1] as f32,
        };
        self.interval = 1;
        self.cd = 0;
        self.path.clear();
    }
}

impl Monster {
    pub fn find_path<P>(&mut self, grids: &mut Vec<Vec<u8>>, start_p: P)
    where
        P: Into<PointUsize>,
    {
        self.path = a_star(grids, start_p.into(), (TOWERH - 1, TOWERW - 1), |v| {
            check_passable(v)
        })
        .unwrap();
    }

    pub fn get_next_pos(&mut self, grids: &mut Vec<Vec<u8>>, rand: &mut Rand) {
        if self.path.is_empty() || rand.rand() % 10 == 0 {
            self.find_path(grids, self.pos);
        }
        let mut ng = self.path.remove(1);
        if check_passable(grids[ng.0][ng.1]) {
            self.next_pos = PointU16 {
                x: ng.1 as u16,
                y: ng.0 as u16,
            };
        } else {
            // 如果不通，重新寻找path
            self.find_path(grids, self.pos);
            ng = self.path.remove(1);
            self.next_pos = PointU16 {
                x: ng.1 as u16,
                y: ng.0 as u16,
            };
        }
        let dy = self.next_pos.y as f32 - self.pos.y as f32;
        let dx = self.next_pos.x as f32 - self.pos.x as f32;
        let angle = dy.atan2(dx);
        self.fspeed = PointF32 {
            x: self.speed as f32 * angle.cos(),
            y: self.speed as f32 * angle.sin(),
        };
    }

    pub fn arrive(&self, w: f32, h: f32) -> bool {
        // let w = ctx.adapter.cell_width();
        // let h = ctx.adapter.cell_height();
        let dx = (self.next_pos.x + 1) as f32 * w - self.pixel_pos.x;
        let dy = (self.next_pos.y + 1) as f32 * h - self.pixel_pos.y;
        let distance = (dx * dx + dy * dy).sqrt();
        //info!("arrive....distance={}", distance);
        distance < self.speed as f32
    }

    pub fn domove(&mut self) {
        self.pixel_pos.x += self.fspeed.x;
        self.pixel_pos.y += self.fspeed.y;
    }

    fn gid(&self) -> usize {
        self.pos.y as usize * TOWERW + self.pos.x as usize
    }

    fn ngid(&self) -> usize {
        self.next_pos.y as usize * TOWERW + self.next_pos.x as usize
    }

    pub fn update(
        &mut self,
        mid: usize,
        grids: &mut Vec<Vec<u8>>,
        mmap: &mut HashMap<usize, HashSet<usize>>,
        w: f32,
        h: f32,
        ctx: &mut Rand,
    ) -> bool {
        self.cd += 1;
        if self.cd > self.interval {
            self.cd = 0;
        } else {
            return true;
        }
        if self.arrive(w, h) {
            // 从老的格子删除monster id,新的格子添加monster id
            mmap.entry(self.gid()).and_modify(|s| {
                s.remove(&mid);
            });
            mmap.entry(self.ngid())
                .or_default()
                .insert(mid);

            self.pos = self.next_pos;

            // 判断逃逸...
            // judge running away
            if self.pos.x as usize == TOWERW - 1 && self.pos.y as usize == TOWERH - 1 {
                return false;
            }
            self.get_next_pos(grids, ctx);
        } else {
            self.domove();
        }
        true
    }
}
