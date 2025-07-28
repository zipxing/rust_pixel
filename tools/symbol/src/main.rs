//! # RustPixel Symbol Extractor
//!
//! A powerful tool for extracting symbols/characters from images and converting them 
//! into analyzable patterns. This tool is particularly useful for:
//! - Creating custom character sets and fonts
//! - Analyzing pixel art and sprite sheets
//! - Converting images to symbol-based representations
//! - Generating ANSI-compatible color mappings
//!
//! ## Core Algorithm
//!
//! The symbol extraction process involves several key steps:
//! 1. **Image Segmentation**: Divide the input image into uniform blocks (symbols)
//! 2. **Color Analysis**: Identify background and foreground colors using Delta E color distance
//! 3. **Pattern Recognition**: Convert each block into a binary pattern (0s and 1s)
//! 4. **Deduplication**: Group identical patterns together to find unique symbols
//! 5. **Color Mapping**: Map original colors to the closest ANSI color palette
//! 6. **Output Generation**: Create both symbol atlas and reconstructed images
//!
//! ## Features
//!
//! - **Delta E Color Distance**: Uses perceptually accurate color comparison
//! - **Automatic Background Detection**: Finds the most common color as background
//! - **ANSI Color Mapping**: Maps colors to standard 256-color ANSI palette
//! - **Flexible Processing**: Supports custom processing regions within images
//! - **Binary Pattern Generation**: Creates 1-bit representations of symbols
//! - **Dual Output**: Generates both symbol atlas and color-reconstructed images

use deltae::*;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use lab::Lab;
use rust_pixel::render::style::ANSI_COLOR_RGB;
use std::collections::HashMap;
use std::env;
use std::path::Path;

/// RGB color structure for internal color processing
/// 
/// This structure provides a convenient way to work with RGB values
/// throughout the symbol extraction process.
struct RGB {
    r: u8,  // Red component (0-255)
    g: u8,  // Green component (0-255)
    b: u8,  // Blue component (0-255)
}

/// Display comprehensive usage information for the symbol extractor
///
/// This function provides detailed documentation about command-line usage,
/// parameters, examples, and feature descriptions. It serves as the primary
/// help system for users.
fn print_symbol_usage() {
    eprintln!("RustPixel Symbol Extractor");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    symbol <IMAGE_FILE> <SYMSIZE> [START_X START_Y WIDTH HEIGHT] [NOISE_THRESHOLD]");
    eprintln!("    cargo pixel symbol <IMAGE_FILE> <SYMSIZE> [START_X START_Y WIDTH HEIGHT] [NOISE_THRESHOLD]");
    eprintln!("    cargo pixel sy <IMAGE_FILE> <SYMSIZE> [START_X START_Y WIDTH HEIGHT] [NOISE_THRESHOLD]");
    eprintln!();
    eprintln!("ARGS:");
    eprintln!("    <IMAGE_FILE>      Input image file path");
    eprintln!("    <SYMSIZE>         Symbol size in pixels (e.g., 8 for 8x8 symbols)");
    eprintln!("    [START_X]         Start X coordinate for processing area");
    eprintln!("    [START_Y]         Start Y coordinate for processing area");
    eprintln!("    [WIDTH]           Width of processing area");
    eprintln!("    [HEIGHT]          Height of processing area");
    eprintln!("    [NOISE_THRESHOLD] Noise detection sensitivity (default: 4.0)");
    eprintln!("                      Lower values = more sensitive to noise");
    eprintln!("                      Higher values = less sensitive to noise");
    eprintln!("                      Range: 1.0-10.0, recommended: 2.0-6.0");
    eprintln!();
    eprintln!("DESCRIPTION:");
    eprintln!("    Extracts symbols/characters from images for use in creating symbol fonts");
    eprintln!("    or character sets. Analyzes image blocks and generates unique symbol");
    eprintln!("    patterns with color mappings for optimal representation.");
    eprintln!("    Includes advanced noise reduction for compressed images.");
    eprintln!();
    eprintln!("PROCESSING:");
    eprintln!("    - Applies intelligent noise reduction to clean compression artifacts");
    eprintln!("    - Divides image into blocks of specified symbol size");
    eprintln!("    - Analyzes each block for unique patterns");
    eprintln!("    - Maps colors to ANSI color palette");
    eprintln!("    - Generates symbol map and color associations");
    eprintln!();
    eprintln!("OUTPUT:");
    eprintln!("    - Symbol patterns displayed in terminal");
    eprintln!("    - Color mappings for foreground/background");
    eprintln!("    - Statistics about unique symbols found");
    eprintln!("    - sout.png: Symbol atlas (black & white patterns)");
    eprintln!("    - bout.png: Reconstructed image with ANSI colors");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    symbol font.png 8                          # Extract 8x8 symbols from entire image");
    eprintln!("    symbol charset.png 16                      # Extract 16x16 symbols");
    eprintln!("    symbol image.png 8 0 0 128 64              # Extract from 128x64 area at (0,0)");
    eprintln!("    symbol tiles.png 16 32 32 256 256 2.5      # Extract with high noise sensitivity");
    eprintln!("    symbol compressed.jpg 8 0 0 64 64 6.0      # Extract with low noise sensitivity");
    eprintln!();
    eprintln!("NOISE REDUCTION:");
    eprintln!("    - Detects isolated pixels that differ from neighbors");
    eprintln!("    - Identifies statistically rare colors in each block");
    eprintln!("    - Uses median filtering to preserve edges");
    eprintln!("    - Applies bilateral smoothing for subtle artifacts");
    eprintln!("    - Particularly effective for JPEG compression artifacts");
    eprintln!();
    eprintln!("FEATURES:");
    eprintln!("    - Color similarity analysis using Delta E");
    eprintln!("    - ANSI color palette mapping");
    eprintln!("    - Binary pattern recognition");
    eprintln!("    - Selective area processing");
    eprintln!("    - Advanced noise detection and removal");
    eprintln!("    - Compression artifact cleanup");
    eprintln!();
    eprintln!("NOTE:");
    eprintln!("    When used via cargo-pixel, equivalent to: cargo pixel r symbol t -r <ARGS...>");
}

