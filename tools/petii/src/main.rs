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

/// Type alias for an 8x8 grayscale image block represented as a 2D vector of u8 values
/// Each value represents a grayscale pixel intensity from 0 (black) to 255 (white)
type Image8x8 = Vec<Vec<u8>>;

/// RGB color structure representing a 24-bit color value
/// Each component (red, green, blue) ranges from 0 to 255
#[derive(Debug, Clone, Copy)]
struct RGB {
    /// Red color component (0-255)
    r: u8,
    /// Green color component (0-255)
    g: u8,
    /// Blue color component (0-255)
    b: u8,
}

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

/// Calculates perceptual color distance between two RGB colors using Delta E 2000 algorithm
/// 
/// This function provides more accurate color difference measurement than simple Euclidean
/// distance by converting RGB to LAB color space and applying the CIE Delta E 2000 formula,
/// which accounts for human perception of color differences.
/// 
/// # Arguments:
/// * `e1` - First RGB color for comparison
/// * `e2` - Second RGB color for comparison
/// 
/// # Returns:
/// * `f32` - Delta E distance value where:
///   - 0.0 = identical colors
///   - 1.0 = just noticeable difference
///   - 2.3 = noticeable difference  
///   - 5.0 = clear difference
///   - Higher values = increasingly different colors
/// 
/// # Usage:
/// Used by `find_best_color()` to match input colors to the closest ANSI terminal colors
/// by finding the minimum Delta E distance across the palette.
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

/// Generates 8x8 pixel representations of 256 PETSCII/C64 characters for pattern matching
/// 
/// This function creates binary images (using only 0 and 255 values) for each character
/// in the C64 character set by interpreting the bitmap data from the character ROM.
/// The generated images are used as templates for finding the best character match
/// for each 8x8 block in the input image.
/// 
/// # Character Set Organization:
/// - Characters 0-127: Normal character set with foreground pixels as 255, background as 0
/// - Characters 128-255: Inverted versions with foreground as 0, background as 255
/// - Total: 256 character variants for comprehensive matching
/// 
/// # Arguments:
/// * `low_up` - Character set variant selector:
///   - `true`: Uses lowercase/uppercase character set (C64LOW)
///   - `false`: Uses uppercase/graphics character set (C64UP) - **typically used**
/// 
/// # Returns:
/// * `Vec<Image8x8>` - Vector of 256 8x8 binary character images
/// 
/// # Implementation Details:
/// Each character is defined by 8 bytes representing 8 rows of 8 bits each.
/// Bits are processed right-to-left to match C64 character orientation.
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

/// Analyzes an 8x8 pixel block to determine optimal foreground and background colors for PETSCII mode
/// 
/// PETSCII characters support only two colors per character cell (foreground and background).
/// This function analyzes the color distribution within a block to determine the best
/// two-color representation, considering the global background color detected in the image.
/// 
/// # Color Analysis Strategy:
/// 1. **Color enumeration**: Collects all unique RGB colors in the 8x8 block
/// 2. **Background consideration**: Checks if the global background color appears in the block
/// 3. **Optimal pairing**: Determines the best foreground/background color combination:
///    - If background color present: uses it as background, most contrasting color as foreground
///    - If no background: uses darker color as background, lighter as foreground
///    - Handles edge cases with single colors or too many colors gracefully
/// 
/// # Arguments:
/// * `image` - Source color image for RGB analysis
/// * `img` - Corresponding grayscale image for brightness comparison
/// * `x` - Block X coordinate in character grid
/// * `y` - Block Y coordinate in character grid  
/// * `back_rgb` - Global background color (packed as u32: RGBA)
/// 
/// # Returns:
/// * `(usize, usize)` - Tuple of (background_color_index, foreground_color_index)
///   where indices refer to positions in the ANSI color palette
/// 
/// # Error Handling:
/// Prints "ERROR!!!" or "ERROR2!!!" for invalid color configurations and returns (0,0)
fn get_petii_block_color(
    image: &DynamicImage,
    img: &ImageBuffer<Luma<u8>, Vec<u8>>,
    x: u32,
    y: u32,
    back_rgb: u32,
) -> (usize, usize) {
    let mut cc: HashMap<u32, (u32, u32)> = HashMap::new();
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
                cc.entry(k).or_insert((pixel_x, pixel_y));
            }
        }
    }
    let cv: Vec<_> = cc.iter().collect();
    let mut include_back = false;
    let clen = cv.len();
    for c in &cv {
        if *c.0 == back_rgb {
            include_back = true;
        }
    }
    let mut ret = None;
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
            println!("ERROR!!!");
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
            println!("ERROR2!!!");
        }
    }
    match ret {
        Some(r) => (find_best_color_u32(r.0), find_best_color_u32(r.1)),
        _ => (0, 0),
    }
}

