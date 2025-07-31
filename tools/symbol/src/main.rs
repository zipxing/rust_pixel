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
//! The extractor uses an advanced multi-stage processing pipeline:
//! 1. **Contrast-Aware Thresholding**: Intelligently chooses between Otsu's algorithm and 
//!    uniform classification based on contrast analysis (10% brightness difference threshold)
//! 2. **Noise Filtering**: Low-contrast blocks are treated as uniform regions to prevent
//!    noise amplification and false pattern detection
//! 3. **Color Analysis**: Calculates optimal foreground/background colors per block
//! 4. **Pattern Recognition**: Extracts binary patterns from symbol blocks
//! 5. **Similarity Clustering**: Groups similar symbols using Hamming distance
//! 6. **ANSI Mapping**: Maps colors to standard 256-color ANSI palette
//!
//! ## Features
//!
//! - **Adaptive Binarization**: Automatic threshold detection using Otsu's method
//! - **Similarity Clustering**: Reduces redundant symbols using Hamming distance
//! - **ANSI Color Mapping**: Maps colors to standard 256-color ANSI palette
//! - **Flexible Processing**: Supports custom processing regions within images
//! - **Binary Pattern Generation**: Creates 1-bit representations of symbols
//! - **Dual Output**: Generates both symbol atlas and color-reconstructed images

use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb, RgbImage};
use rust_pixel::render::symbols::{
    binarize_block, BinarizationConfig, BinarizedBlock, RGB
};
use std::collections::HashMap;
use std::env;
use std::path::Path;

/// Character cell structure representing a variable-sized symbol
///
/// Each cell contains a binary pattern and associated colors.
/// The bitmap uses 0 for background pixels and 1 for foreground pixels.
#[derive(Debug, Clone)]
struct CharacterCell {
    /// Variable-sized binary pattern (0 = background, 1 = foreground)
    bitmap: Vec<Vec<u8>>,
    /// Symbol size (width and height)
    size: usize,
    /// Average foreground color
    foreground_color: Rgb<u8>,
    /// Average background color  
    background_color: Rgb<u8>,
}

impl From<BinarizedBlock> for CharacterCell {
    fn from(block: BinarizedBlock) -> Self {
        // For compatibility with existing code that expects square symbols,
        // ensure the block is square or use the minimum dimension
        let size = if block.width == block.height {
            block.width
        } else {
            // For non-square blocks, use the minimum dimension and warn
            eprintln!("Warning: Converting non-square BinarizedBlock ({}x{}) to square CharacterCell, using size {}", 
                block.width, block.height, block.width.min(block.height));
            block.width.min(block.height)
        };
        
        Self {
            bitmap: block.bitmap,
            size,
            foreground_color: Rgb([block.foreground_color.r, block.foreground_color.g, block.foreground_color.b]),
            background_color: Rgb([block.background_color.r, block.background_color.g, block.background_color.b]),
        }
    }
}

/// Cell information for image reconstruction
///
/// Stores the bitmap index and position-specific colors
#[derive(Debug, Clone)]
struct CellInfo {
    /// Index into the unique character array
    bitmap_index: usize,
    /// Foreground color for this specific position
    foreground_color: Rgb<u8>,
    /// Background color for this specific position
    background_color: Rgb<u8>,
}

