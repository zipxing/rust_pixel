// https://github.com/JuliaPoo/AsciiArtist
// https://github.com/EgonOlsen71/petsciiator

mod c64;
use c64::{C64LOW, C64UP};
use deltae::*;
use image::{DynamicImage, GenericImageView, ImageBuffer, Luma};
use lab::Lab;
use rust_pixel::render::style::ANSI_COLOR_RGB;
use std::collections::HashMap;
use std::env;
use std::path::Path;

// gray 8x8 image...
type Image8x8 = Vec<Vec<u8>>;
struct RGB {
    r: u8,
    g: u8,
    b: u8,
}

fn main() {
    let input_image_path;
    let mut width: u32 = 40;
    let mut height: u32 = 25;
    let mut is_petii: bool = false;
    let mut cx: u32 = u32::MAX;
    let mut cy: u32 = u32::MAX;
    let mut cw: u32 = u32::MAX;
    let mut ch: u32 = u32::MAX;

    let args: Vec<String> = env::args().collect();

    match args.len() {
        2 => {
            input_image_path = Path::new(&args[1]);
        }
        4 => {
            input_image_path = Path::new(&args[1]);
            width = args[2].parse().unwrap();
            height = args[3].parse().unwrap();
        }
        5 => {
            input_image_path = Path::new(&args[1]);
            width = args[2].parse().unwrap();
            height = args[3].parse().unwrap();
            is_petii = args[4].parse().unwrap();
        }
        9 => {
            input_image_path = Path::new(&args[1]);
            width = args[2].parse().unwrap();
            height = args[3].parse().unwrap();
            is_petii = args[4].parse().unwrap();
            cx = args[5].parse().unwrap();
            cy = args[6].parse().unwrap();
            cw = args[7].parse().unwrap();
            ch = args[8].parse().unwrap();
        }
        _ => {
            println!("Usage: tpetii <image file path> [<width>] [<height>] [<is_petscii>]");
            return;
        }
    }

    let mut img = image::open(&input_image_path).expect("Failed to open the input image");

    if cx != u32::MAX {
        img = img.crop(cx, cy, cw, ch);
        img.save("tmp/out0.png").unwrap();
    }
    let resized_img =
        img.resize_exact(width * 8, height * 8, image::imageops::FilterType::Lanczos3);
    resized_img.save("tmp/out1.png").unwrap();
    let gray_img = resized_img.clone().into_luma8();
    gray_img.save("tmp/out2.png").unwrap();

    // up petscii images...
    let vcs = gen_charset_images(false);
    // find background gray value...
    let bret = count_img_colors(&resized_img, &gray_img, width * 8, height * 8);
    let back = bret.0;
    let back_rgb = bret.1;
    println!("back...{} back_rgb...{:x}", back, back_rgb);


    // texture=255 表示每个点拥有自己的texture
    // 这种方式更灵活，但数据量会稍大
    println!("width={},height={},texture=255", width, height);
    for i in 0..height {
        for j in 0..width {
            if !is_petii {
                let block_at = get_block_at(&gray_img, j, i);
                let block_color = get_block_color(&resized_img, j, i);
                let bc = find_best_color(block_color);
                let bm = find_best_match(&block_at, &vcs, back, is_petii);
                // 每个点的texture设置为1
                print!("{},{},1 ", bm, bc,);
            } else {
                let block_at = get_block_at(&gray_img, j, i);
                // if j == 5 && i == 0 {
                //     println!("block50...{:?}", block_at);
                // }
                let bm = find_best_match(&block_at, &vcs, back, is_petii);
                let bc = get_petii_block_color(&resized_img, j, i, back_rgb);
                // 每个点的texture设置为1
                print!("{},{},1 ", bm, bc,);
            }
        }
        println!("");
    }
}

