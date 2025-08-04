// https://github.com/JuliaPoo/AsciiArtist
// https://github.com/EgonOlsen71/petsciiator
//
// REFACTORED VERSION:
// - Extracted binarization logic from calc_eigenvector to a dedicated binarize_block() function
// - Unified binarization processing: now performed once before character matching
// - Simplified calc_eigenvector: removed complex internal binarization logic
// - Improved code clarity and maintainability while preserving original functionality

mod c64;
use c64::{C64LOW, C64UP};
use deltae::*;
use image::{DynamicImage, GenericImageView, ImageBuffer, Luma};
use lab::Lab;
use rust_pixel::render::style::ANSI_COLOR_RGB;
use rust_pixel::render::symbols::find_background_color;
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

fn print_petii_usage() {
    eprintln!("RustPixel PETSCII Converter");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    petii <IMAGE_FILE> [WIDTH] [HEIGHT] [IS_PETSCII] [CROP_PARAMS...]");
    eprintln!("    cargo pixel petii <IMAGE_FILE> [WIDTH] [HEIGHT] [IS_PETSCII] [CROP_PARAMS...]");
    eprintln!("    cargo pixel p <IMAGE_FILE> [WIDTH] [HEIGHT] [IS_PETSCII] [CROP_PARAMS...]");
    eprintln!();
    eprintln!("ARGS:");
    eprintln!("    <IMAGE_FILE>   Input image file path");
    eprintln!("    [WIDTH]        Output width in characters (default: 40)");
    eprintln!("    [HEIGHT]       Output height in characters (default: 25)");
    eprintln!("    [IS_PETSCII]   Use PETSCII characters: true/false (default: false)");
    eprintln!("    [CROP_X]       Crop start X coordinate (requires all crop params)");
    eprintln!("    [CROP_Y]       Crop start Y coordinate");
    eprintln!("    [CROP_WIDTH]   Crop width");
    eprintln!("    [CROP_HEIGHT]  Crop height");
    eprintln!();
    eprintln!("DESCRIPTION:");
    eprintln!("    Converts images to PETSCII character art. Supports optional cropping");
    eprintln!("    and customizable output dimensions and character sets.");
    eprintln!();
    eprintln!("OUTPUT:");
    eprintln!("    Character art displayed in console/terminal");
    eprintln!("    Uses ASCII or PETSCII character set based on settings");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    petii image.png                              # Basic conversion");
    eprintln!("    petii image.png 80 50                        # Custom size 80x50");
    eprintln!("    petii image.png 40 25 true                   # Use PETSCII chars");
    eprintln!("    petii image.png 40 25 false 10 10 100 100    # With cropping");
    eprintln!();
    eprintln!("FEATURES:");
    eprintln!("    - Automatic image resizing and color mapping");
    eprintln!("    - Support for both ASCII and PETSCII character sets");
    eprintln!("    - Optional image cropping before conversion");
    eprintln!("    - Color similarity analysis using Delta E");
    eprintln!();
    eprintln!("NOTE:");
    eprintln!("    When used via cargo-pixel, equivalent to: cargo pixel r petii t -r <ARGS...>");
}

