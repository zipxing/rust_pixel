use crate::bomb::Bomb;
// use crate::model::{BH, BW};
use crate::monster::Monster;
use rust_pixel::util::{
    objpool::{GObj, GameObjPool},
    PointU16,
};
// use log::info;

#[derive(Default)]
pub struct Laser {
    pub btype: u8,
    pub damage: i32,
    pub src_pos: PointU16,
    pub dst_pos: PointU16,
    // pub pixel_pos: PointU16,
    pub csize: PointU16,
    pub target_monster: usize,
    pub stage: u8,
}

impl GObj for Laser {
    fn new(btype: u8, ps: &Vec<PointU16>) -> Laser {
        let mut bt = Laser {
            ..Default::default()
        };
        bt.reset(btype, ps);
        bt
    }

    fn reset(&mut self, btype: u8, ps: &Vec<PointU16>) {
        self.btype = btype;
        self.damage = 25;

        // cell size in pixel...
        self.csize = ps[0];
        // source pos (tower pos)...
        self.src_pos = ps[1];
        // dst pos ( monster pos )
        self.dst_pos = PointU16 {
            x: ps[2].x / self.csize.x,
            y: ps[2].y / self.csize.y,
        };
        self.target_monster = ps[3].x as usize;
        self.stage = 6;
    }
}

impl Laser {
    pub fn update(&mut self, 
        bs: &mut GameObjPool<Bomb>,
        ms: &mut GameObjPool<Monster>,
    ) -> bool {
        let m = &mut ms.pool[self.target_monster];
        if !m.active {
            self.stage = 0;
            return false;
        }
        if self.stage != 0 {
            self.dst_pos = PointU16 {
                x: m.obj.pos.x,
                y: m.obj.pos.y,
            };
            // self.pixel_pos = PointU16 {
            //     x: m.obj.pixel_pos.x as u16 % self.csize.x,
            //     y: m.obj.pixel_pos.y as u16 % self.csize.y,
            // };
            self.stage -= 1;
            return true;
        } else {
            m.obj.life -= self.damage;
            let bpt = PointU16 {
                x: m.obj.pixel_pos.x as u16,
                y: m.obj.pixel_pos.y as u16,
            };
            if m.obj.life < 0 {
                bs.create(0, &vec![bpt]);
                m.active = false;
            } else {
                // let nbpt = PointU16 {
                //     x: ((bpt.x as f32 + x) / 2.0) as u16, 
                //     y: ((bpt.y as f32 + y) / 2.0) as u16,
                // };
                // bs.create(1, &vec![nbpt]);
            }
            return false;
        }
    }
}
