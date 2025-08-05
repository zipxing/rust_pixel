// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! PETSCII Converter Tool
//!
//! This tool converts images to ASCII/PETSCII format using character pattern matching.
//! It supports configurable block dimensions and provides high-quality character matching
//! using feature-based algorithms rather than simple pixel comparison.

use rust_pixel::render::symbols::{
    find_background_color, 
    gen_charset_images, 
    get_grayscale_block_at, 
    binarize_grayscale_block, 
    find_best_match, 
    get_petii_block_color, 
    get_block_color,
    find_best_color
};
mod c64;
use c64::{C64LOW, C64UP};
use std::env;
use std::path::Path;

/// Displays comprehensive usage information for the PETSCII converter tool
/// 
/// This function prints detailed help text including:
/// - Command syntax and available arguments
/// - Parameter descriptions and default values
/// - Usage examples for different scenarios
/// - Feature descriptions and technical notes
/// 
/// Called when user requests help with --help, -h, or help arguments,
/// or when invalid arguments are provided
fn print_petii_usage() {
    println!("PETSCII Converter Tool v2.0");
    println!("Converts images to ASCII/PETSCII format with configurable block dimensions");
    println!();
    println!("USAGE:");
    println!("    petii <IMAGE_FILE> [WIDTH] [HEIGHT] [IS_PETSCII] [CROP_X] [CROP_Y] [CROP_WIDTH] [CROP_HEIGHT]");
    println!();
    println!("ARGUMENTS:");
    println!("    IMAGE_FILE      Path to input image file (required)");
    println!("    WIDTH           Output width in characters (default: 40)");
    println!("    HEIGHT          Output height in characters (default: 25)");
    println!("    IS_PETSCII      Use PETSCII character set: 0=false, 1=true (default: 0)");
    println!("    CROP_X          Crop starting X position in pixels (optional)");
    println!("    CROP_Y          Crop starting Y position in pixels (optional)");
    println!("    CROP_WIDTH      Crop width in pixels (optional)");
    println!("    CROP_HEIGHT     Crop height in pixels (optional)");
    println!();
    println!("EXAMPLES:");
    println!("    petii image.jpg                     # Convert with defaults (40x25, ASCII)");
    println!("    petii image.jpg 80 50               # Convert to 80x50 characters");
    println!("    petii image.jpg 40 25 1             # Use PETSCII character set");
    println!("    petii image.jpg 40 25 0 100 100 400 300  # Crop and convert");
    println!();
    println!("OUTPUT FORMAT:");
    println!("    ASCII mode:   symbol,color,texture");
    println!("    PETSCII mode: symbol,foreground,texture,background");
    println!();
    println!("    First line contains metadata: width=W,height=H,texture=255");
    println!("    Following lines contain character data in row-major order");
    println!();
    println!("FEATURES:");
    println!("    - Feature-based character matching for superior quality");
    println!("    - Delta E 2000 color matching for perceptual accuracy");
    println!("    - Configurable block dimensions (currently 8x8)");
    println!("    - Support for both ASCII and PETSCII character sets");
    println!("    - Optional image cropping before conversion");
    println!("    - Automatic background color detection for PETSCII mode");
    println!();
    println!("TECHNICAL NOTES:");
    println!("    - Images are resized to WIDTH*8 x HEIGHT*8 pixels before processing");
    println!("    - Each character block represents an 8x8 pixel area");
    println!("    - PETSCII mode supports 2 colors per character (foreground/background)");
    println!("    - ASCII mode uses single color per character with transparency");
    println!("    - Color indices refer to the standard 256-color ANSI palette");
}