/// Main entry point for the symbol extraction tool
///
/// This function orchestrates the entire symbol extraction process:
/// 1. Parse command-line arguments and validate input
/// 2. Load and optionally crop the input image
/// 3. Analyze the image to find the most common background color
/// 4. Process each symbol block to extract patterns and colors
/// 5. Generate symbol maps and color associations
/// 6. Create output images (symbol atlas and reconstructed image)
///
/// The algorithm divides the image into uniform blocks of the specified size,
/// analyzes each block for color patterns, and creates binary representations
/// of unique symbols while mapping colors to the ANSI palette.
fn main() {
    // Input parameters parsed from command line
    let input_image_path;
    let symsize: u32;             // Size of each symbol block (e.g., 8 for 8x8)
    let mut width: u32;           // Number of symbol blocks horizontally
    let mut height: u32;          // Number of symbol blocks vertically  
    let start_x: u32;             // Starting X coordinate for processing region
    let start_y: u32;             // Starting Y coordinate for processing region
    
    // Core data structures for symbol extraction
    // Maps binary patterns to lists of block indices that use that pattern
    let mut symbol_map: HashMap<Vec<Vec<u8>>, Vec<u32>> = HashMap::new();
    // Maps block indices to their (background_color, foreground_color) pair
    let mut color_map: HashMap<u32, (usize, usize)> = HashMap::new();

    // Parse command line arguments with validation
    let args: Vec<String> = env::args().collect();
    
    // Handle help requests
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h" || args[1] == "help") {
        print_symbol_usage();
        return;
    }
    
    // Validate argument count: either 3 args (image + symsize) or 7+ args (+ crop region + optional noise threshold)
    let arglen = args.len();
    if arglen < 3 || (arglen > 3 && arglen < 7) || arglen > 8 {
        print_symbol_usage();
        return;
    }
    
    // Parse required arguments
    input_image_path = Path::new(&args[1]);
    symsize = args[2].parse().unwrap_or_else(|_| {
        eprintln!("Error: SYMSIZE must be a valid positive integer");
        std::process::exit(1);
    });

    // Load the input image
    let mut img = image::open(&input_image_path).expect("Failed to open the input image");
    
    // Calculate initial grid dimensions based on symbol size
    width = img.width() as u32 / symsize;
    height = img.height() as u32 / symsize;

    // Handle optional crop parameters for processing specific image regions
    let noise_threshold = if arglen >= 7 {
        start_x = args[3].parse().unwrap_or_else(|_| {
            eprintln!("Error: START_X must be a valid integer");
            std::process::exit(1);
        });
        start_y = args[4].parse().unwrap_or_else(|_| {
            eprintln!("Error: START_Y must be a valid integer");
            std::process::exit(1);
        });
        let crop_width = args[5].parse::<u32>().unwrap_or_else(|_| {
            eprintln!("Error: WIDTH must be a valid positive integer");
            std::process::exit(1);
        });
        let crop_height = args[6].parse::<u32>().unwrap_or_else(|_| {
            eprintln!("Error: HEIGHT must be a valid positive integer");
            std::process::exit(1);
        });
        
        // Parse optional noise threshold (8th argument)
        let threshold = if arglen == 8 {
            args[7].parse::<f32>().unwrap_or_else(|_| {
                eprintln!("Error: NOISE_THRESHOLD must be a valid float (e.g., 4.0)");
                std::process::exit(1);
            })
        } else {
            4.0 // Default noise threshold
        };
        
        // Recalculate grid dimensions for cropped region
        width = crop_width / symsize;
        height = crop_height / symsize;
        
        // Crop the image to the specified region
        img = img.crop(start_x, start_y, width * symsize, height * symsize);
        threshold
    } else {
        4.0 // Default noise threshold for full image processing
    };
    
    println!("Processing {}x{} symbol grid ({}x{} pixels each)", width, height, symsize, symsize);
    println!("Noise reduction threshold: {:.1} Delta E", noise_threshold);

    // Step 1: Analyze the image to find the most common background color
    // This color will be used as a reference for pattern recognition
    let back_color = find_background_color(&img, width * symsize, height * symsize);
    println!("Detected background color: 0x{:08x}", back_color);

    // Step 2: Process each symbol block in the grid
    // Extract binary patterns and color information for each block
    for i in 0..height {
        for j in 0..width {
            // Process individual symbol block at grid position (j, i) with noise reduction
            let (bg_color_idx, fg_color_idx, pattern) = process_block(&img, symsize as usize, j, i, back_color, noise_threshold);
            
            // Calculate linear block index for mapping
            let block_index = i * width + j;
            
            // Store color mapping for this block
            color_map.entry(block_index).or_insert((bg_color_idx, fg_color_idx));
            
            // Group blocks by their binary patterns to identify unique symbols
            symbol_map
                .entry(pattern)
                .or_insert(Vec::new())
                .push(block_index);
        }
    }
    
    // Calculate symbol atlas layout (16 symbols per row)
    let unique_symbol_count = symbol_map.len();
    let atlas_width = 16;
    let atlas_height = unique_symbol_count / 16 + if unique_symbol_count % 16 == 0 { 0 } else { 1 };
    
    println!("Found {} unique symbols, creating {}x{} atlas", unique_symbol_count, atlas_width, atlas_height);

    // Step 3: Generate output images
    // Create symbol atlas showing all unique patterns in black and white
    let mut symbol_atlas = ImageBuffer::new(symsize * atlas_width as u32, symsize * atlas_height as u32);
    // Create reconstructed image using ANSI colors
    let mut reconstructed_img = ImageBuffer::new(symsize * width, symsize * height);
    
    let mut symbol_counter = 0;
    
    // Process each unique symbol pattern
    for (pattern, block_indices) in symbol_map.iter() {
        // Step 3a: Draw symbol in atlas (black and white representation)
        for y in 0..symsize {
            for x in 0..symsize {
                // Convert binary pattern to black/white pixels
                let pixel_value = if pattern[y as usize][x as usize] == 1 {
                    [255u8, 255, 255, 255]  // White for foreground
                } else {
                    [0u8, 0, 0, 255]        // Black for background
                };
                
                // Calculate position in symbol atlas
                let atlas_x = (symbol_counter % 16) * symsize + x;
                let atlas_y = (symbol_counter / 16) * symsize + y;
                symbol_atlas.put_pixel(atlas_x, atlas_y, Rgba(pixel_value));
            }
        }
        symbol_counter += 1;

        // Step 3b: Draw all instances of this symbol in the reconstructed image
        for &block_index in block_indices {
            // Calculate grid position from linear index
            let grid_x = block_index % width;
            let grid_y = block_index / width;
            
            // Get color mapping for this block
            let (bg_color_idx, fg_color_idx) = color_map.get(&block_index).unwrap();
            
            // Draw symbol using ANSI colors
            for y in 0..symsize {
                for x in 0..symsize {
                    // Choose color based on pattern bit
                    let color_idx = if pattern[y as usize][x as usize] == 1 {
                        *fg_color_idx  // Use foreground color for '1' bits
                    } else {
                        *bg_color_idx  // Use background color for '0' bits
                    };
                    
                    // Get RGB values from ANSI color palette
                    let ansi_color = ANSI_COLOR_RGB[color_idx];
                    let pixel_value = [ansi_color[0], ansi_color[1], ansi_color[2], 255];
                    
                    // Calculate final pixel position in reconstructed image
                    let final_x = grid_x * symsize + x;
                    let final_y = grid_y * symsize + y;
                    reconstructed_img.put_pixel(final_x, final_y, Rgba(pixel_value));
                }
            }
        }
    }
    
    // Step 4: Save output images
    println!("Saving symbol atlas to sout.png ({} symbols, {} rows, {} cols)", 
             unique_symbol_count, atlas_height, atlas_width);
    symbol_atlas.save("sout.png").expect("Failed to save symbol atlas");
    
    println!("Saving reconstructed image to bout.png");
    reconstructed_img.save("bout.png").expect("Failed to save reconstructed image");
    
    println!("Symbol extraction completed successfully!");
}

