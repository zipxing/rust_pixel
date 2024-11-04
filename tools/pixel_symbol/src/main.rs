// https://github.com/JuliaPoo/AsciiArtist
// https://github.com/EgonOlsen71/petsciiator

// use image::{DynamicImage, GenericImageView, GrayImage, ImageBuffer, Luma};
use image::{GrayImage, ImageBuffer, Luma};
use std::collections::HashMap;
use std::env;
use std::path::Path;

// gray 8x8 image...
type ImageNxN = Vec<Vec<u8>>;

fn main() {
    let input_image_path;
    let width: u32;
    let height: u32;
    let mut imap = HashMap::new();

    let args: Vec<String> = env::args().collect();

    match args.len() {
        4 => {}
        _ => {
            println!("Usage: pixel_symbol <image file path> <width> <height>");
            return;
        }
    }
    input_image_path = Path::new(&args[1]);
    let img = image::open(&input_image_path).expect("Failed to open the input image");
    width = args[2].parse().unwrap();
    height = args[3].parse().unwrap();

    let gray_img = img.clone().into_luma8();
    gray_img.save("tmp/out2.png").unwrap();

    for i in 0..height {
        for j in 0..width {
            let block_at = get_block_at(&gray_img, 18, j, i);
            imap.entry(block_at)
                .or_insert(Vec::new())
                .push(i * width + j);
        }
    }

    let mut nimg = GrayImage::new(18 * width, 18 * height);
    for (k, v) in imap.iter() {
        for b in v {
            let i = b % width;
            let j = b / width;
            for y in 0..18 {
                for x in 0..18 {
                    let pixel_value = if k[y as usize][x as usize] == 1 {
                        255u8 
                    } else {
                        0u8 
                    };
                    nimg.put_pixel(i * 18 + x, j * 18 + y, Luma([pixel_value]));
                }
            }
        }
    }
    nimg.save("bout.png").expect("save image error");
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
                if block[i][j] < min {
                    min = block[i][j];
                }
            }
        }
    }

    if x == 2 && y == 22 {
        println!("{:?}", block);
    }
    if pm.len() > 2 {
        println!("...{:?}", pm);
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