/// Print usage information and examples for the symbol extraction tool
///
/// This function displays comprehensive help including command-line syntax,
/// parameter descriptions, and practical usage examples.
fn print_symbol_usage() {
    eprintln!("RustPixel Symbol Extractor");
    eprintln!("==========================");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    cargo pixel sy <IMAGE_FILE> <SYMBOL_SIZE> [NOISE_THRESHOLD] [X Y WIDTH HEIGHT]");
    eprintln!();
    eprintln!("ARGUMENTS:");
    eprintln!("    IMAGE_FILE       Path to input image file (PNG, JPG, etc.)");
    eprintln!(
        "    SYMBOL_SIZE      Size of each symbol block (e.g., 8 for 8x8 pixels, 16 for 16x16)"
    );
    eprintln!("    NOISE_THRESHOLD  Optional: Hamming distance threshold for similarity clustering (default: 2)");
    eprintln!("                     Lower values = more strict clustering, higher = more aggressive merging");
    eprintln!("    X Y WIDTH HEIGHT Optional: Crop region coordinates and dimensions");
    eprintln!();
    eprintln!("ALGORITHM:");
    eprintln!("    1. Adaptive Thresholding: Uses Otsu's algorithm for optimal binarization");
    eprintln!("    2. Color Analysis: Calculates foreground/background colors per block");
    eprintln!("    3. Pattern Extraction: Generates binary patterns from symbol blocks");
    eprintln!("    4. Similarity Clustering: Groups similar symbols using Hamming distance");
    eprintln!("    5. ANSI Color Mapping: Maps colors to 256-color ANSI palette");
    eprintln!();
    eprintln!("OUTPUT:");
    eprintln!("    - Symbol patterns displayed in terminal");
    eprintln!("    - Color mappings for foreground/background");
    eprintln!("    - Statistics about unique symbols found");
    eprintln!("    - sout.png: Symbol atlas (black & white patterns)");
    eprintln!("    - bout.png: Reconstructed image with ANSI colors");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    cargo pixel sy image.png 8");
    eprintln!("    cargo pixel sy sprite.png 16 1");
    eprintln!("    cargo pixel sy logo.png 8 3 10 10 100 50");
    eprintln!();
    eprintln!("NOISE_THRESHOLD VALUES:");
    eprintln!("    0: Perfect match only (no clustering)");
    eprintln!("    1-2: Very strict clustering (recommended for pixel art)");
    eprintln!("    3-5: Moderate clustering (good for compressed images)");
    eprintln!("    6+: Aggressive clustering (may lose detail)");
}

/// Process a single variable-sized cell using the library's adaptive thresholding
///
/// This is now a wrapper around the rust_pixel library's binarize_block function
///
/// # Arguments
/// * `cell` - Variable-sized array of RGB pixels representing the symbol block
/// * `_size` - Size of the cell (width and height in pixels) - unused, size is inferred from cell
/// * `min_contrast_ratio` - Minimum contrast ratio (0.0-1.0) for triggering Otsu's algorithm
///
/// # Returns
/// CharacterCell with optimized binary pattern and extracted colors
fn process_cell(cell: &[Vec<Rgb<u8>>], _size: usize, min_contrast_ratio: f32) -> CharacterCell {
    // Convert image RGB to symbols RGB format
    let symbols_pixels: Vec<Vec<RGB>> = cell
        .iter()
        .map(|row| {
            row.iter()
                .map(|&pixel| RGB {
                    r: pixel[0],
                    g: pixel[1],
                    b: pixel[2],
                })
                .collect()
        })
        .collect();

    let config = BinarizationConfig {
        min_contrast_ratio,
    };

    // Use the library's binarization function
    let binarized_block = binarize_block(&symbols_pixels, &config);
    
    // Convert back to CharacterCell format
    CharacterCell::from(binarized_block)
}

// Otsu's algorithm is now provided by the rust_pixel library
// The find_optimal_threshold function has been moved to src/render/symbols.rs

/// Convert variable-sized bitmap to vector for distance calculation
///
/// # Arguments
/// * `bitmap` - Variable-sized binary pattern
///
/// # Returns
/// Flattened vector representation
fn bitmap_to_vector(bitmap: &[Vec<u8>]) -> Vec<u8> {
    let mut vector = Vec::new();
    for row in bitmap {
        for &pixel in row {
            vector.push(pixel);
        }
    }
    vector
}

/// Calculate Hamming distance between two binary vectors
///
/// Hamming distance counts the number of positions where the vectors differ.
/// This is ideal for comparing binary patterns.
///
/// # Arguments
/// * `v1` - First binary vector
/// * `v2` - Second binary vector
///
/// # Returns
/// Number of differing positions
fn hamming_distance(v1: &[u8], v2: &[u8]) -> u32 {
    v1.iter()
        .zip(v2.iter())
        .map(|(a, b)| if a != b { 1 } else { 0 })
        .sum()
}

