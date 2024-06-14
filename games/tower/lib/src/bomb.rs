use rust_pixel::util::{objpool::GObj, FPoint, Point};

#[derive(Default)]
pub struct Bomb {
    pub btype: u8,
    pub pixel_pos: FPoint,
    pub stage: u8,
}

impl GObj for Bomb {
    fn new(btype: u8, ps: &Vec<Point>) -> Bomb {
        let mut b = Bomb {
            ..Default::default()
        };
        b.reset(btype, ps);
        b
    }

    fn reset(&mut self, btype: u8, ps: &Vec<Point>) {
        self.btype = btype;
        self.pixel_pos = FPoint {
            x: ps[0].x as f32,
            y: ps[0].y as f32,
        };
        self.stage = if btype == 0 {15} else {2};
    }
}

impl Bomb {
    pub fn update(&mut self) -> bool {
        if self.stage != 0 {
            self.stage -= 1;
            return true;
        } else {
            return false;
        }
    }
}
