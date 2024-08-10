// RustPixel
// copyright zipxing@hotmail.com 2022~2024

#[derive(Debug, Clone)]
pub struct Select {
    pub area: usize,
    pub ranges: Vec<SelectRange>,
}

impl Select {
    pub fn new() -> Self {
        Self {
            area: 0,
            ranges: vec![],
        }
    }

    pub fn clear(&mut self) {
        self.area = 0;
        self.ranges.clear();
    }

    pub fn add_range(&mut self, r: SelectRange) {
        self.ranges.push(r);
    }

    pub fn cur(&mut self) -> &mut SelectRange {
        &mut self.ranges[self.area]
    }

    pub fn switch_area(&mut self) {
        if self.ranges.len() == 0 {
            return;
        }
        self.area = (self.area + 1) % self.ranges.len();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SelectRange {
    pub width: usize,
    pub height: usize,
    pub count: usize,
    pub x: usize,
    pub y: usize,
}

impl SelectRange {
    pub fn new(w: usize, h: usize, c: usize) -> Self {
        Self {
            width: w,
            height: h,
            count: c,
            x: 0,
            y: 0,
        }
    }

    pub fn forward_x(&mut self) {
        if self.width == 0 || self.count == 0 {
            return;
        }
        let count_last_row = self.count % self.width;
        if self.y == self.height - 1 && count_last_row != 0 {
            if self.x == count_last_row - 1 {
                self.x = 0;
            } else {
                self.x += 1;
            }
        } else {
            if self.x == self.width - 1 {
                self.x = 0;
            } else {
                self.x += 1;
            }
        }
    }

    pub fn backward_x(&mut self) {
        if self.width == 0 || self.count == 0 {
            return;
        }
        let count_last_row = self.count % self.width;
        if self.y == self.height - 1 && count_last_row != 0 {
            if self.x == 0 {
                self.x = count_last_row - 1;
            } else {
                self.x -= 1;
            }
        } else {
            if self.x == 0 {
                self.x = self.width - 1;
            } else {
                self.x -= 1;
            }
        }
    }

    pub fn forward_y(&mut self) {
        if self.height == 0 || self.count == 0 {
            return;
        }
        let count_last_col = self.height - 1;
        let modx = self.count % self.width;
        let mx = if modx == 0 { self.width } else { modx };
        if self.x >= mx {
            if self.y == count_last_col - 1 {
                self.y = 0;
            } else {
                self.y += 1;
            }
        } else {
            if self.y == self.height - 1 {
                self.y = 0;
            } else {
                self.y += 1;
            }
        }
    }

    pub fn backward_y(&mut self) {
        if self.height == 0 || self.count == 0 {
            return;
        }
        let count_last_col = self.height - 1;
        let modx = self.count % self.width;
        let mx = if modx == 0 { self.width } else { modx };
        if self.x >= mx {
            if self.y == 0 {
                self.y = count_last_col - 1;
            } else {
                self.y -= 1;
            }
        } else {
            if self.y == 0 {
                self.y = self.height - 1;
            } else {
                self.y -= 1;
            }
        }
    }
}