/// Calculates the average RGB color of an 8x8 pixel block for ASCII mode color matching
/// 
/// This function computes the mean color across all non-black pixels in the specified
/// character block. Pure black pixels (0,0,0) are excluded from the average to prevent
/// background pixels from skewing the color calculation toward darkness.
/// 
/// # Color Averaging Process:
/// 1. **Pixel iteration**: Scans all 64 pixels in the 8x8 block
/// 2. **Black pixel filtering**: Skips pixels with RGB values of (0,0,0)
/// 3. **Component accumulation**: Sums red, green, and blue values separately
/// 4. **Average calculation**: Divides totals by pixel count for mean color
/// 
/// # Arguments:
/// * `image` - Source color image to analyze
/// * `x` - Block X coordinate in character grid (multiply by 8 for pixel coordinates)
/// * `y` - Block Y coordinate in character grid (multiply by 8 for pixel coordinates)
/// 
/// # Returns:
/// * `RGB` - Average color of the block, or black RGB{0,0,0} if no valid pixels found
/// 
/// # Use Case:
/// In ASCII mode, each character position gets a single color determined by this average.
/// The resulting color is then matched to the closest ANSI terminal color for display.
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

/// Extracts an 8x8 grayscale pixel block from the source image at the specified character position
/// 
/// This function creates a local copy of image data for a single character cell,
/// converting the image coordinates from character space to pixel space.
/// The extracted block is used for character pattern matching and analysis.
/// 
/// # Coordinate System:
/// - Input coordinates (x,y) represent character positions in the output grid
/// - Pixel coordinates are calculated as (x*8, y*8) to (x*8+7, y*8+7)
/// - Handles boundary cases gracefully when blocks extend beyond image edges
/// 
/// # Arguments:
/// * `image` - Source grayscale image (single channel, u8 values)
/// * `x` - Character X position in the output grid
/// * `y` - Character Y position in the output grid
/// 
/// # Returns:
/// * `Image8x8` - 8x8 matrix of grayscale values (0-255)
///   - `block[row][column]` format for easy iteration
///   - Out-of-bounds pixels default to 0 (black)
/// 
/// # Memory Layout:
/// The returned block uses [row][column] indexing where:
/// - First index (0-7) represents vertical position (Y)
/// - Second index (0-7) represents horizontal position (X)
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

/// Finds the character that best matches the input 8x8 grayscale block using feature comparison
/// 
/// This function compares the input block against all available character templates
/// using a sophisticated feature-based matching algorithm rather than simple pixel comparison.
/// The matching process uses eigenvector analysis to capture structural patterns
/// and mean squared error for quantitative comparison.
/// 
/// # Matching Algorithm:
/// 1. **Feature extraction**: Computes eigenvector features for input block
/// 2. **Template comparison**: Calculates features for each character template
/// 3. **Distance calculation**: Uses MSE between feature vectors (not raw pixels)
/// 4. **Best match selection**: Returns character index with minimum MSE distance
/// 
/// # Why Feature-Based Matching:
/// - More robust than pixel-by-pixel comparison
/// - Captures structural patterns and shapes
/// - Less sensitive to minor brightness variations
/// - Better handles aliasing and scaling artifacts
/// 
/// # Arguments:
/// * `input_image` - 8x8 grayscale block to match (from source image)
/// * `char_images` - Array of 256 character template images (from `gen_charset_images`)
/// 
/// # Returns:
/// * `usize` - Index (0-255) of the best matching character in the character set
/// 
/// # Performance:
/// O(n) where n=256 characters, with feature comparison being much faster than full pixel MSE
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

/// Converts a packed u32 color value to RGB and finds the closest ANSI color match
/// 
/// This utility function serves as a wrapper for `find_best_color()` that handles
/// the common case of colors stored in packed 32-bit format (typically RGBA).
/// 
/// # Color Format:
/// The u32 input is expected to be packed as RGBA:
/// - Bits 31-24: Red component (most significant byte)
/// - Bits 23-16: Green component  
/// - Bits 15-8: Blue component
/// - Bits 7-0: Alpha component (ignored)
/// 
/// # Arguments:
/// * `c` - Packed 32-bit color value in RGBA format
/// 
/// # Returns:
/// * `usize` - Index of the closest matching color in the ANSI color palette (0-255)
/// 
/// # Usage:
/// Primarily used by `get_petii_block_color()` to convert the color analysis results
/// into terminal-compatible color indices for PETSCII rendering.
fn find_best_color_u32(c: u32) -> usize {
    find_best_color(RGB {
        r: (c >> 24) as u8,
        g: (c >> 16) as u8,
        b: (c >> 8) as u8,
    })
}

