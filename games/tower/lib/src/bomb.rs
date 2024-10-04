use rust_pixel::util::{objpool::GObj, PointF32};

#[derive(Default)]
pub struct Bomb {
    pub btype: u8,
    pub pixel_pos: PointF32,
    pub stage: u8,
}

impl GObj for Bomb {
    fn new() -> Bomb {
        Default::default()
    }

    fn reset(&mut self, btype: u8, ps: &[u32]) {
        self.btype = btype;
        self.pixel_pos = PointF32 {
            x: ps[0] as f32,
            y: ps[1] as f32,
        };
        self.stage = if btype == 0 { 15 } else { 2 };
    }
}

impl Bomb {
    pub fn update(&mut self) -> bool {
        if self.stage != 0 {
            self.stage -= 1;
            true
        } else {
            false
        }
    }
}