/// Cluster similar characters using Hamming distance threshold
///
/// This function groups similar binary patterns together to reduce redundancy.
/// Characters within the similarity threshold are merged into clusters.
///
/// # Arguments
/// * `chars` - Vector of character cells to cluster
/// * `similarity_threshold` - Maximum Hamming distance for clustering
///
/// # Returns
/// Tuple of (clustered characters, bitmap-to-index mapping)
fn cluster_similar_characters(
    chars: Vec<CharacterCell>,
    similarity_threshold: u32,
) -> (Vec<CharacterCell>, HashMap<Vec<Vec<u8>>, usize>) {
    if chars.is_empty() {
        return (chars, HashMap::new());
    }

    let mut clustered = Vec::new();
    let mut used = vec![false; chars.len()];
    let mut bitmap_to_new_index = HashMap::new();

    for i in 0..chars.len() {
        if used[i] {
            continue;
        }

        let char_vector = bitmap_to_vector(&chars[i].bitmap);
        let mut cluster_chars = vec![chars[i].clone()];
        used[i] = true;

        // Find similar characters
        for j in (i + 1)..chars.len() {
            if used[j] {
                continue;
            }

            let other_vector = bitmap_to_vector(&chars[j].bitmap);
            let distance = hamming_distance(&char_vector, &other_vector);

            if distance <= similarity_threshold {
                cluster_chars.push(chars[j].clone());
                used[j] = true;
            }
        }

        // Choose cluster representative (first character)
        let representative = cluster_chars[0].clone();
        let new_index = clustered.len();

        // Map all bitmaps in this cluster to the new index
        for cluster_char in &cluster_chars {
            bitmap_to_new_index.insert(cluster_char.bitmap.clone(), new_index);
        }

        clustered.push(representative);

        if cluster_chars.len() > 1 {
            println!(
                "Merged {} similar characters (hamming distance <= {})",
                cluster_chars.len(),
                similarity_threshold
            );
        }
    }

    (clustered, bitmap_to_new_index)
}

/// Extract unique character cells from image using adaptive processing
///
/// This is the main processing function that:
/// 1. Divides the image into symbol-sized blocks
/// 2. Processes each block with adaptive thresholding
/// 3. Extracts unique patterns and colors
///
/// # Arguments
/// * `img` - Source image to process
/// * `symsize` - Size of each symbol block
/// * `crop_region` - Optional crop region (x, y, width, height)
///
/// # Returns
/// Tuple of (unique character cells, position mapping)
fn extract_unique_cells(
    img: &DynamicImage,
    symsize: u32,
    crop_region: Option<(u32, u32, u32, u32)>,
) -> (Vec<CharacterCell>, Vec<Vec<CellInfo>>) {
    let (img_width, img_height, start_x, start_y) = if let Some((x, y, w, h)) = crop_region {
        (w, h, x, y)
    } else {
        (img.width(), img.height(), 0, 0)
    };

    let grid_w = img_width / symsize;
    let grid_h = img_height / symsize;
    let mut bitmap_to_index: HashMap<Vec<Vec<u8>>, usize> = HashMap::new();
    let mut unique_cells = vec![];
    let mut cell_map = vec![
        vec![
            CellInfo {
                bitmap_index: 0,
                foreground_color: Rgb([0, 0, 0]),
                background_color: Rgb([0, 0, 0])
            };
            grid_w as usize
        ];
        grid_h as usize
    ];

    // Process each symbol block
    for gy in 0..grid_h {
        for gx in 0..grid_w {
            let mut cell = vec![vec![Rgb([0, 0, 0]); symsize as usize]; symsize as usize];

            // Extract pixel block manually
            for dy in 0..symsize as usize {
                for dx in 0..symsize as usize {
                    let x = start_x + gx * symsize + dx as u32;
                    let y = start_y + gy * symsize + dy as u32;

                    if x < img.width() && y < img.height() {
                        let pixel = img.get_pixel(x, y);
                        cell[dy][dx] = Rgb([pixel[0], pixel[1], pixel[2]]);
                    }
                }
            }

            // Process the cell with adaptive thresholding
            // min_contrast_ratio = 0.1 means minimum 10% brightness difference (25/255) is required
            // to trigger Otsu's algorithm. This value balances noise filtering with pattern preservation:
            // - Lower values (< 0.1): Too sensitive, treats noise/JPEG artifacts as patterns
            // - Higher values (> 0.1): Too conservative, may miss low-contrast real symbols
            // 0.1 is an empirically proven threshold that works well across different image types
            let char_cell = process_cell(&cell, symsize as usize, 0.1);

            // Check for existing bitmap pattern
            let bitmap_index = if let Some(&existing_index) = bitmap_to_index.get(&char_cell.bitmap)
            {
                existing_index
            } else {
                let new_index = unique_cells.len();
                bitmap_to_index.insert(char_cell.bitmap.clone(), new_index);
                unique_cells.push(char_cell.clone());
                new_index
            };

            cell_map[gy as usize][gx as usize] = CellInfo {
                bitmap_index,
                foreground_color: char_cell.foreground_color,
                background_color: char_cell.background_color,
            };
        }
    }

    (unique_cells, cell_map)
}