/// Main entry point for the PETSCII converter tool
/// 
/// This function handles the complete image-to-ASCII/PETSCII conversion pipeline:
/// 
/// # Process Flow:
/// 1. **Argument parsing**: Validates command line arguments for image path, dimensions, and options
/// 2. **Image loading**: Opens and optionally crops the input image
/// 3. **Image preprocessing**: Resizes image to match target character grid (8px per character)
/// 4. **Background detection**: Analyzes image to determine background color for PETSCII mode
/// 5. **Character generation**: Creates character set images for matching
/// 6. **Block processing**: Converts each 8x8 pixel block to the best matching character
/// 7. **Output generation**: Prints formatted character data with color information
/// 
/// # Arguments:
/// - `IMAGE_FILE`: Path to input image file (required)
/// - `WIDTH`: Output width in characters (default: 40)
/// - `HEIGHT`: Output height in characters (default: 25)
/// - `IS_PETSCII`: Whether to use PETSCII character set (default: false)
/// - `CROP_X, CROP_Y, CROP_WIDTH, CROP_HEIGHT`: Optional cropping parameters
/// 
/// # Output Format:
/// - For ASCII mode: `symbol,color,texture `
/// - For PETSCII mode: `symbol,foreground,texture,background `
/// 
/// The output begins with metadata: `width={width},height={height},texture=255`
/// followed by character data for each position in row-major order.
fn main() {
    let input_image_path;
    let mut width: u32 = 40;
    let mut height: u32 = 25;
    let mut is_petii: bool = false;
    
    // Character block dimensions (configurable, default 8x8 for compatibility)
    let block_width: u32 = 8;
    let block_height: u32 = 8;

    let args: Vec<String> = env::args().collect();

    // Check for help argument
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h" || args[1] == "help") {
        print_petii_usage();
        return;
    }

    if args.len() < 2 {
        eprintln!("Error: Missing required IMAGE_FILE argument");
        eprintln!();
        print_petii_usage();
        std::process::exit(1);
    }

    input_image_path = &args[1];
    if !Path::new(input_image_path).exists() {
        eprintln!("Error: Image file '{}' does not exist", input_image_path);
        std::process::exit(1);
    }

    if args.len() > 2 {
        width = args[2].parse().unwrap_or_else(|_| {
            eprintln!("Error: Invalid WIDTH value '{}'", args[2]);
            std::process::exit(1);
        });
    }
    if args.len() > 3 {
        height = args[3].parse().unwrap_or_else(|_| {
            eprintln!("Error: Invalid HEIGHT value '{}'", args[3]);
            std::process::exit(1);
        });
    }
    if args.len() > 4 {
        let petii_flag: u32 = args[4].parse().unwrap_or_else(|_| {
            eprintln!("Error: Invalid IS_PETSCII value '{}' (use 0 or 1)", args[4]);
            std::process::exit(1);
        });
        is_petii = petii_flag != 0;
    }

    // Load and optionally crop the image
    let mut img = image::open(input_image_path).expect("Failed to open image");

    // Handle optional cropping
    if args.len() > 8 {
        let cx: u32 = args[5].parse().expect("Invalid CROP_X");
        let cy: u32 = args[6].parse().expect("Invalid CROP_Y");
        let cw: u32 = args[7].parse().expect("Invalid CROP_WIDTH");
        let ch: u32 = args[8].parse().expect("Invalid CROP_HEIGHT");
        img = img.crop(cx, cy, cw, ch);
        img.save("tmp/out0.png").expect("save tmp/out0.png error");
    }

    let resized_img =
        img.resize_exact(width * block_width, height * block_height, image::imageops::FilterType::Lanczos3);
    resized_img
        .save("tmp/out1.png")
        .expect("save tmp/out1.png error");
    let gray_img = resized_img.clone().into_luma8();
    gray_img
        .save("tmp/out2.png")
        .expect("save tmp/out2.png error");

    // get petscii images...
    let vcs = gen_charset_images(false, block_width as usize, block_height as usize, &C64LOW, &C64UP);

    // find background color...
    let bret = find_background_color(&resized_img, &gray_img, width * block_width, height * block_height);
    let back_gray = bret.0;
    let back_rgb = bret.1;

    println!("width={},height={},texture=255", width, height);
    for i in 0..height {
        for j in 0..width {
            let block_at = get_grayscale_block_at(&gray_img, j, i, block_width, block_height);

            // Apply binarization for PETSCII mode before character matching
            let processed_block = if is_petii {
                binarize_grayscale_block(&block_at, back_gray, block_width as usize, block_height as usize)
            } else {
                block_at
            };

            let bm = find_best_match(&processed_block, &vcs, block_width as usize, block_height as usize);

            if !is_petii {
                let block_color = get_block_color(&resized_img, j, i, block_width, block_height);
                let bc = find_best_color(block_color);
                print!("{},{},1 ", bm, bc,);
            } else {
                let bc = get_petii_block_color(&resized_img, &gray_img, j, i, back_rgb, block_width, block_height);
                // sym, fg, tex, bg
                print!("{},{},1,{} ", bm, bc.1, bc.0);
            }
        }
        println!("");
    }
}