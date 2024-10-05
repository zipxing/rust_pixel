use crate::bomb::Bomb;
use crate::monster::Monster;
use crate::{BH, BW, TOWERH, TOWERW};
// use log::info;
use rust_pixel::util::{
    objpool::{GObj, GameObjPool},
    PointF32, PointU16,
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct Bullet {
    pub btype: u8,
    pub speed: i16,
    pub damage: i32,
    pub src_pos: PointU16,
    pub dst_pos: PointU16,
    pub fspeed: PointF32,
    pub pixel_pos: PointF32,
    pub csize: PointU16,
    pub angle: f32,
}

impl GObj for Bullet {
    fn new() -> Bullet {
        Default::default()
    }

    fn reset(&mut self, btype: u8, ps: &[u32]) {
        self.btype = btype;
        if btype == 0 {
            self.speed = 45;
            self.damage = 8;
        } else {
            self.speed = 25;
            self.damage = 3;
        }

        // cell size in pixel...
        self.csize = PointU16 {
            x: ps[0] as u16,
            y: ps[1] as u16,
        };
        // source pos (tower pos)...
        self.src_pos = PointU16 {
            x: ps[2] as u16,
            y: ps[3] as u16,
        };
        // dst pos ( monster pos )
        self.dst_pos = PointU16 {
            x: ps[4] as u16,
            y: ps[5] as u16,
        };

        // tower size...
        let w = ps[0] as f32 * BW as f32;
        let h = ps[1] as f32 * BH as f32;

        // tower center...
        self.pixel_pos = PointF32 {
            x: (self.src_pos.x as f32 + 0.66) * w,
            y: (self.src_pos.y as f32 + 0.66) * h,
        };

        // info!("bullet reset...src{:?}..dst{:?}", self.pixel_pos, self.dst_pos);

        // angle and speed x y...
        let dy = self.dst_pos.y as f32 - self.pixel_pos.y;
        let dx = self.dst_pos.x as f32 - self.pixel_pos.x;
        let angle = dy.atan2(dx);
        // info!("bullet reset...src{:?}..dst{:?}..angle{:?}", self.pixel_pos, self.dst_pos, angle);
        self.angle = angle;
        self.fspeed = PointF32 {
            x: self.speed as f32 * angle.cos(),
            y: self.speed as f32 * angle.sin(),
        };
    }
}

impl Bullet {
    pub fn domove(&mut self) {
        self.pixel_pos.x += self.fspeed.x;
        self.pixel_pos.y += self.fspeed.y;
    }

    pub fn update(
        &mut self,
        bs: &mut GameObjPool<Bomb>,
        ms: &mut GameObjPool<Monster>,
        mmap: &HashMap<usize, HashSet<usize>>,
    ) -> bool {
        self.domove();
        let x = self.pixel_pos.x;
        let y = self.pixel_pos.y;
        if !(x <= (TOWERW * self.csize.x as usize) as f32
            && x >= 0.0
            && y <= (TOWERH * self.csize.y as usize) as f32
            && y >= 0.0)
        {
            return false;
        }
        let ix = (x / self.csize.x as f32) as usize;
        let iy = (y / self.csize.y as f32) as usize;
        let gid = (iy * TOWERW + ix) as i32;
        let tw = TOWERW as i32;
        let off: [i32; 9] = [0, -tw - 1, -tw, -tw + 1, -1, 1, tw - 1, tw, tw + 1];
        for i in off.iter() {
            let ggid = gid + i;
            if ggid < 0 || ggid >= (TOWERH * TOWERW) as i32 {
                continue;
            }
            if let Some(ids) = mmap.get(&(ggid as usize)) {
                for id in ids {
                    let m = &mut ms.pool[*id];
                    if !m.active {
                        continue;
                    }
                    let dx = m.obj.pixel_pos.x - x;
                    let dy = m.obj.pixel_pos.y - y;
                    let distance = (dx * dx + dy * dy).sqrt();
                    if distance < self.csize.x as f32 * 1.2 {
                        let bpt = (
                            m.obj.pixel_pos.x as u32,
                            m.obj.pixel_pos.y as u32,
                        );
                        m.obj.life -= self.damage;
                        if m.obj.life < 0 {
                            bs.create(0, &[bpt.0, bpt.1]);
                            m.active = false;
                        } else {
                            let nbpt = (
                                ((bpt.0 as f32 + x) / 2.0) as u32,
                                ((bpt.1 as f32 + y) / 2.0) as u32,
                            );
                            bs.create(1, &[nbpt.0, nbpt.1]);
                        }
                        return false;
                    }
                }
            }
        }
        true
    }
}