fn main() {
    let input_image_path;
    let mut width: u32 = 40;
    let mut height: u32 = 25;
    let mut is_petii: bool = false;

    let args: Vec<String> = env::args().collect();

    // Check for help argument
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h" || args[1] == "help") {
        print_petii_usage();
        return;
    }

    match args.len() {
        2 | 4 | 5 | 9 => {}
        _ => {
            print_petii_usage();
            return;
        }
    }
    input_image_path = Path::new(&args[1]);
    let mut img = image::open(&input_image_path).expect("Failed to open the input image");
    if args.len() > 2 {
        width = args[2].parse().unwrap();
        height = args[3].parse().unwrap();
    }
    if args.len() > 4 {
        is_petii = args[4].parse().unwrap();
    }

    if args.len() == 9 {
        let cx = args[5].parse().unwrap();
        let cy = args[6].parse().unwrap();
        let cw = args[7].parse().unwrap();
        let ch = args[8].parse().unwrap();
        img = img.crop(cx, cy, cw, ch);
        img.save("tmp/out0.png").expect("save tmp/out0.png error");
    }

    let resized_img =
        img.resize_exact(width * 8, height * 8, image::imageops::FilterType::Lanczos3);
    resized_img
        .save("tmp/out1.png")
        .expect("save tmp/out1.png error");
    let gray_img = resized_img.clone().into_luma8();
    gray_img
        .save("tmp/out2.png")
        .expect("save tmp/out2.png error");

    // get petscii images...
    let vcs = gen_charset_images(false);

    // find background color...
    let bret = find_background_color(&resized_img, &gray_img, width * 8, height * 8);
    let back_gray = bret.0;
    let back_rgb = bret.1;

    println!("width={},height={},texture=255", width, height);
    for i in 0..height {
        for j in 0..width {
            let block_at = get_block_at(&gray_img, j, i);

            // Apply binarization for PETSCII mode before character matching
            let processed_block = if is_petii {
                binarize_block(&block_at, back_gray)
            } else {
                block_at
            };

            let bm = find_best_match(&processed_block, &vcs);

            if !is_petii {
                let block_color = get_block_color(&resized_img, j, i);
                let bc = find_best_color(block_color);
                print!("{},{},1 ", bm, bc,);
            } else {
                let bc = get_petii_block_color(&resized_img, &gray_img, j, i, back_rgb);
                // sym, fg, tex, bg
                print!("{},{},1,{} ", bm, bc.1, bc.0);
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

/// Determines optimal foreground and background colors for a PETSCII character block
/// 
/// This function analyzes an 8x8 pixel block to determine the best foreground and background
/// color pair for PETSCII rendering. PETSCII characters can only display two colors per block,
/// so this function must intelligently choose which colors to use based on the content.
///
/// # Arguments
/// * `image` - The source color image (DynamicImage) 
/// * `img` - The corresponding grayscale version for luminance analysis
/// * `x` - Block column coordinate (multiply by 8 to get pixel coordinate)
/// * `y` - Block row coordinate (multiply by 8 to get pixel coordinate)  
/// * `back_rgb` - The pre-computed global background color as a packed RGBA u32
///
/// # Returns
/// A tuple `(background_color_index, foreground_color_index)` where each index
/// refers to the ANSI color palette used for rendering.
///
/// # Algorithm Overview
/// 1. **Color Collection**: Scans all 64 pixels in the 8x8 block and collects unique colors
/// 2. **Background Detection**: Checks if the global background color appears in this block
/// 3. **Color Assignment Logic**:
///    - If background color is present:
///      - 1 color total → Both fg/bg use background color (solid background block)
///      - 2 colors total → Background + one foreground color
///      - 3+ colors → Error condition (PETSCII can't represent this)
///    - If no background color:
///      - 1 color total → Both fg/bg use same color (solid foreground block) 
///      - 2 colors total → Use grayscale values to determine which is fg vs bg
///      - 3+ colors → Error condition
/// 4. **Color Mapping**: Converts the selected RGB colors to nearest ANSI palette indices
fn get_petii_block_color(
    image: &DynamicImage,
    img: &ImageBuffer<Luma<u8>, Vec<u8>>,
    x: u32,
    y: u32,
    back_rgb: u32,
) -> (usize, usize) {
    // HashMap to store unique colors found in this block
    // Key: packed RGBA color (R<<24 | G<<16 | B<<8 | A)
    // Value: first occurrence position (pixel_x, pixel_y) for potential grayscale lookup
    let mut cc: HashMap<u32, (u32, u32)> = HashMap::new();
    
    // Scan all 64 pixels in the 8x8 block
    for i in 0..8usize {
        for j in 0..8usize {
            let pixel_x = x * 8 + j as u32;
            let pixel_y = y * 8 + i as u32;
            
            // Ensure we don't go out of image bounds
            if pixel_x < image.width() && pixel_y < image.height() {
                let p = image.get_pixel(pixel_x, pixel_y);
                
                // Pack RGBA components into a single u32 for efficient comparison
                // Format: RGBA (R in highest byte, A in lowest byte)
                let k: u32 = ((p[0] as u32) << 24)    // Red
                    + ((p[1] as u32) << 16)            // Green  
                    + ((p[2] as u32) << 8)             // Blue
                    + (p[3] as u32);                   // Alpha
                
                // Store unique colors with their first occurrence position
                cc.entry(k).or_insert((pixel_x, pixel_y));
            }
        }
    }
    
    // Convert HashMap to Vec for easier iteration and indexing
    let cv: Vec<_> = cc.iter().collect();
    let mut include_back = false;
    let clen = cv.len();
    
    // Check if the global background color appears in this block
    for c in &cv {
        if *c.0 == back_rgb {
            include_back = true;
        }
    }
    
    let mut ret = None;
    
    if include_back {
        // Case 1: Block contains the global background color
        if clen == 1 {
            // Only background color present - solid background block
            ret = Some((back_rgb, back_rgb));
            // println!("<B>{:?}", ret);
        } else if clen == 2 {
            // Background + one other color - ideal PETSCII case
            let mut r = (back_rgb, back_rgb);
            
            // Find the non-background color to use as foreground
            if *cv[0].0 != back_rgb {
                r.1 = *cv[0].0;  // First color is foreground
            }
            if *cv[1].0 != back_rgb {
                r.1 = *cv[1].0;  // Second color is foreground  
            }
            ret = Some(r);
            // println!("<B,F>{:?}", ret);
        } else {
            // 3+ colors including background - cannot represent in PETSCII
            println!("ERROR!!!");
        }
    } else {
        // Case 2: Block does not contain global background color
        if clen == 1 {
            // Single color block - use same color for both fg and bg
            ret = Some((*cv[0].0, *cv[0].0));
            // println!("<F>{:?}", ret);
        } else if clen == 2 {
            // Two colors, neither is global background
            // Use grayscale values to determine which should be background vs foreground
            // Darker color typically becomes background in PETSCII convention
            
            let g0 = img.get_pixel(cv[0].1 .0, cv[0].1 .1).0[0];  // Grayscale of first color
            let g1 = img.get_pixel(cv[1].1 .0, cv[1].1 .1).0[0];  // Grayscale of second color
            
            if g0 <= g1 {
                // First color is darker, use as background
                ret = Some((*cv[0].0, *cv[1].0));
            } else {
                // Second color is darker, use as background
                ret = Some((*cv[1].0, *cv[0].0));
            }
            // println!("<F1,F2>{:?}", ret);
        } else {
            // 3+ colors with no global background - cannot represent in PETSCII
            println!("ERROR2!!!");
        }
    }
    
    // Convert the selected RGB colors to ANSI color palette indices
    match ret {
        Some(r) => (find_best_color_u32(r.0), find_best_color_u32(r.1)),
        _ => (0, 0),  // Fallback to black if something went wrong
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

/// Binarize a grayscale block for PETSCII processing
///
/// This function extracts the binarization logic from calc_eigenvector
/// to provide a unified binarization step before character matching.
fn binarize_block(img: &Image8x8, back: u8) -> Image8x8 {
    let mut binary_block = vec![vec![0u8; 8]; 8];
    let mut min = u8::MAX;
    let mut max = 0u8;
    let mut include_back = false;

    // Find min & max gray value and check for background color
    for x in 0..8 {
        for y in 0..8 {
            let p = img[y][x];
            if !include_back && p == back {
                include_back = true;
            }
            if p > max {
                max = p;
            }
            if p < min {
                min = p;
            }
        }
    }

    // Apply binarization logic
    for x in 0..8 {
        for y in 0..8 {
            let iyx = img[y][x];
            let binary_value = if include_back {
                // If block includes background color
                if iyx == back {
                    0
                } else {
                    255
                }
            } else {
                if min == max {
                    // If only 1 color
                    255
                } else {
                    // Min to 0 and max to 255
                    if iyx == min {
                        0
                    } else {
                        255
                    }
                }
            };
            binary_block[y][x] = binary_value;
        }
    }

    binary_block
}

fn find_best_match(input_image: &Image8x8, char_images: &[Image8x8]) -> usize {
    let mut min_mse = f64::MAX;
    let mut best_match = 0;

    for (i, char_image) in char_images.iter().enumerate() {
        let mse = calculate_mse(input_image, char_image);
        // println!("i..{} mse..{}", i, mse);

        if mse < min_mse {
            min_mse = mse;
            best_match = i;
        }
    }

    best_match
}

fn find_best_color_u32(c: u32) -> usize {
    find_best_color(RGB {
        r: (c >> 24) as u8,
        g: (c >> 16) as u8,
        b: (c >> 8) as u8,
    })
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

fn calc_eigenvector(img: &Image8x8) -> Vec<i32> {
    let mut v = vec![0i32; 10];

    for x in 0..8 {
        for y in 0..8 {
            let p = img[y][x] as i32;

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

fn calculate_mse(img1: &Image8x8, img2: &Image8x8) -> f64 {
    let mut mse = 0.0f64;
    let v1 = calc_eigenvector(img1);
    let v2 = calc_eigenvector(img2);
    // println!("input......{:?}", v1);
    // println!("petii......{:?}", v2);
    for i in 0..10usize {
        mse += ((v1[i] - v2[i]) * (v1[i] - v2[i])) as f64;
    }
    mse.sqrt()
}