///// Find the closest ANSI color match for a given RGB color
/////
///// This function searches through the standard ANSI 256-color palette
///// to find the best perceptual match using Euclidean distance in RGB space.
/////
///// # Arguments
///// * `target` - RGB color to match
/////
///// # Returns
///// Index of the closest ANSI color (0-255)
//fn find_best_ansi_color(target: &Rgb<u8>) -> usize {
//    let mut min_distance = f32::MAX;
//    let mut best_match_index = 0;

//    for (index, ansi_color) in ANSI_COLOR_RGB.iter().enumerate() {
//        // Calculate Euclidean distance in RGB space
//        let dr = target[0] as f32 - ansi_color[0] as f32;
//        let dg = target[1] as f32 - ansi_color[1] as f32;
//        let db = target[2] as f32 - ansi_color[2] as f32;
//        let distance = (dr * dr + dg * dg + db * db).sqrt();

//        if distance < min_distance {
//            min_distance = distance;
//            best_match_index = index;
//        }
//    }

//    best_match_index
//}

/// Render character set as a black and white atlas image
///
/// Creates a grid layout showing all unique symbols as binary patterns.
///
/// # Arguments
/// * `chars` - Vector of unique character cells
/// * `chars_per_row` - Number of characters per row in output
///
/// # Returns
/// RGB image containing the character atlas
fn render_character_set(chars: &Vec<CharacterCell>, chars_per_row: usize) -> RgbImage {
    if chars.is_empty() {
        return ImageBuffer::new(1, 1);
    }

    let symbol_size = chars[0].size;
    let rows = (chars.len() + chars_per_row - 1) / chars_per_row;
    let w = chars_per_row * symbol_size;
    let h = rows * symbol_size;
    let mut out = ImageBuffer::new(w as u32, h as u32);

    for (i, char_cell) in chars.iter().enumerate() {
        let row = i / chars_per_row;
        let col = i % chars_per_row;
        let start_x = col * symbol_size;
        let start_y = row * symbol_size;

        for y in 0..symbol_size {
            for x in 0..symbol_size {
                let color = match char_cell.bitmap[y][x] {
                    0 => Rgb([255, 255, 255]), // White background
                    1 => Rgb([0, 0, 0]),       // Black foreground
                    _ => Rgb([128, 128, 128]), // Should not occur
                };
                out.put_pixel((start_x + x) as u32, (start_y + y) as u32, color);
            }
        }
    }

    out
}

/// Reconstruct the original image using extracted symbols and colors
///
/// Creates a faithful reconstruction using the extracted patterns and
/// position-specific colors.
///
/// # Arguments
/// * `unique_cells` - Vector of unique character patterns
/// * `cell_map` - Mapping of positions to characters and colors
/// * `width` - Output image width
/// * `height` - Output image height
/// * `symsize` - Symbol size
///
/// # Returns
/// RGB image reconstruction
fn reconstruct_image(
    unique_cells: &Vec<CharacterCell>,
    cell_map: &Vec<Vec<CellInfo>>,
    width: u32,
    height: u32,
    symsize: u32,
) -> RgbImage {
    let mut out = ImageBuffer::new(width, height);
    let grid_w = width / symsize;
    let grid_h = height / symsize;

    for gy in 0..grid_h {
        for gx in 0..grid_w {
            let cell_info = &cell_map[gy as usize][gx as usize];
            let char_cell = &unique_cells[cell_info.bitmap_index];

            for dy in 0..symsize as usize {
                for dx in 0..symsize as usize {
                    let x = gx * symsize + dx as u32;
                    let y = gy * symsize + dy as u32;

                    if x < width && y < height {
                        let color = match char_cell.bitmap[dy][dx] {
                            0 => cell_info.background_color,
                            1 => cell_info.foreground_color,
                            _ => Rgb([128, 128, 128]),
                        };
                        out.put_pixel(x, y, color);
                    }
                }
            }
        }
    }

    out
}

