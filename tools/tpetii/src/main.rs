// https://github.com/JuliaPoo/AsciiArtist
// https://github.com/EgonOlsen71/petsciiator

use image::{DynamicImage, GenericImageView, ImageBuffer, Luma};
mod c64;
use c64::{C64LOW, C64UP};
use deltae::*;
use lab::Lab;
use std::env;
use std::path::Path;
use rust_pixel::render::style::ANSI_COLOR_RGB;

type Image = Vec<Vec<u8>>;
struct RGB {
    r: u8,
    g: u8,
    b: u8,
}

fn main() {
    let mut width: u32 = 40;
    let mut height: u32 = 25;
    let mut is_petii: bool = false;
    let input_image_path;
    let mut cx: u32; 
    let mut cy: u32;
    let mut cw: u32;
    let mut ch: u32;

    cx = u32::MAX;
    cy = u32::MAX;
    cw = u32::MAX;
    ch = u32::MAX;

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
            // threhold = args[9].parse().unwrap();
        }
        _ => {
            println!("Usage: tpetii <image file path> [<width>] [<height>] [<is_petscii>]");
            return;
        }
    }

    let mut img = image::open(&input_image_path).expect("Failed to open the input image");

    if cx != u32::MAX {
        img = img.crop(cx, cy, cw, ch);
        img.save("out0.png").unwrap();
    }

    let resized_img =
        img.resize_exact(width * 8, height * 8, image::imageops::FilterType::Lanczos3);
    //resized_img.save("out1.png").unwrap();
    let gray_img = resized_img.clone().into_luma8();
    //gray_img.save("out2.png").unwrap();

    // up petscii images...
    let vcs = gen_charset_images(false);

    // texture=255 表示每个点拥有自己的texture
    // 这种方式更灵活，但数据量会稍大
    println!("width={},height={},texture=255", width, height);
    for i in 0..height {
        for j in 0..width {
            let block_at = get_block_at(&gray_img, j, i);
            let block_color = get_block_color(&resized_img, j, i);
            let bc = find_best_color(block_color);
            let bm = find_best_match(&block_at, &vcs, is_petii);
            // 每个点的texture设置为1
            print!("{},{},1 ", bm, bc,);
        }
        println!("");
    }
}

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

fn gen_charset_images(low_up: bool) -> Vec<Image> {
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

fn get_block_at(image: &ImageBuffer<Luma<u8>, Vec<u8>>, x: u32, y: u32) -> Image {
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

fn find_best_match(input_image: &Image, char_images: &[Image], is_petii: bool) -> usize {
    let mut min_mse = f64::MAX;
    let mut best_match = 0;

    for (i, char_image) in char_images.iter().enumerate() {
        let mse = calculate_mse(input_image, char_image, is_petii);

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

fn calc_eigenvector(img: &Image, is_petii: bool) -> Vec<i32> {
    let mut v = vec![0i32; 10];

    for x in 0..8 {
        for y in 0..8 {
            let p;
            if is_petii {
                // 提取已有petscii art图片
                // p = if img[y][x] == 0 { 0i32 } else { 1i32 };
                p = if img[y][x] < 180 { 0i32 } else { 1i32 };
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

fn calculate_mse(img1: &Image, img2: &Image, is_petii: bool) -> f64 {
    let mut mse = 0.0f64;
    let v1 = calc_eigenvector(img1, is_petii);
    let v2 = calc_eigenvector(img2, is_petii);
    for i in 0..10usize {
        mse += ((v1[i] - v2[i]) * (v1[i] - v2[i])) as f64;
    }
    mse.sqrt()
}
