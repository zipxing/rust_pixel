// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//!
//! A symbol comprises a point vector with width * height elements
//!
//!

use deltae::*;
use image::{DynamicImage, GenericImageView};
use crate::render::style::ANSI_COLOR_RGB;
use lab::Lab;
use std::collections::HashMap;

struct RGB {
    r: u8,
    g: u8,
    b: u8,
}

// find big image background colors...
pub fn find_background_color(img: &DynamicImage, w: u32, h: u32) -> u32 {
    // color_u32 : (first_x, first_y, count)
    let mut cc: HashMap<u32, (u32, u32, u32)> = HashMap::new();
    for i in 0..h {
        for j in 0..w {
            let p = img.get_pixel(j, i);
            let k: u32 = ((p[0] as u32) << 24)
                + ((p[1] as u32) << 16)
                + ((p[2] as u32) << 8)
                + (p[3] as u32);
            (*cc.entry(k).or_insert((j, i, 0))).2 += 1;
        }
    }
    let mut cv: Vec<_> = cc.iter().collect();
    cv.sort_by(|b, a| (&a.1 .2).cmp(&b.1 .2));
    *cv[0].0
}

pub fn find_best_color(color: RGB) -> usize {
    let mut min_mse = f32::MAX;
    let mut best_match = 0;

    for (i, pcolor) in ANSI_COLOR_RGB.iter().enumerate() {
        let pcrgb = RGB {
            r: pcolor[0],
            g: pcolor[1],
            b: pcolor[2],
        };
        let mse = color_distance_rgb(&pcrgb, &color);

        if mse < min_mse {
            min_mse = mse;
            best_match = i;
        }
    }

    best_match
}

pub fn find_best_color_u32(c: u32) -> usize {
    find_best_color(RGB {
        r: (c >> 24) as u8,
        g: (c >> 16) as u8,
        b: (c >> 8) as u8,
    })
}

// get color distance
pub fn color_distance_rgb(e1: &RGB, e2: &RGB) -> f32 {
    let l1 = Lab::from_rgb(&[e1.r, e1.g, e1.b]);
    let l2 = Lab::from_rgb(&[e2.r, e2.g, e2.b]);
    let lab1 = LabValue {
        l: l1.l,
        a: l1.a,
        b: l1.b,
    };
    let lab2 = LabValue {
        l: l2.l,
        a: l2.a,
        b: l2.b,
    };
    *DeltaE::new(&lab1, &lab2, DE2000).value()
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub width: u8,
    pub height: u8,
    pub is_binary: bool,
    pub fore_color: u32,
    pub back_color: u32,
    pub data: Vec<Vec<u32>>,
    pub binary_data: Vec<Vec<u8>>,
}

impl Symbol {
    pub fn new(width: u8, height: u8, is_binary: bool, img: &DynamicImage) -> Self {
        let mut data = vec![];
        for i in 0..height {
            let mut row = vec![];
            for j in 0..width {
                let p = img.get_pixel(j as u32, i as u32);
                let k: u32 = ((p[0] as u32) << 24)
                    + ((p[1] as u32) << 16)
                    + ((p[2] as u32) << 8)
                    + (p[3] as u32);
                row.push(k);
            }
            data.push(row);
        }
        let binary_data = vec![];
        let mut sym = Self {
            width,
            height,
            is_binary,
            fore_color: 0,
            back_color: 0,
            data,
            binary_data,
        };
        sym.make_binary();
        sym
    }

    pub fn make_binary(&mut self) {
        let mut cc: HashMap<u32, (u32, u32)> = HashMap::new();
        let mut cm: Vec<u32> = vec![];
        let mut block = vec![vec![0u8; n]; n];
        for i in 0..n {
            for j in 0..n {
                let pixel_x = x * n as u32 + j as u32;
                let pixel_y = y * n as u32 + i as u32;
                if pixel_x < image.width() && pixel_y < image.height() {
                    let p = image.get_pixel(pixel_x, pixel_y);
                    let k: u32 = ((p[0] as u32) << 24)
                        + ((p[1] as u32) << 16)
                        + ((p[2] as u32) << 8)
                        + (p[3] as u32);
                    cc.entry(k).or_insert((pixel_x, pixel_y));
                    cm.push(k);
                }
            }
        }
        let mut cv: Vec<_> = cc.iter().collect();
        let mut include_back = false;
        let clen = cv.len();
        for c in &mut cv {
            if *c.0 == back_rgb {
                include_back = true;
            } else {
                let cd = color_distance(*c.0, back_rgb);
                // fix simliar color to back
                if cd < 1.0 {
                    // println!("cd={} c1={} c2={}", cd, *c.0, back_rgb);
                    (*c).0 = &back_rgb;
                    include_back = true;
                }
            }
        }
        let ret;
        if include_back {
            if clen == 1 {
                ret = Some((back_rgb, back_rgb));
                // println!("<B>{:?}", ret);
            } else if clen == 2 {
                let mut r = (back_rgb, back_rgb);
                if *cv[0].0 != back_rgb {
                    r.1 = *cv[0].0;
                }
                if *cv[1].0 != back_rgb {
                    r.1 = *cv[1].0;
                }
                ret = Some(r);
                // println!("<B,F>{:?}", ret);
            } else {
                // select bigest distance color to forecolor
                let mut bigd = 0.0f32;
                let mut bcv = cv[0];
                for c in &cv {
                    let cd = color_distance(*c.0, back_rgb);
                    if cd > bigd {
                        bigd = cd;
                        bcv = *c;
                    }
                }
                ret = Some((back_rgb, *bcv.0));
                // println!("ERROR!!! clen={} cv={:?}", clen, cv);
                // println!("bcv={:?}", bcv);
            }
        } else {
            if clen == 1 {
                ret = Some((*cv[0].0, *cv[0].0));
                // println!("<F>{:?}", ret);
            } else if clen == 2 {
                let l1 = luminance(*cv[0].0);
                let l2 = luminance(*cv[1].0);
                if l2 > l1 {
                    ret = Some((*cv[0].0, *cv[1].0));
                } else {
                    ret = Some((*cv[1].0, *cv[0].0));
                }
                // println!("<F1,F2>{:?}", ret);
            } else {
                let mut ccv = vec![];
                cv.sort();
                // println!("ERROR2!!! clen={} cv={:?}", clen, cv);
                let mut base = *cv[0].0;
                ccv.push(cv[0]);
                for i in 1..clen {
                    let cd = color_distance(*cv[i].0, base);
                    if cd > 1.0 {
                        ccv.push(cv[i]);
                    }
                    base = *cv[i].0;
                }
                let l1 = luminance(*ccv[0].0);
                let l2 = luminance(*ccv[1].0);
                if l2 > l1 {
                    ret = Some((*ccv[0].0, *ccv[1].0));
                } else {
                    ret = Some((*ccv[1].0, *ccv[0].0));
                }
                // println!("ccv = {:?}", ccv);
            }
        }

        for i in 0..n {
            for j in 0..n {
                let color = cm[i * n + j];
                let cd0 = color_distance(color, ret.unwrap().0);
                let cd1 = color_distance(color, ret.unwrap().1);
                if cd0 <= cd1 {
                    block[i][j] = 0;
                } else {
                    block[i][j] = 1;
                }
            }
        }

        match ret {
            Some(r) => (find_best_color_u32(r.0), find_best_color_u32(r.1), block),
            _ => (0, 0, block),
        }
    }
}