///// Print symbol pattern in terminal using ASCII representation
/////
///// Displays the binary pattern using '█' for foreground and ' ' for background.
/////
///// # Arguments
///// * `bitmap` - Variable-sized binary pattern to display
///// * `index` - Symbol index for labeling
//fn print_symbol_pattern(bitmap: &[Vec<u8>], index: usize) {
//    println!("Symbol {}:", index);
//    for row in bitmap {
//        print!("  ");
//        for &pixel in row {
//            if pixel == 1 {
//                print!("█");
//            } else {
//                print!(" ");
//            }
//        }
//        println!();
//    }
//}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_symbol_usage();
        return;
    }

    // Parse command line arguments
    let image_path = &args[1];
    let symsize: u32 = args[2].parse().unwrap_or_else(|_| {
        eprintln!("Error: Symbol size must be a valid number");
        std::process::exit(1);
    });

    let similarity_threshold: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(2); // Default to 2 for moderate clustering

    // Parse optional crop region
    let crop_region = if args.len() >= 8 {
        let x: u32 = args[4].parse().unwrap_or(0);
        let y: u32 = args[5].parse().unwrap_or(0);
        let w: u32 = args[6].parse().unwrap_or(0);
        let h: u32 = args[7].parse().unwrap_or(0);
        Some((x, y, w, h))
    } else {
        None
    };

    // Validate file path
    if !Path::new(image_path).exists() {
        eprintln!("Error: Image file '{}' not found", image_path);
        std::process::exit(1);
    }

    // Load and process image
    println!("Loading image: {}", image_path);
    let img = image::open(image_path).unwrap_or_else(|e| {
        eprintln!("Error: Failed to load image: {}", e);
        std::process::exit(1);
    });

    println!("Processing image with adaptive thresholding...");
    let (original_unique_cells, mut cell_map) = extract_unique_cells(&img, symsize, crop_region);

    println!(
        "Found {} unique character cells (before similarity clustering)",
        original_unique_cells.len()
    );

    // Apply similarity clustering
    println!(
        "Clustering similar characters (threshold: {} pixels)...",
        similarity_threshold
    );
    let (unique_cells, bitmap_to_new_index) =
        cluster_similar_characters(original_unique_cells.clone(), similarity_threshold);

    println!(
        "After similarity clustering: {} unique character cells",
        unique_cells.len()
    );

    // Update cell mapping with clustered indices
    for row in &mut cell_map {
        for cell_info in row {
            let old_char = &original_unique_cells[cell_info.bitmap_index];
            if let Some(&new_index) = bitmap_to_new_index.get(&old_char.bitmap) {
                cell_info.bitmap_index = new_index;
            }
        }
    }

    // Display symbols and color information
    // println!("\nExtracted Symbols:");
    // println!("==================");

    // for (i, char_cell) in unique_cells.iter().enumerate() {
    //     print_symbol_pattern(&char_cell.bitmap, i);

    //     let fg_ansi = find_best_ansi_color(&char_cell.foreground_color);
    //     let bg_ansi = find_best_ansi_color(&char_cell.background_color);

    //     println!(
    //         "  Colors: FG=RGB({},{},{})→ANSI[{}], BG=RGB({},{},{})→ANSI[{}]",
    //         char_cell.foreground_color[0], char_cell.foreground_color[1], char_cell.foreground_color[2], fg_ansi,
    //         char_cell.background_color[0], char_cell.background_color[1], char_cell.background_color[2], bg_ansi
    //     );
    //     println!();
    // }

    // Generate output images
    let image_width = if let Some((_, _, w, _)) = crop_region {
        w
    } else {
        img.width()
    };
    let image_height = if let Some((_, _, _, h)) = crop_region {
        h
    } else {
        img.height()
    };

    // Save character set (black & white)
    let char_set = render_character_set(&unique_cells, 16);
    char_set
        .save("sout.png")
        .expect("Failed to save character set");
    println!("Saved character set to sout.png (16 chars per row, B&W)");

    // Save reconstructed image (with colors)
    let reconstructed =
        reconstruct_image(&unique_cells, &cell_map, image_width, image_height, symsize);
    reconstructed
        .save("bout.png")
        .expect("Failed to save reconstructed image");
    println!("Saved reconstructed image to bout.png (with colors)");

    // Print summary statistics
    println!("\nProcessing Summary:");
    println!("===================");
    println!("Image size: {}x{}", image_width, image_height);
    println!("Symbol size: {}x{}", symsize, symsize);
    println!(
        "Grid dimensions: {}x{}",
        image_width / symsize,
        image_height / symsize
    );
    println!("Original unique symbols: {}", original_unique_cells.len());
    println!("After clustering: {}", unique_cells.len());
    println!(
        "Compression ratio: {:.1}%",
        (1.0 - unique_cells.len() as f32 / original_unique_cells.len() as f32) * 100.0
    );
    println!("Similarity threshold: {} pixels", similarity_threshold);
}
