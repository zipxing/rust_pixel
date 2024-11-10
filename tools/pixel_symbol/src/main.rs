// https://github.com/JuliaPoo/AsciiArtist
// https://github.com/EgonOlsen71/petsciiator
use deltae::*;
use image::{DynamicImage, GenericImageView, ImageBuffer, Luma, Rgba};
use lab::Lab;
use rust_pixel::render::style::ANSI_COLOR_RGB;
use std::collections::HashMap;
use std::env;
use std::path::Path;

// gray 8x8 image...
type ImageNxN = Vec<Vec<u8>>;
struct RGB {
    r: u8,
    g: u8,
    b: u8,
}

fn main() {
    let input_image_path;
    let symsize: u32;
    let width: u32;
    let height: u32;
    let mut imap = HashMap::new();
    let mut cmap = HashMap::new();

    // parse command line...
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        println!("Usage: pixel_symbol <image file path> <symsize> <width> <height>");
        return;
    }
    input_image_path = Path::new(&args[1]);
    let img = image::open(&input_image_path).expect("Failed to open the input image");
    symsize = args[2].parse().unwrap();
    width = args[3].parse().unwrap();
    height = args[4].parse().unwrap();

    // make gray img...
    let gray_img = img.clone().into_luma8();
    gray_img.save("tmp/gray.png").unwrap();
    let (_, b2) = find_background_color(&img, &gray_img, width * symsize, height * symsize);

    // find block symbols...
    dig_symbols(&mut imap, &gray_img, width, height, symsize);

    // find block colors...
    for i in 0..height {
        for j in 0..width {
            let c = get_block_color(&img, &gray_img, symsize as usize, j, i, b2);
            cmap.entry(i * width + j).or_insert(c);
        }
    }

    // redraw gray image...
    let mut nimg = ImageBuffer::new(symsize * width, symsize * height);
    for (k, v) in imap.iter() {
        for b in v {
            let i = b % width;
            let j = b / width;
            let color = cmap.get(b).unwrap();
            for y in 0..symsize {
                for x in 0..symsize {
                    let pixel_value = if k[y as usize][x as usize] == 1 {
                        let ac = ANSI_COLOR_RGB[color.1 as usize];
                        [ac[0], ac[1], ac[2], 255]
                    } else {
                        let ac = ANSI_COLOR_RGB[color.0 as usize];
                        [ac[0], ac[1], ac[2], 255]
                    };
                    nimg.put_pixel(i * symsize + x, j * symsize + y, Rgba(pixel_value));
                }
            }
        }
    }
    nimg.save("bout.png").expect("save image error");
}

fn dig_symbols(
    imap: &mut HashMap<ImageNxN, Vec<u32>>,
    gray_img: &ImageBuffer<Luma<u8>, Vec<u8>>,
    width: u32,
    height: u32,
    symsize: u32,
) {
    // dig symbols...
    for i in 0..height {
        for j in 0..width {
            let block_at = get_block_at(&gray_img, symsize as usize, j, i);
            // key : ImageNxN, value : index vector
            imap.entry(block_at)
                .or_insert(Vec::new())
                .push(i * width + j);
        }
    }
    println!("imap len = {}", imap.len());
}

// find background colors...
fn find_background_color(
    img: &DynamicImage,
    image: &ImageBuffer<Luma<u8>, Vec<u8>>,
    w: u32,
    h: u32,
) -> (u8, u32) {
    // (first_x, first_y, count)
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
    let bx = cv[0].1 .0;
    let by = cv[0].1 .1;
    let bc = cv[0].0;
    // for c in cv {
    //     println!("cc..{:x} {:?}", c.0, c.1);
    // }
    // for i in 0..h {
    //     for j in 0..w {
    //         print!("{:?} ", image.get_pixel(j, i).0[0]);
    //     }
    //     println!("");
    // }
    let gray = image.get_pixel(bx, by).0[0];
    // println!("gray..{}", gray);
    (gray, *bc)
}

fn luminance(e1: u32) -> f32 {
    let e1r = (e1 >> 24 & 0xff) as u8;
    let e1g = (e1 >> 16 & 0xff) as u8;
    let e1b = (e1 >> 8 & 0xff) as u8;
    0.299 * e1r as f32 + 0.587 * e1g as f32 + 0.114 * e1b as f32
}