/// Find the most common color in the image to use as background
///
/// This function analyzes all pixels in the specified region to determine
/// the most frequently occurring color, which is assumed to be the background.
/// This background color is crucial for proper pattern recognition and
/// binary conversion of symbol blocks.
///
/// # Arguments
/// * `img` - The source image to analyze
/// * `w` - Width of the region to analyze (in pixels)
/// * `h` - Height of the region to analyze (in pixels)
///
/// # Returns
/// The most common color encoded as a 32-bit integer (RGBA format)
///
/// # Algorithm
/// 1. Scan every pixel in the specified region
/// 2. Count occurrence of each unique color
/// 3. Sort by frequency and return the most common
fn find_background_color(img: &DynamicImage, w: u32, h: u32) -> u32 {
    // Map to store color frequencies: color_u32 -> (first_x, first_y, count)
    let mut color_counts: HashMap<u32, (u32, u32, u32)> = HashMap::new();
    
    // Scan every pixel in the specified region
    for y in 0..h {
        for x in 0..w {
            let pixel = img.get_pixel(x, y);
            
            // Pack RGBA values into a single 32-bit integer for efficient hashing
            let color_key: u32 = ((pixel[0] as u32) << 24)  // Red
                                + ((pixel[1] as u32) << 16)  // Green  
                                + ((pixel[2] as u32) << 8)   // Blue
                                + (pixel[3] as u32);         // Alpha
            
            // Update color count, storing first occurrence position
            (*color_counts.entry(color_key).or_insert((x, y, 0))).2 += 1;
        }
    }
    
    // Convert to vector and sort by frequency (descending)
    let mut color_frequency: Vec<_> = color_counts.iter().collect();
    color_frequency.sort_by(|a, b| (&b.1 .2).cmp(&a.1 .2));  // Sort by count (descending)
    
    // Return the most frequent color
    *color_frequency[0].0
}