/// Finds the closest matching ANSI terminal color for a given RGB color using perceptual distance
/// 
/// This function performs color quantization by mapping arbitrary RGB colors to the
/// limited ANSI color palette available in terminal environments. It uses the Delta E 2000
/// algorithm for perceptually accurate color matching rather than simple Euclidean distance.
/// 
/// # Color Matching Process:
/// 1. **Palette iteration**: Compares input color against all ANSI colors (0-255)
/// 2. **Perceptual distance**: Uses `color_distance()` with Delta E 2000 for accuracy
/// 3. **Best match selection**: Returns the palette index with minimum perceptual difference
/// 
/// # Why Delta E 2000:
/// - Accounts for human color perception non-linearities
/// - More accurate than RGB Euclidean distance
/// - Better handles color differences in different hue regions
/// - Industry standard for color matching applications
/// 
/// # Arguments:
/// * `color` - RGB color to match against the ANSI palette
/// 
/// # Returns:
/// * `usize` - Index (0-255) in the ANSI color palette (`ANSI_COLOR_RGB` array)
/// 
/// # Performance:
/// O(n) where n=256 ANSI colors, with Delta E calculation being the primary cost
/// 
/// # Usage:
/// Used by both ASCII and PETSCII modes to convert arbitrary image colors to
/// terminal-displayable color indices.
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

/// Calculates a 10-dimensional feature vector that captures structural patterns in an 8x8 image block
/// 
/// This function extracts geometric and structural features from grayscale image blocks
/// to enable robust character matching. Instead of comparing raw pixel values, it computes
/// higher-level features that are more invariant to brightness variations and minor distortions.
/// 
/// # Feature Components (10-dimensional vector):
/// - **v[0]**: Top-left quadrant sum (pixels 0-3, 0-3)
/// - **v[1]**: Top-right quadrant sum (pixels 4-7, 0-3)  
/// - **v[2]**: Bottom-left quadrant sum (pixels 0-3, 4-7)
/// - **v[3]**: Bottom-right quadrant sum (pixels 4-7, 4-7)
/// - **v[4]**: Central region sum (pixels 2-5, 2-5) - captures core pattern
/// - **v[5]**: Main diagonals sum (x==y or x==(7-y)) - detects diagonal patterns
/// - **v[6]**: Left edge sum (x==0) - vertical line detection
/// - **v[7]**: Right edge sum (x==7) - vertical line detection  
/// - **v[8]**: Top edge sum (y==0) - horizontal line detection
/// - **v[9]**: Bottom edge sum (y==7) - horizontal line detection
/// 
/// # Mathematical Properties:
/// - **Translation invariant**: Features remain stable under small shifts
/// - **Brightness adaptive**: Relative patterns matter more than absolute values
/// - **Structurally descriptive**: Captures key geometric properties of characters
/// 
/// # Arguments:
/// * `img` - 8x8 grayscale image block to analyze
/// 
/// # Returns:
/// * `Vec<i32>` - 10-element feature vector representing structural characteristics
/// 
/// # Usage:
/// Used by `calculate_mse()` to compare feature vectors rather than raw pixel data,
/// providing more robust character matching for ASCII/PETSCII conversion.
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

/// Calculates the mean squared error between two 8x8 image blocks using feature vectors
/// 
/// This function computes a distance metric between image blocks by comparing their
/// structural features rather than raw pixel values. This approach provides more robust
/// and perceptually meaningful matching for character recognition applications.
/// 
/// # Algorithm Steps:
/// 1. **Feature extraction**: Computes 10-dimensional eigenvectors for both images
/// 2. **Component differences**: Calculates squared differences for each feature component
/// 3. **Distance calculation**: Sums squared differences and takes square root (Euclidean norm)
/// 
/// # Why Feature-Based MSE:
/// - **Structural focus**: Emphasizes shape and pattern over exact pixel intensity
/// - **Noise robustness**: Less sensitive to compression artifacts and minor variations
/// - **Efficiency**: 10 comparisons instead of 64 pixel comparisons
/// - **Perceptual relevance**: Features correlate better with human character recognition
/// 
/// # Mathematical Formula:
/// ```
/// MSE = sqrt(Σ(feature1[i] - feature2[i])²) for i = 0 to 9
/// ```
/// 
/// # Arguments:
/// * `img1` - First 8x8 image block (typically from source image)
/// * `img2` - Second 8x8 image block (typically character template)
/// 
/// # Returns:
/// * `f64` - Distance metric where:
///   - 0.0 = identical structural features
///   - Lower values = better matches
///   - Higher values = more dissimilar patterns
/// 
/// # Usage:
/// Core function used by `find_best_match()` to rank character template similarity
/// for optimal ASCII/PETSCII character selection.
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