// get color distance
fn color_distance(e1: u32, e2: u32) -> f32 {
    let e1r = (e1 >> 24 & 0xff) as u8;
    let e1g = (e1 >> 16 & 0xff) as u8;
    let e1b = (e1 >> 8 & 0xff) as u8;
    let e2r = (e2 >> 24 & 0xff) as u8;
    let e2g = (e2 >> 16 & 0xff) as u8;
    let e2b = (e2 >> 8 & 0xff) as u8;

    let l1 = Lab::from_rgb(&[e1r, e1g, e1b]);
    let l2 = Lab::from_rgb(&[e2r, e2g, e2b]);
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

// get symbol block color
fn get_block_color(
    image: &DynamicImage,
    img: &ImageBuffer<Luma<u8>, Vec<u8>>,
    n: usize,
    x: u32,
    y: u32,
    back_rgb: u32,
) -> (usize, usize) {
    let mut cc: HashMap<u32, (u32, u32)> = HashMap::new();
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
            let g0 = img.get_pixel(cv[0].1 .0, cv[0].1 .1).0[0];
            let g1 = img.get_pixel(cv[1].1 .0, cv[1].1 .1).0[0];
            if g0 <= g1 {
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
                ret = Some((*ccv[1].0, *ccv[2].0));
            }
            // println!("ccv = {:?}", ccv);
        }
    }
    match ret {
        Some(r) => (find_best_color_u32(r.0), find_best_color_u32(r.1)),
        _ => (0, 0),
    }
}

fn get_block_at(image: &ImageBuffer<Luma<u8>, Vec<u8>>, n: usize, x: u32, y: u32) -> ImageNxN {
    let mut block = vec![vec![0u8; n]; n];
    let mut min = 255u8;
    let mut pm = HashMap::new();

    for i in 0..n {
        for j in 0..n {
            let pixel_x = x * n as u32 + j as u32;
            let pixel_y = y * n as u32 + i as u32;
            if pixel_x < image.width() && pixel_y < image.height() {
                block[i][j] = image.get_pixel(pixel_x, pixel_y).0[0];
                pm.insert(block[i][j], 1);
            }
        }
    }

    // if x == 2 && y == 22 {
    //     println!("{:?}", block);
    // }

    if pm.len() > 2 {
        let mut keys: Vec<u8> = pm.keys().cloned().collect();
        keys.sort();
        let mut nm = HashMap::new();
        let mut base: Option<u8> = None;
        for k in &keys {
            if base.is_none() {
                base = Some(*k);
            } else {
                if *k - base.unwrap() < 4 {
                    nm.insert(*k, base.unwrap());
                } else {
                    base = Some(*k);
                }
            }
        }
        for i in 0..n {
            for j in 0..n {
                let pixel_x = x * n as u32 + j as u32;
                let pixel_y = y * n as u32 + i as u32;
                if pixel_x < image.width() && pixel_y < image.height() {
                    block[i][j] = image.get_pixel(pixel_x, pixel_y).0[0];
                    if nm.contains_key(&block[i][j]) {
                        block[i][j] = nm[&block[i][j]]
                    }
                }
            }
        }
    }

    // if x == 2 && y == 22 {
    //     println!("{:?}", block);
    // }

    for i in 0..n {
        for j in 0..n {
            let pixel_x = x * n as u32 + j as u32;
            let pixel_y = y * n as u32 + i as u32;
            if pixel_x < image.width() && pixel_y < image.height() {
                if block[i][j] < min {
                    min = block[i][j];
                }
            }
        }
    }

    for i in 0..n {
        for j in 0..n {
            if block[i][j] == min {
                block[i][j] = 0;
            } else {
                block[i][j] = 1;
            }
        }
    }

    block
}

fn find_best_color_u32(c: u32) -> usize {
    find_best_color(RGB {
        r: (c >> 24) as u8,
        g: (c >> 16) as u8,
        b: (c >> 8) as u8,
    })
}

// get color distance
fn color_distance_rgb(e1: &RGB, e2: &RGB) -> f32 {
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

fn find_best_color(color: RGB) -> usize {
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