/// Calculate luminance of a color using standard coefficients
///
/// Luminance is used to determine which color should be considered
/// foreground vs background when only intensity differences matter.
///
/// # Arguments
/// * `color` - 32-bit RGBA color value
///
/// # Returns
/// Luminance value as a float (0.0 = black, 255.0 = white)
fn luminance(color: u32) -> f32 {
    let r = (color >> 24 & 0xff) as u8;
    let g = (color >> 16 & 0xff) as u8;
    let b = (color >> 8 & 0xff) as u8;
    
    // Standard luminance formula (ITU-R BT.601)
    0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32
}

/// Calculate perceptually accurate color distance using Delta E (CIE Lab)
///
/// Delta E provides a more accurate measure of color difference than
/// simple Euclidean distance in RGB space, as it accounts for human
/// color perception characteristics.
///
/// # Arguments
/// * `color1` - First color as 32-bit RGBA
/// * `color2` - Second color as 32-bit RGBA
///
/// # Returns
/// Delta E distance (0.0 = identical, higher = more different)
///
/// # Color Distance Thresholds
/// - < 1.0: Barely perceptible difference
/// - 1.0-2.0: Perceptible through close observation
/// - 2.0-10.0: Perceptible at a glance
/// - > 10.0: Very obvious difference
fn color_distance(color1: u32, color2: u32) -> f32 {
    // Extract RGB components from packed format
    let r1 = (color1 >> 24 & 0xff) as u8;
    let g1 = (color1 >> 16 & 0xff) as u8;
    let b1 = (color1 >> 8 & 0xff) as u8;
    let r2 = (color2 >> 24 & 0xff) as u8;
    let g2 = (color2 >> 16 & 0xff) as u8;
    let b2 = (color2 >> 8 & 0xff) as u8;

    // Convert RGB to LAB color space for perceptually accurate comparison
    let lab1 = Lab::from_rgb(&[r1, g1, b1]);
    let lab2 = Lab::from_rgb(&[r2, g2, b2]);
    
    // Create LAB value structures for Delta E calculation
    let lab_val1 = LabValue { l: lab1.l, a: lab1.a, b: lab1.b };
    let lab_val2 = LabValue { l: lab2.l, a: lab2.a, b: lab2.b };
    
    // Calculate Delta E 2000 (most accurate color difference formula)
    *DeltaE::new(&lab_val1, &lab_val2, DE2000).value()
}

