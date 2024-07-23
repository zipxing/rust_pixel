// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Implements some shape drawing algorithms
//!
//! lightning implements drawing of lightnings
//! line implements drawing of lines
//! circle implements drawing of circles

use rand;

pub fn circle(x0: u16, y0: u16, radius: u16) -> Vec<(i16, i16)> {
    let mut points = Vec::new();
    let mut x: i16 = 0;
    let mut y: i16 = radius as i16;
    let mut d: i16 = 3 - 2 * radius as i16;

    while x <= y {
        // Each quadrant's PointU16s are symmetric, so we can add them all at once
        points.push((x0 as i16 + x, y0 as i16 + y));
        points.push((x0 as i16 - x, y0 as i16 + y));
        points.push((x0 as i16 + x, y0 as i16 - y));
        points.push((x0 as i16 - x, y0 as i16 - y));
        points.push((x0 as i16 + y, y0 as i16 + x));
        points.push((x0 as i16 - y, y0 as i16 + x));
        points.push((x0 as i16 + y, y0 as i16 - x));
        points.push((x0 as i16 - y, y0 as i16 - x));
        x += 1;
        if d > 0 {
            y -= 1;
            d = d + 4 * (x - y) + 10;
        } else {
            d = d + 4 * x + 6;
        }
    }
    points
}

fn reverse_bresenham_next_point(x0: i16, y0: i16, x1: i16, y1: i16) -> (i16, i16) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { -1i16 } else { 1i16 };
    let sy = if y0 < y1 { -1i16 } else { 1i16 };
    let err = if dx > dy { dx } else { -dy } / 2;
    let mut x = x0;
    let mut y = y0;
    if err > -dx {
        x += sx;
    }
    if err < dy {
        y += sy;
    }
    (x, y)
}

// draws a line using | - \ / chars, needs to adjust the start and end PointU16 of the line
pub fn prepare_line(px0: u16, py0: u16, px1: u16, py1: u16) -> (i16, i16, i16, i16) {
    let fdy = py1 as f32 - py0 as f32;
    let fdx = px1 as f32 - px0 as f32;
    let mut angle = fdy.atan2(fdx);
    if angle < 0.0 {
        angle = angle + std::f32::consts::PI * 2.0;
    }
    angle = angle / std::f32::consts::PI;
    let (mut x0, mut y0, x1, y1);
    if (angle > 0.0 && angle < 0.5)
        || (angle > 0.75 && angle < 1.0)
        || (angle > 1.5 && angle < 1.75)
    {
        x0 = px1 as i16;
        y0 = py1 as i16;
        x1 = px0 as i16;
        y1 = py0 as i16;
    } else {
        x0 = px0 as i16;
        y0 = py0 as i16;
        x1 = px1 as i16;
        y1 = py1 as i16;
    }

    let np = reverse_bresenham_next_point(x0, y0, x1, y1);
    x0 = np.0;
    y0 = np.1;

    (x0, y0, x1, y1)
}

#[repr(u8)]
pub enum LineSym {
    START = 0,
    END = 1,
    VLINE = 2,
    HLINE = 3,
    SLASH = 4,
    BACKSLASH = 5,
}

pub fn line(x0: i16, y0: i16, x1: i16, y1: i16) -> Vec<(i16, i16, LineSym)> {
    let mut res: Vec<(i16, i16, LineSym)> = Vec::new();

    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };

    let mut err = if dx > dy { dx } else { -dy } / 2;
    let mut err2: i16;

    let mut x = x0;
    let mut y = y0;

    let mut flag: u8 = 0;

    loop {
        let sym_auto = match flag {
            0 => LineSym::START,
            1 => LineSym::VLINE,
            2 => LineSym::HLINE,
            3 => {
                if (sx < 0 && sy < 0) || (sx > 0 && sy > 0) {
                    LineSym::SLASH
                } else {
                    LineSym::BACKSLASH
                }
            }
            _ => LineSym::END,
        };
        // if x == x1 as i16 && y == y1 as i16 {
        //     sym_auto = sdlsym(syms.1);
        // }
        res.push((x, y, sym_auto));

        if x == x1 && y == y1 {
            break;
        }

        err2 = err;
        flag = 0;

        if err2 > -dx {
            err -= dy;
            x += sx;
            flag += 1;
        }

        if err2 < dy {
            err += dx;
            y += sy;
            flag += 2;
        }
    }
    res
}

struct LineSegment {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    displace: f32,
}

impl LineSegment {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32, displace: f32) -> LineSegment {
        LineSegment {
            x1,
            y1,
            x2,
            y2,
            displace,
        }
    }
}

pub fn lightning(
    x0: u16,
    y0: u16,
    x1: u16,
    y1: u16,
    displace: u16,
    cur_detail: u16,
) -> Vec<(u16, u16, u16, u16)> {
    let line = LineSegment::new(x0 as f32, y0 as f32, x1 as f32, y1 as f32, displace as f32);
    let mut stack = vec![line];
    let mut result = vec![];

    while let Some(cur) = stack.pop() {
        if cur.displace < cur_detail as f32 {
            result.push((cur.x1 as u16, cur.y1 as u16, cur.x2 as u16, cur.y2 as u16));
        } else {
            let mid_x = (cur.x2 + cur.x1) / 2.0;
            let mid_y = (cur.y2 + cur.y1) / 2.0;
            let mid_x = mid_x + (rand::random::<f32>() - 0.5) * cur.displace;
            let mid_y = mid_y + (rand::random::<f32>() - 0.5) * cur.displace;

            stack.push(LineSegment::new(
                cur.x1,
                cur.y1,
                mid_x,
                mid_y,
                cur.displace / 2.0,
            ));
            stack.push(LineSegment::new(
                cur.x2,
                cur.y2,
                mid_x,
                mid_y,
                cur.displace / 2.0,
            ));
        }
    }
    result
}
