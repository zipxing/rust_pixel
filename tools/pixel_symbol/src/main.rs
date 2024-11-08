// https://github.com/JuliaPoo/AsciiArtist
// https://github.com/EgonOlsen71/petsciiator

use image::{DynamicImage, GenericImageView, GrayImage, ImageBuffer, Luma};
use std::collections::HashMap;
use std::env;
use std::path::Path;

// gray 8x8 image...
type ImageNxN = Vec<Vec<u8>>;

fn main() {
    let input_image_path;
    let symsize: u32;
    let width: u32;
    let height: u32;
    let mut imap = HashMap::new();

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
    let gray_img = img.clone().into_luma8();
    gray_img.save("tmp/gray.png").unwrap();

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

    let (b1, b2) = find_background_color(&img, &gray_img, width * symsize, height * symsize);
    println!("gray_back={} back={}", b1, b2);

    // redraw gray image...
    let mut nimg = GrayImage::new(symsize * width, symsize * height);
    for (k, v) in imap.iter() {
        for b in v {
            let i = b % width;
            let j = b / width;
            for y in 0..symsize {
                for x in 0..symsize {
                    let pixel_value = if k[y as usize][x as usize] == 1 {
                        255u8
                    } else {
                        0u8
                    };
                    nimg.put_pixel(i * symsize + x, j * symsize + y, Luma([pixel_value]));
                }
            }
        }
    }
    nimg.save("bout.png").expect("save image error");
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

    if x == 2 && y == 22 {
        println!("{:?}", block);
    }

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

    if x == 2 && y == 22 {
        println!("{:?}", block);
    }

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