/// Process a single symbol block to extract pattern and color information
///
/// This is the core function that analyzes each symbol block to:
/// 1. Apply noise reduction to clean compression artifacts
/// 2. Identify all unique colors within the cleaned block
/// 3. Determine optimal foreground and background colors
/// 4. Convert the block to a binary pattern
/// 5. Map colors to the closest ANSI palette entries
///
/// # Arguments
/// * `image` - Source image to process
/// * `block_size` - Size of the symbol block (e.g., 8 for 8x8)
/// * `grid_x` - X position in the symbol grid
/// * `grid_y` - Y position in the symbol grid  
/// * `background_color` - The detected background color for reference
/// * `noise_threshold` - Delta E threshold for noise detection (4.0 = moderate, lower = more sensitive)
///
/// # Returns
/// Tuple containing:
/// - Background color index in ANSI palette
/// - Foreground color index in ANSI palette  
/// - Binary pattern as 2D vector (0s and 1s)
///
/// # Algorithm
/// 1. Apply denoising to reduce compression artifacts
/// 2. Collect all colors within the cleaned block
/// 3. Determine if background color is present
/// 4. Select optimal foreground/background pair
/// 5. Convert each pixel to binary based on color similarity
/// 6. Map final colors to ANSI palette
fn process_block(
    image: &DynamicImage,
    block_size: usize,
    grid_x: u32,
    grid_y: u32,
    background_color: u32,
    noise_threshold: f32,
) -> (usize, usize, Vec<Vec<u8>>) {
    
    // Step 1: Apply denoising to reduce compression artifacts and noise
    let denoised_pixel_colors = denoise_block(image, block_size, grid_x, grid_y, noise_threshold);
    
    // Map to store unique colors and their first occurrence position
    let mut unique_colors: HashMap<u32, (u32, u32)> = HashMap::new();
    // Initialize binary pattern matrix
    let mut binary_pattern = vec![vec![0u8; block_size]; block_size];
    
    // Step 2: Analyze the denoised colors to build color map
    for (idx, &color_key) in denoised_pixel_colors.iter().enumerate() {
        let row = idx / block_size;
        let col = idx % block_size;
        let pixel_x = grid_x * block_size as u32 + col as u32;
        let pixel_y = grid_y * block_size as u32 + row as u32;
        
        // Record unique colors and their positions
        unique_colors.entry(color_key).or_insert((pixel_x, pixel_y));
    }
    
    // Step 3: Analyze color composition and determine optimal color pair
    let mut colors_vec: Vec<_> = unique_colors.iter().collect();
    let mut background_present = false;
    let unique_color_count = colors_vec.len();
    
    // Check if any colors are similar to the detected background
    for color_entry in &mut colors_vec {
        if *color_entry.0 == background_color {
            background_present = true;
        } else {
            let distance = color_distance(*color_entry.0, background_color);
            // Merge colors that are very similar to background (Delta E < 1.5, slightly more tolerant after denoising)
            if distance < 1.5 {
                (*color_entry).0 = &background_color;
                background_present = true;
            }
        }
    }
    
    // Step 4: Determine optimal foreground/background color pair based on color composition
    let optimal_colors = if background_present {
        match unique_color_count {
            1 => {
                // Single color block (likely all background)
                Some((background_color, background_color))
            }
            2 => {
                // Two colors: background + one foreground
                let mut result = (background_color, background_color);
                for color_entry in &colors_vec {
                    if *color_entry.0 != background_color {
                        result.1 = *color_entry.0;
                        break;
                    }
                }
                Some(result)
            }
            _ => {
                // Multiple colors: keep background, find most contrasting foreground
                let mut max_distance = 0.0f32;
                let mut best_foreground = colors_vec[0];
                
                for color_entry in &colors_vec {
                    let distance = color_distance(*color_entry.0, background_color);
                    if distance > max_distance {
                        max_distance = distance;
                        best_foreground = *color_entry;
                    }
                }
                Some((background_color, *best_foreground.0))
            }
        }
    } else {
        // No background color present
        match unique_color_count {
            1 => {
                // Single foreground color
                Some((*colors_vec[0].0, *colors_vec[0].0))
            }
            2 => {
                // Two colors: assign based on luminance (darker = background)
                let lum1 = luminance(*colors_vec[0].0);
                let lum2 = luminance(*colors_vec[1].0);
                
                if lum2 > lum1 {
                    Some((*colors_vec[0].0, *colors_vec[1].0))  // Dark bg, light fg
                } else {
                    Some((*colors_vec[1].0, *colors_vec[0].0))  // Light bg, dark fg
                }
            }
            _ => {
                // Multiple colors: merge similar ones and pick two most contrasting
                let mut distinct_colors = vec![];
                colors_vec.sort();
                
                let mut base_color = *colors_vec[0].0;
                distinct_colors.push(colors_vec[0]);
                
                // Find distinct colors (Delta E > 1.5, slightly more tolerant after denoising)
                for i in 1..unique_color_count {
                    let distance = color_distance(*colors_vec[i].0, base_color);
                    if distance > 1.5 {
                        distinct_colors.push(colors_vec[i]);
                    }
                    base_color = *colors_vec[i].0;
                }
                
                // Use luminance to determine foreground/background
                if distinct_colors.len() >= 2 {
                    let lum1 = luminance(*distinct_colors[0].0);
                    let lum2 = luminance(*distinct_colors[1].0);
                    
                    if lum2 > lum1 {
                        Some((*distinct_colors[0].0, *distinct_colors[1].0))
                    } else {
                        Some((*distinct_colors[1].0, *distinct_colors[0].0))
                    }
                } else {
                    Some((*distinct_colors[0].0, *distinct_colors[0].0))
                }
            }
        }
    };

    // Step 5: Generate binary pattern based on color similarity using denoised colors
    if let Some((bg_color, fg_color)) = optimal_colors {
        for row in 0..block_size {
            for col in 0..block_size {
                let pixel_idx = row * block_size + col;
                let pixel_color = denoised_pixel_colors[pixel_idx];
                
                // Calculate distances to both background and foreground
                let distance_to_bg = color_distance(pixel_color, bg_color);
                let distance_to_fg = color_distance(pixel_color, fg_color);
                
                // Assign bit based on which color is closer
                binary_pattern[row][col] = if distance_to_bg <= distance_to_fg { 0 } else { 1 };
            }
        }
        
        // Step 6: Map colors to ANSI palette and return result
        (
            find_best_ansi_color_from_u32(bg_color),
            find_best_ansi_color_from_u32(fg_color),
            binary_pattern
        )
    } else {
        // Fallback case (should rarely occur)
        (0, 0, binary_pattern)
    }
}