// get color distance
fn color_distance(e1: &RGB, e2: &RGB) -> f32 {
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

// generate 256 petscii image with 0 and 255
fn gen_charset_images(low_up: bool) -> Vec<Image8x8> {
    let data = if low_up { &C64LOW } else { &C64UP };
    let mut vcs = vec![vec![vec![0u8; 8]; 8]; 256];

    for i in 0..128 {
        for row in 0..8 {
            for bit in 0..8 {
                if data[i][row] >> bit & 1 == 1 {
                    vcs[i][row][7 - bit] = 255;
                    vcs[128 + i][row][7 - bit] = 0;
                } else {
                    vcs[i][row][7 - bit] = 0;
                    vcs[128 + i][row][7 - bit] = 255;
                }
            }
        }
    }
    vcs
}

// find background colors...
fn count_img_colors(
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

// get petscii block color
fn get_petii_block_color(image: &DynamicImage, x: u32, y: u32, back_rgb: u32) -> usize {
    let mut rgb = None;
    for i in 0..8usize {
        for j in 0..8usize {
            let pixel_x = x * 8 + j as u32;
            let pixel_y = y * 8 + i as u32;
            if pixel_x < image.width() && pixel_y < image.height() {
                let p = image.get_pixel(pixel_x, pixel_y);
                let k: u32 = ((p[0] as u32) << 24)
                    + ((p[1] as u32) << 16)
                    + ((p[2] as u32) << 8)
                    + (p[3] as u32);
                if k != back_rgb {
                    rgb = Some(p);
                    break;
                }
            }
        }
    }
    if rgb != None {
        let p = rgb.unwrap();
        find_best_color(RGB{r: p[0], g: p[1], b: p[2]})
    } else {
        0
    }
}

// get block average color(for not petscii image)
fn get_block_color(image: &DynamicImage, x: u32, y: u32) -> RGB {
    let mut r = 0u32;
    let mut g = 0u32;
    let mut b = 0u32;

    let mut count = 0u32;

    for i in 0..8usize {
        for j in 0..8usize {
            let pixel_x = x * 8 + j as u32;
            let pixel_y = y * 8 + i as u32;

            if pixel_x < image.width() && pixel_y < image.height() {
                let p = image.get_pixel(pixel_x, pixel_y);
                if p[0] != 0 || p[1] != 0 || p[2] != 0 {
                    r += p[0] as u32;
                    g += p[1] as u32;
                    b += p[2] as u32;
                    count += 1;
                }
            }
        }
    }

    if count == 0 {
        return RGB { r: 0, g: 0, b: 0 };
    }

    RGB {
        r: (r / count) as u8,
        g: (g / count) as u8,
        b: (b / count) as u8,
    }
}

fn get_block_at(image: &ImageBuffer<Luma<u8>, Vec<u8>>, x: u32, y: u32) -> Image8x8 {
    let mut block = vec![vec![0u8; 8]; 8];

    for i in 0..8usize {
        for j in 0..8usize {
            let pixel_x = x * 8 + j as u32;
            let pixel_y = y * 8 + i as u32;

            if pixel_x < image.width() && pixel_y < image.height() {
                block[i][j] = image.get_pixel(pixel_x, pixel_y).0[0];
            }
        }
    }

    block
}

fn find_best_match(
    input_image: &Image8x8,
    char_images: &[Image8x8],
    back: u8,
    is_petii: bool,
) -> usize {
    let mut min_mse = f64::MAX;
    let mut best_match = 0;

    for (i, char_image) in char_images.iter().enumerate() {
        let mse = calculate_mse(input_image, char_image, back, is_petii);
        // println!("i..{} mse..{}", i, mse);

        if mse < min_mse {
            min_mse = mse;
            best_match = i;
        }
    }

    best_match
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
        let mse = color_distance(&pcrgb, &color);

        if mse < min_mse {
            min_mse = mse;
            best_match = i;
        }
    }

    best_match
}

fn calc_eigenvector(img: &Image8x8, back: u8, is_petii: bool, is_source: bool) -> Vec<i32> {
    let mut v = vec![0i32; 10];
    // let mut min = u8::MAX;
    // let mut max = 0u8;
    // let mut include_back = false;

    // // find min & max gray value...
    // if is_petii {
    //     for x in 0..8 {
    //         for y in 0..8 {
    //             let p = img[y][x];
    //             if !include_back {
    //                 if p == back {
    //                     include_back = true;
    //                 }
    //             }
    //             if p > max {
    //                 max = p;
    //             }
    //             if p < min {
    //                 min = p;
    //             }
    //         }
    //     }
    // }

    for x in 0..8 {
        for y in 0..8 {
            let p;
            if is_petii {
                // 提取已有petscii art图片,首先进行二值化
                let iyx = img[y][x];
                if !is_source {
                    p = if iyx == back { 0i32 } else { 1i32 };
                } else {
                    p = if iyx == 0 { 0i32 } else { 1i32 };
                }
            } else {
                // 提取普通图片
                p = img[y][x] as i32;
            }

            if x < 4 && y < 4 {
                v[0] += p;
            }
            if x > 3 && y < 4 {
                v[1] += p;
            }
            if x < 4 && y > 3 {
                v[2] += p;
            }
            if x > 3 && y > 3 {
                v[3] += p;
            }
            if x > 2 && x < 6 && y > 2 && y < 6 {
                v[4] += p;
            }
            if x == y || x == (7 - y) {
                v[5] += p;
            }
            if x == 0 {
                v[6] += p;
            }
            if x == 7 {
                v[7] += p;
            }
            if y == 0 {
                v[8] += p;
            }
            if y == 7 {
                v[9] += p;
            }
        }
    }
    v
}

fn calculate_mse(img1: &Image8x8, img2: &Image8x8, back: u8, is_petii: bool) -> f64 {
    let mut mse = 0.0f64;
    let v1 = calc_eigenvector(img1, back, is_petii, false);
    let v2 = calc_eigenvector(img2, back, is_petii, true);
    // println!("input......{:?}", v1);
    // println!("petii......{:?}", v2);
    for i in 0..10usize {
        mse += ((v1[i] - v2[i]) * (v1[i] - v2[i])) as f64;
    }
    mse.sqrt()
}