/// Convert a 32-bit color to RGB struct for ANSI color matching
///
/// # Arguments
/// * `color` - 32-bit RGBA color value
///
/// # Returns
/// RGB struct with extracted color components
fn find_best_ansi_color_from_u32(color: u32) -> usize {
    find_best_ansi_color(RGB {
        r: (color >> 24) as u8,
        g: (color >> 16) as u8,
        b: (color >> 8) as u8,
    })
}

/// Calculate color distance between two RGB colors using Delta E
///
/// # Arguments  
/// * `color1` - First RGB color
/// * `color2` - Second RGB color
///
/// # Returns
/// Delta E color distance value
fn color_distance_rgb(color1: &RGB, color2: &RGB) -> f32 {
    // Convert both colors to LAB space
    let lab1 = Lab::from_rgb(&[color1.r, color1.g, color1.b]);
    let lab2 = Lab::from_rgb(&[color2.r, color2.g, color2.b]);
    
    // Create LAB value structures
    let lab_val1 = LabValue { l: lab1.l, a: lab1.a, b: lab1.b };
    let lab_val2 = LabValue { l: lab2.l, a: lab2.a, b: lab2.b };
    
    // Calculate Delta E 2000
    *DeltaE::new(&lab_val1, &lab_val2, DE2000).value()
}

/// Find the closest ANSI color match for a given RGB color
///
/// This function searches through the standard ANSI 256-color palette
/// to find the perceptually closest match using Delta E color distance.
///
/// # Arguments
/// * `target_color` - RGB color to match
///
/// # Returns
/// Index of the closest ANSI color (0-255)
///
/// # Algorithm
/// 1. Compare input color against all 256 ANSI colors
/// 2. Use Delta E for perceptually accurate distance measurement
/// 3. Return index of color with minimum distance
fn find_best_ansi_color(target_color: RGB) -> usize {
    let mut min_distance = f32::MAX;
    let mut best_match_index = 0;

    // Search through all ANSI colors for the best match
    for (index, ansi_color) in ANSI_COLOR_RGB.iter().enumerate() {
        let ansi_rgb = RGB {
            r: ansi_color[0],
            g: ansi_color[1], 
            b: ansi_color[2],
        };
        
        // Calculate perceptual color distance
        let distance = color_distance_rgb(&ansi_rgb, &target_color);

        // Track the closest match
        if distance < min_distance {
            min_distance = distance;
            best_match_index = index;
        }
    }

    best_match_index
}

/// Detect and remove noise pixels from an image block
///
/// This function identifies noise pixels that are likely artifacts from image compression
/// or other sources, and replaces them with more appropriate colors based on their
/// local neighborhood context.
///
/// # Arguments
/// * `image` - Source image to analyze
/// * `block_size` - Size of the symbol block
/// * `grid_x` - X position in the symbol grid
/// * `grid_y` - Y position in the symbol grid
/// * `noise_threshold` - Delta E threshold for noise detection (lower = more sensitive)
///
/// # Returns
/// Vector of cleaned pixel colors
///
/// # Noise Detection Strategy
/// 1. **Isolation Detection**: Pixels that differ significantly from all neighbors
/// 2. **Statistical Outliers**: Colors that appear very rarely in the block
/// 3. **Compression Artifacts**: Subtle color variations near edges
///
/// # Denoising Approach
/// - Use neighborhood voting to determine correct pixel colors
/// - Apply median filtering for edge preservation
/// - Replace isolated pixels with dominant neighbor colors
fn denoise_block(
    image: &DynamicImage,
    block_size: usize,
    grid_x: u32,
    grid_y: u32,
    noise_threshold: f32,
) -> Vec<u32> {
    let mut pixel_colors = vec![0u32; block_size * block_size];
    let mut denoised_colors = vec![0u32; block_size * block_size];
    
    // Step 1: Extract all pixel colors from the block
    for row in 0..block_size {
        for col in 0..block_size {
            let pixel_x = grid_x * block_size as u32 + col as u32;
            let pixel_y = grid_y * block_size as u32 + row as u32;
            
            if pixel_x < image.width() && pixel_y < image.height() {
                let pixel = image.get_pixel(pixel_x, pixel_y);
                let color = ((pixel[0] as u32) << 24)
                          + ((pixel[1] as u32) << 16)
                          + ((pixel[2] as u32) << 8)
                          + (pixel[3] as u32);
                pixel_colors[row * block_size + col] = color;
            }
        }
    }
    
    // Step 2: Identify noise pixels using neighborhood analysis
    for row in 0..block_size {
        for col in 0..block_size {
            let current_idx = row * block_size + col;
            let current_color = pixel_colors[current_idx];
            
            // Collect neighbor colors (3x3 neighborhood)
            let mut neighbor_colors = Vec::new();
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nr = row as i32 + dy;
                    let nc = col as i32 + dx;
                    
                    // Skip out-of-bounds and center pixel
                    if nr < 0 || nc < 0 || nr >= block_size as i32 || nc >= block_size as i32 || (dy == 0 && dx == 0) {
                        continue;
                    }
                    
                    let neighbor_idx = nr as usize * block_size + nc as usize;
                    neighbor_colors.push(pixel_colors[neighbor_idx]);
                }
            }
            
            // Step 3: Check if current pixel is noise
            let is_noise = if neighbor_colors.is_empty() {
                false // Edge pixels are not considered noise
            } else {
                // Calculate average distance to neighbors
                let total_distance: f32 = neighbor_colors.iter()
                    .map(|&neighbor| color_distance(current_color, neighbor))
                    .sum();
                let avg_distance = total_distance / neighbor_colors.len() as f32;
                
                // Also check if this color is statistically rare in the block
                let color_frequency = pixel_colors.iter()
                    .filter(|&&c| color_distance(c, current_color) < 2.0)
                    .count();
                let frequency_ratio = color_frequency as f32 / (block_size * block_size) as f32;
                
                // Consider it noise if it's isolated AND rare
                avg_distance > noise_threshold && frequency_ratio < 0.1
            };
            
            // Step 4: Replace noise pixels with corrected colors
            if is_noise {
                // Use median color from neighbors for replacement
                let corrected_color = if !neighbor_colors.is_empty() {
                    find_median_color(&neighbor_colors)
                } else {
                    current_color // Keep original if no neighbors
                };
                denoised_colors[current_idx] = corrected_color;
            } else {
                denoised_colors[current_idx] = current_color;
            }
        }
    }
    
    // Step 5: Apply additional smoothing for remaining artifacts
    apply_smoothing_filter(&mut denoised_colors, block_size)
}

/// Find the median color from a list of colors
///
/// This function helps reduce noise by finding a representative color
/// from a neighborhood, which is more robust than simple averaging.
///
/// # Arguments
/// * `colors` - List of colors to analyze
///
/// # Returns
/// Median color that best represents the input colors
fn find_median_color(colors: &[u32]) -> u32 {
    if colors.is_empty() {
        return 0;
    }
    
    if colors.len() == 1 {
        return colors[0];
    }
    
    // Extract RGB components for median calculation
    let mut reds: Vec<u8> = colors.iter().map(|&c| (c >> 24) as u8).collect();
    let mut greens: Vec<u8> = colors.iter().map(|&c| (c >> 16) as u8).collect();
    let mut blues: Vec<u8> = colors.iter().map(|&c| (c >> 8) as u8).collect();
    let mut alphas: Vec<u8> = colors.iter().map(|&c| c as u8).collect();
    
    // Sort each component
    reds.sort();
    greens.sort();
    blues.sort();
    alphas.sort();
    
    // Find median values
    let mid = colors.len() / 2;
    let median_r = if colors.len() % 2 == 0 {
        ((reds[mid - 1] as u16 + reds[mid] as u16) / 2) as u8
    } else {
        reds[mid]
    };
    
    let median_g = if colors.len() % 2 == 0 {
        ((greens[mid - 1] as u16 + greens[mid] as u16) / 2) as u8
    } else {
        greens[mid]
    };
    
    let median_b = if colors.len() % 2 == 0 {
        ((blues[mid - 1] as u16 + blues[mid] as u16) / 2) as u8
    } else {
        blues[mid]
    };
    
    let median_a = if colors.len() % 2 == 0 {
        ((alphas[mid - 1] as u16 + alphas[mid] as u16) / 2) as u8
    } else {
        alphas[mid]
    };
    
    // Reconstruct median color
    ((median_r as u32) << 24) + ((median_g as u32) << 16) + ((median_b as u32) << 8) + (median_a as u32)
}

/// Apply additional smoothing to reduce remaining compression artifacts
///
/// This function performs a gentle smoothing operation that preserves edges
/// while reducing subtle noise that might remain after the initial denoising.
///
/// # Arguments
/// * `colors` - Mutable reference to the color array
/// * `block_size` - Size of the block (assuming square)
///
/// # Returns
/// Modified color array with smoothing applied
fn apply_smoothing_filter(colors: &mut [u32], block_size: usize) -> Vec<u32> {
    let mut smoothed = colors.to_vec();
    
    // Apply a gentle bilateral-like filter
    for row in 1..(block_size - 1) {
        for col in 1..(block_size - 1) {
            let center_idx = row * block_size + col;
            let center_color = colors[center_idx];
            
            // Collect nearby colors with weights
            let mut weighted_colors = Vec::new();
            let mut total_weight = 0.0f32;
            
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nr = row as i32 + dy;
                    let nc = col as i32 + dx;
                    let neighbor_idx = nr as usize * block_size + nc as usize;
                    let neighbor_color = colors[neighbor_idx];
                    
                    // Calculate spatial and color distances
                    let spatial_distance = ((dy * dy + dx * dx) as f32).sqrt();
                    let color_distance_val = color_distance(center_color, neighbor_color);
                    
                    // Bilateral filter weight: close in space AND similar in color get higher weight
                    let spatial_weight = (-spatial_distance * spatial_distance / 2.0).exp();
                    let color_weight = (-color_distance_val * color_distance_val / 8.0).exp();
                    let weight = spatial_weight * color_weight;
                    
                    weighted_colors.push((neighbor_color, weight));
                    total_weight += weight;
                }
            }
            
            // Calculate weighted average color
            if total_weight > 0.0 {
                let mut sum_r = 0.0f32;
                let mut sum_g = 0.0f32;
                let mut sum_b = 0.0f32;
                let mut sum_a = 0.0f32;
                
                for (color, weight) in weighted_colors {
                    let w = weight / total_weight;
                    sum_r += ((color >> 24) as u8) as f32 * w;
                    sum_g += ((color >> 16) as u8) as f32 * w;
                    sum_b += ((color >> 8) as u8) as f32 * w;
                    sum_a += (color as u8) as f32 * w;
                }
                
                let smoothed_color = ((sum_r as u8 as u32) << 24)
                                   + ((sum_g as u8 as u32) << 16)
                                   + ((sum_b as u8 as u32) << 8)
                                   + (sum_a as u8 as u32);
                
                // Only apply smoothing if the change is subtle (preserve edges)
                let change_amount = color_distance(center_color, smoothed_color);
                if change_amount < 3.0 {
                    smoothed[center_idx] = smoothed_color;
                }
            }
        }
    }
    
    smoothed
}
